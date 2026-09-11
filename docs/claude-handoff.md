# Notch implementation handoff for Claude

Updated September 11, 2026. Continue implementation in the existing checkout. This is a handoff of work in progress, not a launch approval or a completed parity claim.

> **Status update:** The Gemini slice below was reviewed, completed and committed as `553063d`; both native lanes passed in [run 34575972466](https://github.com/moha-b/Notch/actions/runs/34575972466). Review follow-ups aligned per-source window ids and the compact budget label with the Mac provider. The preview helper now injects its bridge into pages without `<head>`. Continue with the remaining work after Gemini. Live-account validation of Gemini sources is still pending. Follow-up commits through `c581b68` (optional Command Code organization, disabled-provider activity gating, Claude transcript watcher pause) passed both native lanes in [run 34577329689](https://github.com/moha-b/Notch/actions/runs/34577329689). Open owner decision: whether a disabled provider's hook and session activity should be ignored or only hidden on both platforms; the Mac session monitors currently ignore the disconnected set.

## Start here

The user approved one product named **Notch**, with matching native Mac and Windows apps, straightforward installers, and coordinated updates. Architecture and release decisions were already debated with Claude and approved. Do not restart planning or rewrite the Mac app in Tauri.

Work in `D:\Projects\Notch` on branch `feat/notch-product`. Read this document, `docs/implementation-status.md`, `docs/provider-feasibility.md`, `docs/releases.md`, and the current working-tree diff. **First finish and validate the uncommitted Gemini work described below. Preserve all existing changes.**

The user prefers concise English replies using `C:/Users/mohab/.agents/skills/caveman/SKILL.md`. Persisted documentation and source comments remain normal prose. Apply `clean-code-guard/SKILL.md` from the same skill directory to production changes and `test-guard/SKILL.md` to tests. Review actual diffs before committing; report concrete fixes and remaining findings. Do not spawn other agents unless the user or an applicable instruction explicitly requests delegation.

Follow the supplied AGENTS.md: use Semble to locate implementations, then read the returned paths directly. Use grep only for all occurrences of a literal string. CLI fallback: `uvx --from "semble[mcp]==0.5.5" semble search "query" D:/Projects/Notch`.

## Repository state

- Origin: https://github.com/moha-b/Notch.git (renamed from `moha-b/codenotch-windows`).
- Upstream: https://github.com/Im-Midi/codenotch-windows.git.
- HEAD: `dd8d809495a485a32e82391ec0037597de1a77d5`, already pushed to `feat/notch-product`.
- Preserve `feat/windows-installer` at `2444ca14a1d0555c5db409bbca083fd247f043fb` and the original upstream contribution PR #2. They are separate from product development.
- Preserve `local-starter`, containing the initial local repository commit `5ff908a`.
- No product merge to main or public Notch release has been performed. No Notch product PR has been created in this implementation session.
- When pushing the existing product branch, use `git push origin feat/notch-product` explicitly; do not rely on potentially old tracking configuration.
- Current Gemini changes and this handoff exist only in the local checkout until committed. A fresh clone of GitHub alone will not include them.

## Approved scope and constraints

- One repository: `apps/macos`, `apps/windows`, and `shared` design tokens, fixtures, and contracts.
- Mac: Swift/AppKit/SwiftUI, macOS 15+, Apple Silicon. Windows: Rust/Tauri/WebView2, Windows 11 x64.
- Mac source is a provenance-recorded snapshot of `vinzdg/codenotch` commit `6f0eb970201d895627e9c8ed9792d7befa27f809`. Duplicate Windows source, generated outputs, upstream site, and release binaries were excluded. Keep licenses and credits; see `docs/provenance.md`.
- Full parity before public launch: Claude, Cursor, Codex, Antigravity, GLM, Grok, OpenCode, Command Code, GitHub Copilot, Ollama Local, Ollama Cloud, and Gemini API usage. Disabled experimental providers are excluded. Upstream feature additions do not automatically expand this frozen scope.
- Match profiles/accounts, provider ordering and enablement, usage/status, four-edge placement, displays/focus, sizing/offsets/visibility, accents, reset displays, usage pace, session activity, peeks, sounds, application focus, threshold alerts, mute, settings, first run, release notes, and update controls. Native OS dialogs/fonts may differ.
- Do not fabricate provider readings or silently omit unsupported behavior. Record gaps and block parity qualification.
- Borrowed credentials are read-only. Owned Ollama Cloud keys use Keychain or Windows Credential Manager. Gemini reads usage logs/databases without API secrets. Disabled providers must perform no credential reads or polling; remaining activity paths need auditing.
- Notch has independent icons, app IDs, storage, hooks, signing keys, and feeds. Existing Codenotch installations remain separate, without silent settings or credential migration.
- Main merges prepare SHA development candidates; documentation-only changes run relevant checks without installers. Public publication requires the owner's approval and qualification.
- Stable versions use X.Y.Z, starting at 1.0.0; Windows numeric installer version is X.Y.Z.0. One source/version manifest coordinates both apps and an increasing Mac build number. Serialize publication, reject duplicate versions or older sources, never overwrite published assets.
- Both platforms must pass before publication. Publish packages first, then advance Sparkle and Tauri feeds together through one Pages deployment. Failure preserves previous deployed feeds. Recovery uses a higher version.
- Updates check at launch and daily, configurable. Require Update before downloading and confirmation before restarting/installing. Offline failures leave the current app usable. Updater signatures are mandatory.
- Owner Apple signing/notarization is required before public launch. Initial Windows installers may be Authenticode-unsigned; that does not waive updater signatures. Intel Mac, Windows ARM64, public beta channels, and automatic installation are deferred.

## Implemented and committed

These are implemented slices, not evidence that every parity requirement is complete.

### Foundation and identity

- Repository reorganized; native Mac snapshot imported with provenance and licenses preserved.
- Original Notch SVG icon and generated platform assets; independent stable identity `io.github.moha-b.notch` and development `.preview` identity.
- Preview/stable storage, startup entries, hooks, and ports are isolated (48766/48767). Mac development DMG installs `Notch Preview.app` separately.
- Claude hook merging preserves unrelated hooks and Codenotch entries. Malformed settings are backed up/rejected rather than overwritten blindly.
- Shared release-note generation and design-token generation produce platform-specific resources.

### Providers, profiles, and credential handling

- All twelve provider families have Windows adapter implementations. Real account/source coverage remains incomplete; adapters alone do not satisfy parity.
- Named Claude/Codex profiles have separate credentials and cached readings, with profile-specific stale log fallback. Profiles are discovered at startup; new profiles currently require relaunch. Profile session/hook integration remains unfinished.
- Additional adapters live under `apps/windows/notch/src/providers/`. They include persisted backoff, stale snapshots, and disabled-provider guards in the polling path.
- Owned Ollama Cloud keys use Windows Credential Manager. Gemini reads Gemini CLI, OpenCode, and Hermes usage sources.
- Shared HTTP handling rejects redirects and non-2xx responses before JSON parsing. Errors do not expose response bodies. Cursor uses this path and preserves rate-limit deadlines across refresh/restart.
- GitHub CLI token lookup has a five-second deadline, bounded output, hidden process execution, and cleanup of its own child process.
- Both platforms reject lookalike Grok issuer names. Windows chooses a live trusted session without refreshing borrowed credentials.

### UI and behavior

- Shared palette and scale generate Swift constants and CSS. Windows usage pace follows Mac logic and shared JSON fixtures, with a settings toggle.
- Known/reported usage durations are carried for Claude, Codex, Cursor, Grok, GLM, OpenCode, and supported Copilot boundaries. Unknown periods do not display a pace estimate.
- Codex relative reset times use the log record timestamp, preventing old logs from restarting their timer on reread.
- Session transitions drive peeks and optional Windows system sounds. Threshold alerts trigger at 80% and 100%; muted crossings are consumed without replay after unmute.
- Invalid settings preserve a recovery copy and disable providers. New profiles remain disabled during first run/recovery until enabled.
- Edge geometry bounds provider strips and cards. Browser fixtures exercised one/twelve providers, four edges, extreme offsets, and 50%/200% scale. These are not native screenshot baselines.
- Windows native regions restrict interaction to visible controls; actual desktop click-through and mixed-DPI behavior remain unqualified.

### Release and updater infrastructure

- `.github/workflows/ci.yml` runs shared checks, Windows installer build/tests/lifecycle checks, and Mac tests/development DMG packaging.
- `.github/workflows/release.yml` and `scripts/release/` implement source/version gates, shared stamping, two-platform packages, signing, checksums, draft/public publication, and coordinated feeds.
- Publication reserves source tags atomically. Retrying a public release verifies the same manifest, source, assets, checksums, signatures, and notes; no asset overwrites or feed rollback.
- Sparkle remains on Mac; Tauri updater is integrated on Windows. Public updater is disabled in development builds. Consent is implemented, but signed end-to-end update scenarios remain pending.
- GitHub `production` environment was configured with required reviewer `moha-b` and only `main` permitted. `prevent_self_review` is false. Signing secrets and Pages setup remain owner requirements; exact variable/secret names are in `docs/releases.md`.

## Verified test evidence

Latest completed native run was rechecked through GitHub during this handoff:

- **`dd8d809` — both platforms and shared checks passed:** [run 34515435708](https://github.com/moha-b/Notch/actions/runs/34515435708). Includes Windows installer build, `cargo test --locked --workspace`, installer lifecycle; Mac `make test-ci` and `make candidate`; design generation, release policy, alerts, geometry, and pace checks.
- Earlier shared design/pace batch `b4f63d6`: [run 34514604491](https://github.com/moha-b/Notch/actions/runs/34514604491), both platforms passed.
- Earlier alerts/layout batch `496e92d`: [run 34478073633](https://github.com/moha-b/Notch/actions/runs/34478073633), both platforms passed.
- Credential regression tests use real local HTTP listeners and child processes: HTTP status/redaction, no Bearer/Cookie redirect forwarding, subprocess timeout, malformed/oversized tokens, trusted issuer boundaries, and expired/live session selection.
- Pace has shared fixtures consumed by JS and Swift. Profile isolation, settings recovery, provider parsing, and release policy tests also exist.

**None of these runs includes the current uncommitted Gemini changes.** All eleven categories in `release/qualification.json` remain pending on both platforms; its source SHA is empty. Do not mark them passed from CI alone.

## Current uncommitted Gemini slice: finish this first

Modified files:

- `apps/windows/notch/src/config.rs`
- `apps/windows/notch/src/config/validation.rs`
- `apps/windows/notch/src/providers/credentials.rs`
- `apps/windows/notch/src/providers/gemini.rs`
- `apps/windows/notch/src/providers/mod.rs`
- `apps/windows/notch/ui/settings.html`
- `apps/windows/notch/ui/settings.js`

New, untracked source files that must be retained and eventually staged:

- `apps/windows/notch/src/providers/gemini/summary.rs`
- `apps/windows/notch/src/providers/gemini/database_tests.rs`

Intended behavior already written:

- Optional `gemini_monthly_budget: Option<u64>` defaults to none. Accept positive safe JS integers up to 9007199254740991; reject zero/oversized values. Blank UI input saves null; HTML numeric validation prevents invalid values silently becoming null through JSON serialization.
- Gemini polling reads the current budget from settings. The generic disabled-provider guard remains before the fetch closure.
- A single local clock is used throughout a reading. Monthly and daily totals use local calendar boundaries. No budget shows token counts; a budget shows a derived usage fraction, clearly labeled personal budget rather than Google's quota. Preserve per-source monthly totals and the daily aggregate.
- CLI JSONL is streamed, older files are skipped by modification time, duplicate message IDs retain their last token record, and malformed/partial lines follow the pinned source's handling.
- OpenCode uses its shared XDG/default data-directory resolver. SQLite queries are read-only and filter from local month start. Count Google API calls, exclude Vertex, prefer authoritative token totals, otherwise sum components.
- Hermes excludes unrelated/old sessions and avoids double-counting reasoning tokens.
- Added real temporary SQLite tests that verify expected totals and unchanged source database bytes; summary tests cover aggregate/source totals, over-budget fraction, and leap-February calendar reset. These tests have been written, **not run natively**.

Next actions for this slice:

1. Review the complete diff under clean-code-guard and test-guard. The CLI reader is still long; extract a focused helper if needed. Review calendar/DST boundaries, future/invalid timestamps, deduplication, SQLite source assumptions, and budget validation without inventing readings.
2. Complete the budget settings browser check. The disposable preview helper `work/layout-server.cjs` injects a fake Tauri bridge only by replacing literal `<head>`. `settings.html` lacks that literal, so its preview showed empty providers and an unpopulated budget. This is a suspected fixture injection defect, not proof of a production settings defect. Fix only the disposable helper first, then inspect the real UI behavior.
3. Run syntax/shared checks, finish review, commit the completed slice, and push the explicit product branch. Watch both native jobs and fix any failures before claiming it verified.
4. Update implementation status with the exact successful commit/run. Do not label Gemini or overall provider parity complete until remaining source/account behavior is validated.

## Remaining work after Gemini

Prioritize bounded slices with evidence rather than broad rewrites:

1. **Provider parity:** complete account metadata and source coverage; validate live accounts; audit all disabled-provider credential/log/process paths. Activity watchers may keep reading logs until their presence refresh, and an already-running default Claude retry needs a toggle/cancellation review. Investigate Command Code's Windows organization requirement versus optional Mac organization, generic cloud negative-fraction clamping, and Antigravity duration coverage. These are leads, not all confirmed regressions.
2. **Profile activity:** connect session identity, hook installation, application focus, and mute/alerts to the correct Claude/Codex profile; preserve unrelated hooks and prove isolation.
3. **Display and native UI parity:** finish all-display/visibility modes, focus behavior, matching settings and first-run flows, and provider glyphs. Review current-monitor DPR versus primary-monitor assumptions and legacy edge-specific dragging. Test native click-through and mixed DPI. Capture separate Mac and Windows screenshots for shared scenarios.
4. **Updater/release qualification:** exercise explicit consent, offline retry, invalid signatures, wrong platform, cross-version upgrades, failed publication/Pages deployment, and duplicate/concurrent release requests. Policy unit tests are not full signed publication proof.
5. **Launch gates:** clean installs/uninstall cleanup, corrupted settings, native UI review, live providers, and owner signing/notarization/Pages configuration. Keep `release/qualification.json` pending until evidence exists for the selected source. Ask the owner only for genuinely required credentials or final publication approval.

## Local tools and useful commands

PowerShell is the local shell. Native Rust linking could not run here because the MSVC SDK/toolchain was unavailable; GitHub Windows CI provides native execution. Mac builds/tests run on the Mac CI lane. An ignored isolated Rust toolchain exists under `work/toolchain` for formatting:

```powershell
$env:RUSTUP_HOME='D:/Projects/Notch/work/toolchain/rustup'
$env:CARGO_HOME='D:/Projects/Notch/work/toolchain/cargo'
./work/toolchain/cargo/bin/rustfmt.exe --edition 2021 --config skip_children=true <touched-rust-file>
```

Avoid formatting unrelated legacy modules. No new dependency is needed for the current slice. `node --test` hit sandbox spawn EPERM previously; directly invoking these test modules works:

```powershell
node scripts/generate-design.mjs --check
node scripts/release/policy.test.mjs
node apps/windows/scripts/alerts.test.mjs
node apps/windows/scripts/geometry.test.mjs
node apps/windows/scripts/usage-pace.test.mjs
node --check apps/windows/notch/ui/settings.js
git -c core.safecrlf=false diff --check
```

Prior syntax/rustfmt/diff checks passed for the Gemini edits; they do not prove compilation or runtime behavior. Run relevant checks again after finishing the slice.

`work/` contains disposable fixture/toolchain files and must not be committed. Preview helper used localhost port 4178; its former terminal session is no longer available, so check the port before restarting it. Browser bindings and terminal session IDs from the preceding agent are not portable to Claude. Recreate browser state as needed. Fake provider readings belong only in fixtures, never production.

Git metadata was read-only inside Codex's sandbox, and network commands required escalation. Use your environment's permission mechanism as needed; do not interpret a local sandbox denial as a repository failure. Do not reset the working tree, merge main, publish, modify upstream contribution history, or expose credentials while continuing.

## Suggested continuation prompt

> Continue Notch implementation in D:\Projects\Notch. Read docs/claude-handoff.md and docs/implementation-status.md first, then inspect the existing diff. Finish and validate the uncommitted Gemini budget/log slice before moving to remaining parity work. Preserve branch history and all local changes. Use caveman for concise replies, clean-code-guard for production code, and test-guard for tests. Work through bounded implementation slices; do not restart architecture planning. Do not merge main or publish a public release without my approval. Report exact test evidence and remaining gaps honestly.
