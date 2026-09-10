use super::{count_window, credentials, get, keyring, request, reset, window, Failure};
use crate::usage::LimitWindow;
use serde_json::Value;

pub fn glm() -> Result<Vec<LimitWindow>, Failure> {
    let (token, host) = credentials::glm()?;
    let endpoint = format!("{host}/api/monitor/usage/quota/limit");
    parse_glm(&get(
        request(&endpoint, &token).set("Authorization", &token)
    )?)
}

pub fn parse_glm(reply: &Value) -> Result<Vec<LimitWindow>, Failure> {
    match reply["code"].as_u64() {
        Some(401 | 403) => return Err(Failure::NeedsAuth),
        Some(429) => return Err(Failure::RateLimited(60)),
        Some(code) if code != 200 && code != 0 => {
            return Err(Failure::Invalid("GLM rejected the request."))
        }
        _ => {}
    }
    if reply["success"] == false {
        return Err(Failure::Invalid("GLM rejected the request."));
    }
    let limits = reply["data"]["limits"]
        .as_array()
        .ok_or(Failure::Invalid("GLM limits are missing."))?;
    Ok(limits.iter().filter_map(glm_window).collect())
}

fn glm_window(limit: &Value) -> Option<LimitWindow> {
    Some(LimitWindow {
        duration_seconds: glm_duration(limit),
        ..window(
            &glm_window_id(limit)?,
            limit["percentage"].as_f64()? / 100.0,
            limit["nextResetTime"].as_u64().filter(|stamp| *stamp > 0),
        )
    })
}

fn glm_duration(limit: &Value) -> Option<f64> {
    let number = limit["number"].as_f64().filter(|number| *number > 0.0)?;
    match limit["unit"].as_u64()? {
        3 => Some(number * 3600.0),
        6 => Some(number * 7.0 * 86400.0),
        _ => None,
    }
}

fn glm_window_id(limit: &Value) -> Option<String> {
    let id = match (
        limit["type"].as_str(),
        limit["unit"].as_u64(),
        limit["number"].as_u64(),
    ) {
        (Some("TIME_LIMIT"), _, _) => "mcp".into(),
        (_, Some(3), Some(5)) => "session".into(),
        (_, Some(6), Some(1)) => "weekly".into(),
        (_, Some(unit), Some(number)) => format!("window-{unit}x{number}"),
        (Some(kind), _, _) => kind.to_lowercase(),
        _ => return None,
    };
    Some(id)
}

pub fn grok() -> Result<Vec<LimitWindow>, Failure> {
    parse_grok(&get(request(
        "https://cli-chat-proxy.grok.com/v1/billing?format=credits",
        &credentials::grok()?,
    ))?)
}

pub fn parse_grok(reply: &Value) -> Result<Vec<LimitWindow>, Failure> {
    let config = reply
        .get("config")
        .ok_or(Failure::Invalid("Grok billing configuration is missing."))?;
    let (reset_at, duration_seconds) = grok_period(config);
    if let Some(percent) = config["creditUsagePercent"].as_f64() {
        return Ok(vec![LimitWindow {
            duration_seconds,
            ..window("credits", percent / 100.0, reset_at)
        }]);
    }
    let mut windows: Vec<_> = config["productUsage"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|product| {
            Some(LimitWindow {
                duration_seconds,
                ..window(
                    product["product"].as_str().unwrap_or("credits"),
                    product["usagePercent"].as_f64()? / 100.0,
                    reset_at,
                )
            })
        })
        .collect();
    // Grok's weekly-plan contract omits usage until the first charge in a new pool.
    if windows.is_empty()
        && config["currentPeriod"]["type"]
            .as_str()
            .is_some_and(|kind| kind.contains("WEEKLY"))
    {
        windows.push(window("credits", 0.0, reset_at));
    }
    Ok(windows)
}

fn grok_period(config: &Value) -> (Option<u64>, Option<f64>) {
    let current_end = reset(&config["currentPeriod"]["end"]);
    let reset_at = current_end.or_else(|| reset(&config["billingPeriodEnd"]));
    let start = if current_end.is_some() {
        reset(&config["currentPeriod"]["start"])
    } else {
        reset(&config["billingPeriodStart"])
    };
    (reset_at, crate::usage::period_duration(start, reset_at))
}

pub fn opencode() -> Result<Vec<LimitWindow>, Failure> {
    let auth = credentials::opencode_auth()?;
    let token = credentials::token(&auth["opencode-go"]).ok_or(Failure::NeedsAuth)?;
    parse_opencode(&get(request(
        "https://opencode.ai/zen/go/v1/usage",
        &token,
    ))?)
}

pub fn parse_opencode(reply: &Value) -> Result<Vec<LimitWindow>, Failure> {
    let usage = reply
        .get("usage")
        .ok_or(Failure::Invalid("OpenCode usage is missing."))?;
    Ok(["rolling", "weekly", "monthly"]
        .iter()
        .filter_map(|id| {
            let reset_at = reset(&usage[*id]["resetsAt"]);
            let duration_seconds = match *id {
                "rolling" => Some(5.0 * 3600.0),
                "weekly" => Some(7.0 * 86400.0),
                "monthly" => monthly_duration(reset_at),
                _ => None,
            };
            Some(LimitWindow {
                duration_seconds,
                ..window(id, usage[*id]["percent"].as_f64()? / 100.0, reset_at)
            })
        })
        .collect())
}

pub fn copilot() -> Result<Vec<LimitWindow>, Failure> {
    parse_copilot(&get(request(
        "https://api.github.com/copilot_internal/user",
        &credentials::copilot()?,
    )
    .set("X-GitHub-Api-Version", "2022-11-28"))?)
}

pub fn parse_copilot(reply: &Value) -> Result<Vec<LimitWindow>, Failure> {
    let quotas = reply["quota_snapshots"]
        .as_object()
        .ok_or(Failure::Invalid("Copilot quotas are missing."))?;
    Ok(quotas
        .iter()
        .filter_map(|(id, quota)| {
            if quota["unlimited"] == true {
                return None;
            }
            let cap = quota["entitlement"].as_f64().filter(|cap| *cap > 0.0)?;
            let fraction = quota["used"]
                .as_f64()
                .map(|used| used / cap)
                .or_else(|| {
                    quota["remaining"]
                        .as_f64()
                        .map(|remaining| (cap - remaining) / cap)
                })
                .or_else(|| {
                    quota["percent_remaining"]
                        .as_f64()
                        .map(|remaining| 1.0 - remaining / 100.0)
                })?;
            let reset_at = ["reset_date", "reset_at", "resets_at"]
                .iter()
                .find_map(|field| reset(&quota[*field]))
                .or_else(|| reset(&reply["quota_reset_date"]));
            Some(LimitWindow {
                duration_seconds: copilot_duration(reset_at),
                ..window(id, fraction, reset_at)
            })
        })
        .collect())
}

fn monthly_duration(reset_at: Option<u64>) -> Option<f64> {
    let end = chrono::DateTime::from_timestamp_millis(i64::try_from(reset_at?).ok()?)?;
    let start = end.checked_sub_months(chrono::Months::new(1))?;
    Some((end - start).num_milliseconds() as f64 / 1000.0)
}

fn copilot_duration(reset_at: Option<u64>) -> Option<f64> {
    use chrono::{Datelike, Timelike};
    let end = chrono::DateTime::from_timestamp_millis(i64::try_from(reset_at?).ok()?)?;
    if end.day() != 1 || end.num_seconds_from_midnight() != 0 {
        return None;
    }
    monthly_duration(reset_at)
}

pub fn ollama() -> Result<Vec<LimitWindow>, Failure> {
    let token = std::env::var("OLLAMA_API_KEY")
        .ok()
        .filter(|token| !token.is_empty())
        .map(Ok)
        .unwrap_or_else(keyring::read)?;
    parse_ollama(&get(request("https://ollama.com/api/usage", &token))?)
}

pub fn parse_ollama(reply: &Value) -> Result<Vec<LimitWindow>, Failure> {
    let limits = reply
        .get("limits")
        .ok_or(Failure::Invalid("Ollama limits are missing."))?;
    let mut windows = Vec::new();
    for id in ["monthly", "weekly", "session"] {
        if let Some(fraction) = limits[id]["usage"]
            .as_f64()
            .filter(|fraction| *fraction > 0.0)
        {
            // activity.period is a rolling report interval, never a billing reset.
            windows.push(window(id, fraction, None));
        }
        windows.extend(
            limits[id]["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|model| {
                    let count = model["request_count"].as_i64().filter(|count| *count > 0)?;
                    let name = model["name"].as_str()?;
                    Some(count_window(&format!("{id}.{name}"), count))
                }),
        );
    }
    Ok(windows)
}

pub fn ollama_local() -> Result<Vec<LimitWindow>, Failure> {
    parse_ollama_local(&get(ureq::get("http://127.0.0.1:11434/api/ps"))?)
}

pub fn parse_ollama_local(reply: &Value) -> Result<Vec<LimitWindow>, Failure> {
    let models = reply["models"]
        .as_array()
        .ok_or(Failure::Invalid("Ollama model list is missing."))?;
    Ok(models
        .iter()
        .filter_map(|model| {
            let name = model["name"].as_str().or_else(|| model["model"].as_str())?;
            let bytes = model["size_vram"]
                .as_i64()
                .filter(|bytes| *bytes > 0)
                .or_else(|| model["size"].as_i64())?;
            Some(count_window(&format!("{name} · memory bytes"), bytes))
        })
        .collect())
}

fn command_get(path: &str, token: &str) -> Result<Value, Failure> {
    get(
        request(&format!("https://api.commandcode.ai/alpha{path}"), token)
            .set("User-Agent", "command-code-desktop")
            .set("x-command-code-version", "desktop"),
    )
}

pub fn commandcode() -> Result<Vec<LimitWindow>, Failure> {
    let token = credentials::commandcode()?;
    let whoami = command_get("/whoami", &token)?;
    let org = whoami["org"]["id"]
        .as_str()
        .ok_or(Failure::Invalid("Command Code organization is missing."))?;
    let query = command_query(org, &Value::Null)?;
    let credits = command_get(&format!("/billing/credits?{query}"), &token)?;
    let subscriptions = command_get(&format!("/billing/subscriptions?{query}"), &token)?;
    let subscription = subscriptions.get("data").unwrap_or(&subscriptions);
    let period = subscription
        .as_array()
        .and_then(|rows| rows.first())
        .unwrap_or(subscription);
    let summary = command_get(
        &format!("/usage/summary?{}", command_query(org, period)?),
        &token,
    )?;
    parse_commandcode(&credits, &summary, period)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_periods_handle_leap_years_and_reject_unknown_copilot_boundaries() {
        for (stamp, monthly, copilot) in [
            ("2028-03-01T00:00:00Z", 29.0, Some(29.0)),
            ("2027-03-01T00:00:00Z", 28.0, Some(28.0)),
            ("2027-03-31T08:00:00Z", 31.0, None),
            ("2027-03-01T08:00:00Z", 28.0, None),
        ] {
            let reset_at = reset(&serde_json::json!(stamp));
            assert_eq!(monthly_duration(reset_at), Some(monthly * 86400.0));
            assert_eq!(
                copilot_duration(reset_at),
                copilot.map(|days| days * 86400.0)
            );
        }
    }

    #[test]
    fn glm_unknown_units_have_no_pace_duration() {
        for (unit, number, expected) in [
            (3, 5, Some(18000.0)),
            (6, 2, Some(1209600.0)),
            (9, 1, None),
            (3, 0, None),
        ] {
            let reply = serde_json::json!({"data":{"limits":[{"unit":unit,"number":number,"percentage":42}]}});
            assert_eq!(parse_glm(&reply).unwrap()[0].duration_seconds, expected);
        }
    }

    #[test]
    fn billing_query_preserves_organization_and_optional_timestamp() {
        for (period, expected) in [
            (serde_json::json!({}), None),
            (
                serde_json::json!({"currentPeriodStart":"2026-09-01T00:00:00Z"}),
                Some("2026-09-01T00:00:00+00:00"),
            ),
        ] {
            let query = command_query("organization +/?&", &period).unwrap();
            let url = tauri::Url::parse(&format!("https://api.commandcode.ai/?{query}")).unwrap();
            let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
            assert_eq!(
                pairs.get("orgId").map(|org| org.as_ref()),
                Some("organization +/?&")
            );
            assert_eq!(pairs.get("since").map(|since| since.as_ref()), expected);
        }
    }
}

fn command_query(org: &str, period: &Value) -> Result<String, Failure> {
    let mut url = tauri::Url::parse("https://api.commandcode.ai")
        .map_err(|_| Failure::Invalid("Invalid API URL."))?;
    url.query_pairs_mut().append_pair("orgId", org);
    if let Some(start) = reset(&period["currentPeriodStart"]) {
        let start = i64::try_from(start)
            .ok()
            .and_then(chrono::DateTime::from_timestamp_millis)
            .ok_or(Failure::Invalid("Invalid billing start."))?;
        url.query_pairs_mut()
            .append_pair("since", &start.to_rfc3339());
    }
    Ok(url.query().unwrap_or("").into())
}

pub fn parse_commandcode(
    credits: &Value,
    summary: &Value,
    period: &Value,
) -> Result<Vec<LimitWindow>, Failure> {
    let used = summary["totalCost"]
        .as_f64()
        .ok_or(Failure::Invalid("Command Code spend is missing."))?;
    let remaining = credits["credits"]["monthlyCredits"]
        .as_f64()
        .ok_or(Failure::Invalid("Command Code allowance is missing."))?;
    if used + remaining <= 0.0 {
        return Ok(Vec::new());
    }
    let mut windows = vec![window(
        "monthly",
        used / (used + remaining),
        reset(&period["currentPeriodEnd"]),
    )];
    for id in ["fiveHour", "weekly"] {
        let limit = &credits["windowLimits"][id];
        if let (Some(used), Some(cap)) = (
            limit["used"].as_f64(),
            limit["cap"].as_f64().filter(|cap| *cap > 0.0),
        ) {
            windows.push(window(id, used / cap, reset(&limit["resetAt"])));
        }
    }
    Ok(windows)
}
