use super::*;
use rusqlite::{params, Connection};
use serde_json::json;

struct DatabaseFixture(std::path::PathBuf);
impl DatabaseFixture {
    fn new(name: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "notch-gemini-{name}-{}-{}.db",
            std::process::id(),
            super::super::now_ms()
        )))
    }
}
impl Drop for DatabaseFixture {
    fn drop(&mut self) {
        // Panicking here during a failed assertion would abort the test binary.
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn opencode_counts_only_this_months_google_calls_without_changing_the_database() {
    let fixture = DatabaseFixture::new("opencode");
    let now = Local
        .with_ymd_and_hms(2028, 2, 15, 12, 0, 0)
        .single()
        .unwrap();
    let (month_start, _) = summary::month_bounds(&now).unwrap();
    let connection = Connection::open(&fixture.0).unwrap();
    connection.execute_batch("CREATE TABLE message(time_created INTEGER, data TEXT); CREATE INDEX by_time ON message(time_created);").unwrap();
    for (at, provider, tokens) in [
        (
            now.timestamp_millis(),
            "google",
            json!({"total":100,"input":999}),
        ),
        (
            now.timestamp_millis(),
            "google",
            json!({"input":10,"output":20,"reasoning":3,"cache":{"read":4,"write":5}}),
        ),
        (
            now.timestamp_millis(),
            "google-vertex",
            json!({"total":999}),
        ),
        (month_start - 1, "google", json!({"total":999})),
        (now.timestamp_millis(), "google", json!({"total":0})),
    ] {
        connection
            .execute(
                "INSERT INTO message VALUES(?1, ?2)",
                params![
                    at,
                    json!({"role":"assistant","providerID":provider,"tokens":tokens}).to_string()
                ],
            )
            .unwrap();
    }
    drop(connection);
    let original = std::fs::read(&fixture.0).unwrap();
    let totals = opencode(&fixture.0, &now).unwrap().unwrap();
    assert_eq!((totals.month, totals.today, totals.calls), (142, 142, 2));
    assert_eq!(std::fs::read(&fixture.0).unwrap(), original);
}

#[test]
fn hermes_does_not_double_count_reasoning_or_include_old_and_unrelated_sessions() {
    let fixture = DatabaseFixture::new("hermes");
    let now = Local
        .with_ymd_and_hms(2028, 2, 15, 12, 0, 0)
        .single()
        .unwrap();
    let (month_start, _) = summary::month_bounds(&now).unwrap();
    let connection = Connection::open(&fixture.0).unwrap();
    connection.execute_batch("CREATE TABLE session_model_usage(last_seen REAL, billing_provider TEXT, input_tokens INTEGER, output_tokens INTEGER, cache_read_tokens INTEGER, cache_write_tokens INTEGER, reasoning_tokens INTEGER, api_call_count INTEGER);").unwrap();
    for (at, provider) in [
        (now.timestamp_millis(), "gemini"),
        (now.timestamp_millis(), "other"),
        (month_start - 1000, "gemini"),
    ] {
        connection
            .execute(
                "INSERT INTO session_model_usage VALUES(?1, ?2, 10, 40, 20, 30, 100, 3)",
                params![at as f64 / 1000.0, provider],
            )
            .unwrap();
    }
    drop(connection);
    let original = std::fs::read(&fixture.0).unwrap();
    let totals = hermes(&fixture.0, &now).unwrap().unwrap();
    assert_eq!((totals.month, totals.today, totals.calls), (100, 100, 3));
    assert_eq!(std::fs::read(&fixture.0).unwrap(), original);
}
