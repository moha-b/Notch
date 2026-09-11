use super::{home, Failure};
use crate::usage::LimitWindow;
use chrono::{DateTime, Datelike, Local, TimeZone};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::BufRead,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod database_tests;
mod summary;

struct Totals {
    now: DateTime<Local>,
    today: i64,
    month: i64,
    calls: i64,
}

impl Totals {
    fn new(now: &DateTime<Local>) -> Self {
        Self {
            now: *now,
            today: 0,
            month: 0,
            calls: 0,
        }
    }
    fn add(&mut self, timestamp: i64, tokens: i64, calls: i64) {
        let Some(at) = Local.timestamp_millis_opt(timestamp).single() else {
            return;
        };
        let now = &self.now;
        if tokens <= 0 || (at.year(), at.month()) != (now.year(), now.month()) {
            return;
        }
        self.month = self.month.saturating_add(tokens);
        self.calls = self.calls.saturating_add(calls.max(0));
        if at.day() == now.day() {
            self.today = self.today.saturating_add(tokens);
        }
    }
}

pub fn read(budget: Option<u64>) -> Result<Vec<LimitWindow>, Failure> {
    let home = home()?;
    let now = Local::now();
    let readings = [
        ("cli", "Gemini CLI", cli(&home.join(".gemini/tmp"), &now)?),
        (
            "opencode",
            "OpenCode",
            opencode(
                &super::credentials::opencode_directory()?.join("opencode.db"),
                &now,
            )?,
        ),
        (
            "hermes",
            "Hermes",
            hermes(&home.join(".hermes/state.db"), &now)?,
        ),
    ];
    // A missing tool is not a zero reading, so it gets no row at all.
    let sources: Vec<_> = readings
        .into_iter()
        .filter_map(|(id, name, totals)| totals.map(|totals| summary::Source { id, name, totals }))
        .collect();
    if sources.is_empty() {
        return Err(Failure::Unavailable(
            "No Gemini usage logs found. API keys are never read.".into(),
        ));
    }
    summary::windows(&sources, budget, &now)
}

fn cli(root: &Path, now: &DateTime<Local>) -> Result<Option<Totals>, Failure> {
    if !root.exists() {
        return Ok(None);
    }
    let (month_start, _) = summary::month_bounds(now)?;
    let mut totals = Totals::new(now);
    for log in current_session_logs(root, month_start)? {
        let file = std::fs::File::open(log)
            .map_err(|_| Failure::Unavailable("Cannot read Gemini session log.".into()))?;
        read_cli_records(&mut totals, std::io::BufReader::new(file))?;
    }
    Ok(Some(totals))
}

/// Session logs are append-only, so a file last written before this month holds none of its records.
fn current_session_logs(root: &Path, month_start: i64) -> Result<Vec<PathBuf>, Failure> {
    let mut logs = Vec::new();
    for project in directory_entries(root, "Cannot read Gemini logs.")? {
        let chats = project.path().join("chats");
        if !chats.is_dir() {
            continue;
        }
        for session in directory_entries(&chats, "Cannot read Gemini chats.")? {
            let path = session.path();
            if path
                .extension()
                .is_none_or(|extension| extension != "jsonl")
            {
                continue;
            }
            let modified = session
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_err(|_| Failure::Unavailable("Cannot inspect Gemini session log.".into()))?;
            if DateTime::<chrono::Utc>::from(modified).timestamp_millis() >= month_start {
                logs.push(path);
            }
        }
    }
    Ok(logs)
}

fn directory_entries(path: &Path, failure: &str) -> Result<Vec<std::fs::DirEntry>, Failure> {
    std::fs::read_dir(path)
        .and_then(|entries| entries.collect())
        .map_err(|_| Failure::Unavailable(failure.into()))
}

fn read_cli_records(totals: &mut Totals, reader: impl BufRead) -> Result<(), Failure> {
    let mut completed = BTreeMap::new();
    for line in reader.lines() {
        let line =
            line.map_err(|_| Failure::Unavailable("Cannot read Gemini log record.".into()))?;
        // A live append-only log may end with a partial record; the next poll reads it again.
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if record["type"] != "gemini" || !record["tokens"].is_object() {
            continue;
        }
        if let Some(id) = record["id"].as_str() {
            completed.insert(
                id.to_owned(),
                (
                    super::reset(&record["timestamp"]),
                    record["tokens"]["total"].as_i64().unwrap_or(0),
                ),
            );
        }
    }
    for (timestamp, tokens) in completed.values() {
        if let Some(timestamp) = timestamp.and_then(|stamp| i64::try_from(stamp).ok()) {
            totals.add(timestamp, *tokens, 1);
        }
    }
    Ok(())
}

fn database_totals(
    path: &Path,
    sql: &str,
    now: &DateTime<Local>,
) -> Result<Option<Totals>, Failure> {
    if !path.exists() {
        return Ok(None);
    }
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| {
                Failure::Unavailable("Cannot open the Gemini usage database read-only.".into())
            })?;
    let mut query = connection
        .prepare(sql)
        .map_err(|_| Failure::Invalid("Gemini usage database schema is unsupported."))?;
    let (month_start, _) = summary::month_bounds(now)?;
    let rows = query
        .query_map([month_start], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| Failure::Invalid("Cannot query Gemini usage."))?;
    let mut totals = Totals::new(now);
    for row in rows {
        let (at, tokens, calls) = row.map_err(|_| Failure::Invalid("Invalid Gemini usage row."))?;
        totals.add(at, tokens, calls);
    }
    Ok(Some(totals))
}

fn opencode(path: &Path, now: &DateTime<Local>) -> Result<Option<Totals>, Failure> {
    database_totals(path, "SELECT time_created,
        CASE WHEN json_extract(data, '$.tokens.total') > 0 THEN json_extract(data, '$.tokens.total') ELSE
        coalesce(json_extract(data, '$.tokens.input'),0) + coalesce(json_extract(data, '$.tokens.output'),0) +
        coalesce(json_extract(data, '$.tokens.reasoning'),0) + coalesce(json_extract(data, '$.tokens.cache.read'),0) +
        coalesce(json_extract(data, '$.tokens.cache.write'),0) END, 1
        FROM message WHERE time_created >= ?1 AND json_extract(data, '$.role') = 'assistant' AND json_extract(data, '$.providerID') = 'google'", now)
}

fn hermes(path: &Path, now: &DateTime<Local>) -> Result<Option<Totals>, Failure> {
    // Hermes includes reasoning in output_tokens; adding it again would double-count.
    database_totals(
        path,
        "SELECT cast(last_seen * 1000 AS INTEGER),
        input_tokens + cache_read_tokens + cache_write_tokens + output_tokens, api_call_count
        FROM session_model_usage WHERE billing_provider = 'gemini' AND last_seen >= ?1 / 1000.0",
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repeated_completed_records_count_once_and_aborted_turns_do_not_count() {
        let now = Local::now();
        let stamp = now.to_rfc3339();
        let records = [
            json!({"id":"one","type":"gemini","timestamp":stamp}),
            json!({"id":"one","type":"gemini","timestamp":stamp,"tokens":{"total":10}}),
            json!({"id":"one","type":"gemini","timestamp":stamp,"tokens":{"total":20}}),
            json!({"id":"aborted","type":"gemini","timestamp":stamp,"tokens":{"total":0}}),
        ];
        let log = records
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let mut totals = Totals::new(&now);
        read_cli_records(&mut totals, std::io::Cursor::new(log)).unwrap();
        assert_eq!((totals.today, totals.month, totals.calls), (20, 20, 1));
    }
}
