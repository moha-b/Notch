use super::{count_window, home, Failure};
use crate::usage::LimitWindow;
use chrono::{Datelike, Local, TimeZone};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

#[derive(Default)]
struct Totals {
    today: i64,
    month: i64,
    calls: i64,
}

impl Totals {
    fn add(&mut self, timestamp: i64, tokens: i64, calls: i64) {
        let Some(at) = Local.timestamp_millis_opt(timestamp).single() else {
            return;
        };
        let now = Local::now();
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

pub fn read() -> Result<Vec<LimitWindow>, Failure> {
    let home = home()?;
    let sources = [
        ("Gemini CLI", cli(&home.join(".gemini/tmp"))?),
        (
            "OpenCode",
            opencode(&home.join(".local/share/opencode/opencode.db"))?,
        ),
        ("Hermes", hermes(&home.join(".hermes/state.db"))?),
    ];
    let mut windows = Vec::new();
    for (name, totals) in sources {
        if let Some(totals) = totals {
            windows.push(count_window(
                &format!("{name} · tokens this month"),
                totals.month,
            ));
            windows.push(count_window(
                &format!("{name} · tokens today"),
                totals.today,
            ));
            windows.push(count_window(
                &format!("{name} · calls this month"),
                totals.calls,
            ));
        }
    }
    if windows.is_empty() {
        return Err(Failure::Unavailable(
            "No Gemini usage logs found. API keys are never read.".into(),
        ));
    }
    Ok(windows)
}

fn cli(root: &Path) -> Result<Option<Totals>, Failure> {
    if !root.exists() {
        return Ok(None);
    }
    let mut totals = Totals::default();
    let projects = std::fs::read_dir(root)
        .map_err(|_| Failure::Unavailable("Cannot read Gemini logs.".into()))?;
    for project in projects {
        let project =
            project.map_err(|_| Failure::Unavailable("Cannot read Gemini project.".into()))?;
        let chats = project.path().join("chats");
        if !chats.is_dir() {
            continue;
        }
        for session in std::fs::read_dir(chats)
            .map_err(|_| Failure::Unavailable("Cannot read Gemini chats.".into()))?
        {
            let session =
                session.map_err(|_| Failure::Unavailable("Cannot read Gemini session.".into()))?;
            if session
                .path()
                .extension()
                .is_none_or(|extension| extension != "jsonl")
            {
                continue;
            }
            let text = std::fs::read_to_string(session.path())
                .map_err(|_| Failure::Unavailable("Cannot read Gemini session log.".into()))?;
            add_cli_records(&mut totals, &text);
        }
    }
    Ok(Some(totals))
}

fn add_cli_records(totals: &mut Totals, text: &str) {
    let mut completed = BTreeMap::new();
    for line in text.lines() {
        // A live append-only log may end with a partial record; the next poll reads it again.
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if record["type"] != "gemini" || !record["tokens"].is_object() {
            continue;
        }
        if let Some(id) = record["id"].as_str() {
            completed.insert(id.to_owned(), record);
        }
    }
    for record in completed.values() {
        if let Some(timestamp) = super::reset(&record["timestamp"]) {
            totals.add(
                timestamp as i64,
                record["tokens"]["total"].as_i64().unwrap_or(0),
                1,
            );
        }
    }
}

fn database_totals(path: &Path, sql: &str) -> Result<Option<Totals>, Failure> {
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
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| Failure::Invalid("Cannot query Gemini usage."))?;
    let mut totals = Totals::default();
    for row in rows {
        let (at, tokens, calls) = row.map_err(|_| Failure::Invalid("Invalid Gemini usage row."))?;
        totals.add(at, tokens, calls);
    }
    Ok(Some(totals))
}

fn opencode(path: &Path) -> Result<Option<Totals>, Failure> {
    database_totals(path, "SELECT time_created,
        CASE WHEN json_extract(data, '$.tokens.total') > 0 THEN json_extract(data, '$.tokens.total') ELSE
        coalesce(json_extract(data, '$.tokens.input'),0) + coalesce(json_extract(data, '$.tokens.output'),0) +
        coalesce(json_extract(data, '$.tokens.reasoning'),0) + coalesce(json_extract(data, '$.tokens.cache.read'),0) +
        coalesce(json_extract(data, '$.tokens.cache.write'),0) END, 1
        FROM message WHERE json_extract(data, '$.role') = 'assistant' AND json_extract(data, '$.providerID') = 'google'")
}

fn hermes(path: &Path) -> Result<Option<Totals>, Failure> {
    // Hermes includes reasoning in output_tokens; adding it again would double-count.
    database_totals(
        path,
        "SELECT cast(last_seen * 1000 AS INTEGER),
        input_tokens + cache_read_tokens + cache_write_tokens + output_tokens, api_call_count
        FROM session_model_usage WHERE billing_provider = 'gemini'",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repeated_completed_records_count_once_and_aborted_turns_do_not_count() {
        let stamp = Local::now().to_rfc3339();
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
        let mut totals = Totals::default();
        add_cli_records(&mut totals, &log);
        assert_eq!((totals.today, totals.month, totals.calls), (20, 20, 1));
    }
}
