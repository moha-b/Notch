use super::*;
use serde_json::json;

#[test]
fn vendor_units_and_unknown_resets_survive_normalization() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../../../../shared/provider-fixtures.json")).unwrap();
    for fixture in fixtures {
        let response = &fixture["response"];
        let windows = match fixture["provider"].as_str().unwrap() {
            "glm" => cloud::parse_glm(response),
            "grok" => cloud::parse_grok(response),
            "opencode" => cloud::parse_opencode(response),
            "copilot" => cloud::parse_copilot(response),
            "ollama" => cloud::parse_ollama(response),
            "ollama-local" => cloud::parse_ollama_local(response),
            "commandcode" => {
                cloud::parse_commandcode(response, &fixture["summary"], &fixture["period"])
            }
            provider => panic!("Fixture has no adapter: {provider}"),
        }
        .unwrap();
        assert_eq!(windows.len(), 1, "{}", fixture["provider"]);
        if let Some(count) = fixture["count"].as_i64() {
            assert_eq!(windows[0].count, Some(count));
        }
        if let Some(used) = fixture["used"].as_f64() {
            assert!((windows[0].used - used).abs() < 0.000001);
        }
        assert_eq!(
            windows[0].resets_at,
            fixture["reset"].as_u64(),
            "{}",
            fixture["provider"]
        );
    }
}

#[test]
fn glm_http_success_does_not_hide_auth_or_rate_limit_failure() {
    assert!(matches!(
        cloud::parse_glm(&json!({"code":401,"success":false})),
        Err(Failure::NeedsAuth)
    ));
    assert!(matches!(
        cloud::parse_glm(&json!({"code":429,"success":false})),
        Err(Failure::RateLimited(_))
    ));
}

#[test]
fn failures_preserve_last_reading_and_backoff_does_not_reset_on_refresh() {
    let previous = UsageSnapshot {
        status: "ok".into(),
        fetched_at: 42,
        windows: vec![window("weekly", 0.7, None)],
        ..Default::default()
    };
    let mut attempts = 0;
    let limited = update_snapshot(previous, Err(Failure::RateLimited(500)), &mut attempts);
    assert_eq!(limited.status, "backoff");
    assert_eq!(limited.windows[0].used, 0.7);
    assert_eq!(limited.fetched_at, 42);
    assert!(limited.backoff_until > now_ms() + 490_000);
    let stale = update_snapshot(
        limited.clone(),
        Err(Failure::Unavailable("Offline".into())),
        &mut attempts,
    );
    assert_eq!(stale.status, "stale");
    assert_eq!(stale.backoff_until, limited.backoff_until);
}

#[test]
fn grok_does_not_borrow_a_lookalike_issuer_credential() {
    let auth = json!({"https://auth.x.ai.attacker.test::client":{"key":"wrong"}});
    assert!(credentials::trusted_grok_token(&auth).is_none());
    let auth = json!({"https://auth.x.ai::client":{"key":"fixture-token"}});
    assert_eq!(
        credentials::trusted_grok_token(&auth).as_deref(),
        Some("fixture-token")
    );
}

#[test]
fn unlimited_and_unpublished_allowances_are_not_fabricated_as_zero() {
    assert!(cloud::parse_copilot(
        &json!({"quota_snapshots":{"chat":{"unlimited":true,"entitlement":300}}})
    )
    .unwrap()
    .is_empty());
    assert!(cloud::parse_opencode(
        &json!({"usage":{"rolling":{"resetsAt":"2027-01-15T08:00:00Z"}}})
    )
    .unwrap()
    .is_empty());
    assert!(cloud::parse_ollama(&json!({"limits":{}}))
        .unwrap()
        .is_empty());
}
