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
    Some(window(
        &id,
        limit["percentage"].as_f64()? / 100.0,
        limit["nextResetTime"].as_u64().filter(|stamp| *stamp > 0),
    ))
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
    let reset_at =
        reset(&config["currentPeriod"]["end"]).or_else(|| reset(&config["billingPeriodEnd"]));
    if let Some(percent) = config["creditUsagePercent"].as_f64() {
        return Ok(vec![window("credits", percent / 100.0, reset_at)]);
    }
    let mut windows: Vec<_> = config["productUsage"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|product| {
            Some(window(
                product["product"].as_str().unwrap_or("credits"),
                product["usagePercent"].as_f64()? / 100.0,
                reset_at,
            ))
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
            Some(window(
                id,
                usage[*id]["percent"].as_f64()? / 100.0,
                reset(&usage[*id]["resetsAt"]),
            ))
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
            Some(window(id, fraction, reset_at))
        })
        .collect())
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
    let mut query = tauri::Url::parse("https://api.commandcode.ai")
        .map_err(|_| Failure::Invalid("Invalid API URL."))?;
    query.query_pairs_mut().append_pair("orgId", org);
    let query = query.query().unwrap_or("");
    let credits = command_get(&format!("/billing/credits?{query}"), &token)?;
    let subscriptions = command_get(&format!("/billing/subscriptions?{query}"), &token)?;
    let period = subscriptions["data"]
        .as_array()
        .and_then(|rows| rows.first())
        .unwrap_or(&subscriptions["data"]);
    let start = reset(&period["currentPeriodStart"])
        .ok_or(Failure::Invalid("Command Code billing period is missing."))?;
    let summary = command_get(
        &format!(
            "/usage/summary?{query}&since={}",
            chrono::DateTime::from_timestamp_millis(start as i64)
                .ok_or(Failure::Invalid("Invalid billing start."))?
                .to_rfc3339()
        ),
        &token,
    )?;
    parse_commandcode(&credits, &summary, period)
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
