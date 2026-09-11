use super::{home, json_file, Failure};
use serde_json::Value;

pub fn token(entry: &Value) -> Option<String> {
    entry
        .as_str()
        .or_else(|| {
            [
                "apiKey",
                "api_key",
                "token",
                "key",
                "accessToken",
                "auth_token",
            ]
            .iter()
            .find_map(|field| entry[*field].as_str())
        })
        .filter(|token| !token.trim().is_empty() && !token.starts_with("enc:v1:"))
        .map(str::to_owned)
}

pub fn opencode_auth() -> Result<Value, Failure> {
    Ok(json_file(&opencode_directory()?.join("auth.json"))?.unwrap_or(Value::Null))
}

pub(super) fn opencode_directory() -> Result<std::path::PathBuf, Failure> {
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or(home()?.join(".local/share"));
    Ok(root.join("opencode"))
}

pub fn glm() -> Result<(String, &'static str), Failure> {
    let root = home()?;
    if let Some(settings) = json_file(&root.join(".claude/settings.json"))? {
        let env = &settings["env"];
        if let Some(host) = env["ANTHROPIC_BASE_URL"].as_str().and_then(console) {
            if let Some(key) = env["ANTHROPIC_AUTH_TOKEN"]
                .as_str()
                .or_else(|| env["ANTHROPIC_API_KEY"].as_str())
            {
                if !key.is_empty() {
                    return Ok((key.into(), host));
                }
            }
        }
    }
    if let Some(credential) = zcode(&root)? {
        return Ok(credential);
    }
    let auth = opencode_auth()?;
    for id in [
        "zai-coding-plan",
        "zai",
        "z-ai",
        "z.ai",
        "glm",
        "zhipu",
        "zhipuai",
    ] {
        if let Some(key) = token(&auth[id]) {
            return Ok((
                key,
                if id.starts_with("zhipu") {
                    "https://open.bigmodel.cn"
                } else {
                    "https://api.z.ai"
                },
            ));
        }
    }
    Err(Failure::NeedsAuth)
}

fn console(base: &str) -> Option<&'static str> {
    let url = tauri::Url::parse(base).ok()?;
    if url.scheme() != "https" {
        return None;
    }
    match url.host_str()? {
        "api.z.ai" | "z.ai" => Some("https://api.z.ai"),
        "open.bigmodel.cn" | "bigmodel.cn" => Some("https://open.bigmodel.cn"),
        _ => None,
    }
}

fn zcode(root: &std::path::Path) -> Result<Option<(String, &'static str)>, Failure> {
    if let Some(config) = json_file(&root.join(".zcode/v2/config.json"))? {
        if let Some(providers) = config["provider"].as_object() {
            for (id, provider) in providers {
                if !id.contains("coding-plan") || provider["enabled"] == false {
                    continue;
                }
                if let Some(key) = token(&provider["options"]["apiKey"]) {
                    let host = match provider["options"]["baseURL"].as_str() {
                        Some(base) => match console(base) {
                            Some(host) => host,
                            None => continue,
                        },
                        None => "https://api.z.ai",
                    };
                    return Ok(Some((key, host)));
                }
            }
        }
    }
    let credential = json_file(&root.join(".zcode/v2/credentials.json"))?.unwrap_or(Value::Null);
    Ok(token(&credential["oauth:zai:access_token"]).map(|key| (key, "https://api.z.ai")))
}

pub fn grok() -> Result<String, Failure> {
    let auth = json_file(&home()?.join(".grok/auth.json"))?.ok_or(Failure::NeedsAuth)?;
    trusted_grok_token(&auth).ok_or(Failure::NeedsAuth)
}

pub fn trusted_grok_token(auth: &Value) -> Option<String> {
    auth.as_object()?
        .iter()
        .filter(|(issuer, entry)| {
            issuer.split("::").next() == Some("https://auth.x.ai")
                || entry["oidc_issuer"].as_str() == Some("https://auth.x.ai")
        })
        .filter(|(_, entry)| {
            super::reset(&entry["expires_at"]).is_none_or(|expiry| expiry > super::now_ms())
        })
        .find_map(|(_, entry)| token(&entry["key"]))
}

pub fn commandcode() -> Result<String, Failure> {
    if let Ok(key) = std::env::var("COMMAND_CODE_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    let auth = json_file(&home()?.join(".commandcode/auth.json"))?.ok_or(Failure::NeedsAuth)?;
    token(&auth["apiKey"]).ok_or(Failure::NeedsAuth)
}

pub fn copilot() -> Result<String, Failure> {
    for name in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(key) = std::env::var(name) {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }
    super::github_cli::token()
}
