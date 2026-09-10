use crate::{codex, cursor, usage::LimitWindow};
use serde_json::json;

#[test]
fn old_cached_windows_keep_their_reading_without_inventing_a_duration() {
    let cached: LimitWindow = serde_json::from_str(
        r#"{"id":"weekly","label":"Weekly","used":0.42,"resets_at":1800000000000}"#,
    )
    .unwrap();
    assert_eq!(cached.used, 0.42);
    assert_eq!(cached.duration_seconds, None);
}

#[test]
fn cursor_billing_periods_use_reported_dates_only() {
    for (start, end, expected) in [
        (
            json!("2028-02-01T00:00:00Z"),
            json!("2028-03-01T00:00:00Z"),
            Some(29.0 * 86400.0),
        ),
        (json!(null), json!("2028-03-01T00:00:00Z"), None),
        (
            json!("2028-03-02T00:00:00Z"),
            json!("2028-03-01T00:00:00Z"),
            None,
        ),
    ] {
        let (windows, _) = cursor::parse_summary(&json!({"billingCycleStart":start,
            "billingCycleEnd":end,"individualUsage":{"plan":{"totalPercentUsed":42}}}));
        assert_eq!(windows[0].duration_seconds, expected);
    }
}

#[test]
fn rollout_relative_reset_uses_recorded_time_instead_of_restarting_the_clock() {
    for (timestamp, expected) in [
        (json!("2027-01-15T08:00:00Z"), Some(1800003600000)),
        (json!(null), None),
    ] {
        let rollout = json!({"timestamp":timestamp,"payload":{"rate_limits":{"primary":
            {"used_percent":42,"window_minutes":180,"resets_in_seconds":3600}}}})
        .to_string();
        let (windows, _, _) = codex::snapshot_from_rollout(&rollout).unwrap();
        assert_eq!(windows[0].duration_seconds, Some(10800.0));
        assert_eq!(windows[0].resets_at, expected);
    }
}
