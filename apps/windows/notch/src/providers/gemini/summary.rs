use super::{DateTime, Datelike, Failure, LimitWindow, Local, TimeZone, Totals};

/// One tool's contribution, kept separate so the card can say which log a number came from.
pub(super) struct Source {
    pub id: &'static str,
    pub name: &'static str,
    pub totals: Totals,
}

pub(super) fn month_bounds(now: &DateTime<Local>) -> Result<(i64, i64), Failure> {
    let start = now
        .date_naive()
        .with_day(1)
        .ok_or(Failure::Invalid("Invalid local month."))?;
    let end = start
        .checked_add_months(chrono::Months::new(1))
        .ok_or(Failure::Invalid("Invalid next month."))?;
    Ok((local_midnight(start)?, local_midnight(end)?))
}

fn local_midnight(date: chrono::NaiveDate) -> Result<i64, Failure> {
    // Some zones advance at midnight; the calendar day starts at its first valid local minute.
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(Failure::Invalid("Invalid calendar date."))?;
    (0..180)
        .find_map(|minute| {
            Local
                .from_local_datetime(&(midnight + chrono::Duration::minutes(minute)))
                .earliest()
        })
        .map(|stamp| stamp.timestamp_millis())
        .ok_or(Failure::Invalid("Local calendar boundary is unavailable."))
}

pub(super) fn windows(
    sources: &[Source],
    budget: Option<u64>,
    now: &DateTime<Local>,
) -> Result<Vec<LimitWindow>, Failure> {
    let month = sources.iter().fold(0i64, |total, source| {
        total.saturating_add(source.totals.month)
    });
    let today = sources.iter().fold(0i64, |total, source| {
        total.saturating_add(source.totals.today)
    });
    let mut windows = vec![monthly_window(month, budget, now)?];
    let tomorrow = now
        .date_naive()
        .succ_opt()
        .ok_or(Failure::Invalid("Invalid next day."))?;
    windows.push(LimitWindow {
        resets_at: Some(local_midnight(tomorrow)? as u64),
        ..count("today", "Tokens today", today)
    });
    windows.extend(sources.iter().map(|source| {
        count(
            source.id,
            &format!("{} · this month", source.name),
            source.totals.month,
        )
    }));
    Ok(windows)
}

fn monthly_window(
    tokens: i64,
    budget: Option<u64>,
    now: &DateTime<Local>,
) -> Result<LimitWindow, Failure> {
    let (start, end) = month_bounds(now)?;
    Ok(LimitWindow {
        id: "month".into(),
        label: budget
            .map(|budget| format!("Tokens this month · personal budget {}", compact(budget)))
            .unwrap_or_else(|| "Tokens this month · billed per token, no limit".into()),
        used: budget
            .map(|budget| tokens as f64 / budget as f64)
            .unwrap_or(0.0),
        count: if budget.is_none() { Some(tokens) } else { None },
        derived: true,
        resets_at: Some(end as u64),
        duration_seconds: Some((end - start) as f64 / 1000.0),
    })
}

/// Matches the Mac label format: exact below 10k, whole thousands below 1M, then one decimal.
fn compact(count: u64) -> String {
    match count {
        0..=9_999 => count.to_string(),
        10_000..=999_999 => format!("{}k", count / 1_000),
        _ => format!("{:.1}M", count as f64 / 1_000_000.0),
    }
}

fn count(id: &str, label: &str, tokens: i64) -> LimitWindow {
    LimitWindow {
        id: id.into(),
        label: label.into(),
        count: Some(tokens),
        derived: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        id: &'static str,
        name: &'static str,
        now: DateTime<Local>,
        month: i64,
        today: i64,
    ) -> Source {
        Source {
            id,
            name,
            totals: Totals {
                now,
                month,
                today,
                calls: 1,
            },
        }
    }

    #[test]
    fn personal_budget_changes_the_headline_without_losing_source_totals() {
        let now = Local
            .with_ymd_and_hms(2028, 2, 15, 12, 0, 0)
            .single()
            .unwrap();
        let sources = [
            source("cli", "Gemini CLI", now, 60, 20),
            source("opencode", "OpenCode", now, 90, 30),
        ];
        let counted = windows(&sources, None, &now).unwrap();
        assert_eq!(
            counted
                .iter()
                .map(|window| window.id.as_str())
                .collect::<Vec<_>>(),
            vec!["month", "today", "cli", "opencode"]
        );
        assert_eq!(
            counted
                .iter()
                .map(|window| window.count)
                .collect::<Vec<_>>(),
            vec![Some(150), Some(50), Some(60), Some(90)]
        );
        let budgeted = windows(&sources, Some(2_000_000), &now).unwrap();
        assert_eq!(budgeted[0].used, 150.0 / 2_000_000.0);
        assert_eq!(budgeted[0].count, None);
        assert!(budgeted[0].label.ends_with("personal budget 2.0M"));
        assert!(budgeted.iter().all(|window| window.derived));
        assert_eq!(windows(&sources, Some(100), &now).unwrap()[0].used, 1.5);
    }

    #[test]
    fn month_resets_at_the_next_local_month_after_a_leap_february() {
        let now = Local
            .with_ymd_and_hms(2028, 2, 29, 23, 0, 0)
            .single()
            .unwrap();
        let month = monthly_window(0, None, &now).unwrap();
        let reset = Local
            .timestamp_millis_opt(month.resets_at.unwrap() as i64)
            .single()
            .unwrap();
        assert_eq!((reset.year(), reset.month(), reset.day()), (2028, 3, 1));
        let (start, end) = month_bounds(&now).unwrap();
        assert_eq!(month.duration_seconds, Some((end - start) as f64 / 1000.0));
    }

    #[test]
    fn compact_budget_labels_match_the_mac_format() {
        assert_eq!(
            [9_999, 10_000, 999_999, 1_000_000, 2_450_000].map(compact),
            ["9999", "10k", "999k", "1.0M", "2.5M"]
        );
    }
}
