# Frozen provider feasibility matrix

Baseline: macOS source `6f0eb970201d895627e9c8ed9792d7befa27f809`, registered providers in `AppDelegate.swift`. This is a source/API feasibility map, not a claim of live-account verification.

| Family | Windows input / adapter | Semantics to preserve |
| --- | --- | --- |
| Claude | `.claude[-profile]` credentials, sessions, hook helper | Session/weekly windows; independent profiles, retry deadlines, working/waiting state |
| Cursor | Read-only `%APPDATA%/Cursor/User/globalStorage/state.vscdb` | Signed-in editor account, billed windows, editor activity |
| Codex | `.codex[-profile]/auth.json`, rollout logs | Read-only session; primary/secondary windows; profile isolation |
| Antigravity | Windows language-server process/local bridge and local logs | Official quotas when available, otherwise explicitly derived counts |
| GLM | Claude settings, `.zcode/v2`, OpenCode auth | Z.ai/BigModel host ownership; skip encrypted ZCode credentials; HTTP-200 error envelopes |
| Grok | `.grok/auth.json` | Only xAI-issued sessions; credits/current period; no invented billing date |
| OpenCode | `.local/share/opencode/auth.json` or XDG data location | `opencode-go` plan token; rolling/weekly/monthly windows |
| Command Code | `COMMAND_CODE_API_KEY` or `.commandcode/auth.json` | Organization-scoped billing requests; monthly, five-hour, weekly caps |
| GitHub Copilot | GitHub environment token or `gh auth token` | Existing account; premium/chat/completion quotas; unlimited remains unlimited |
| Ollama Local | Loopback `127.0.0.1:11434/api/ps` | Loaded models and memory, not an invented vendor usage allowance |
| Ollama Cloud | `OLLAMA_API_KEY` or Notch-owned Windows Credential Manager entry | Monthly/legacy usage fractions and model counts; reset only when published |
| Gemini API usage | `.gemini/tmp`, OpenCode and Hermes SQLite logs | Local token counts and optional user budget; never read API secrets |

Each source has a native Windows filesystem/process/HTTP equivalent; no Swift-to-Rust bridge or vendor-login replacement is required. The implementation must verify these paths with fixtures and platform probes. If an actual unsupported vendor behavior is discovered, record it and block parity qualification rather than silently removing the provider.

Borrowed credentials are never modified or refreshed. Only Notch-owned keys are stored/deleted by Notch. Turning a provider off must prevent both credential access and polling, including on launch.
