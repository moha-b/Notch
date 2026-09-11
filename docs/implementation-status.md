# Implementation and qualification status

Notch remains a development product. Passing build lanes do not establish feature parity or authorize publication.

## Verified milestones

- Native tests, Windows installer lifecycle and Mac development DMG passed for `c581b68` in [candidate run 34577329689](https://github.com/moha-b/Notch/actions/runs/34577329689). This includes the Command Code organization and disabled-provider fixes, the Gemini budget/log slice (first passing for `553063d` in [run 34575972466](https://github.com/moha-b/Notch/actions/runs/34575972466)), and the earlier credential-hardening batch ([run 34515435708](https://github.com/moha-b/Notch/actions/runs/34515435708) for `dd8d809`).
- Named Windows Claude and Codex profiles use separate credentials and cached readings. Tests cover credential separation and profile-specific stale rollout fallback. Profile session activity and hooks still need parity work.
- The GitHub `production` environment requires `moha-b` review and permits only the `main` branch. Apple and updater signing credentials remain owner setup requirements.
- Release preparation rejects existing versions and tags. Publication creates its source tag atomically; a failed feed publication can resume only after verifying the same public build. This recovery path has policy tests; a complete signed publication exercise remains pending.

## Latest development changes

- Shared palette and design scale generate native Swift constants and CSS. Windows usage pace follows the Mac calculation and the same JSON behavior fixtures; its settings toggle is verified in a browser fixture. Both native pace test suites passed in the candidate run above.
- Credential hardening passed both native lanes: shared HTTP handling rejects redirects before parsing, Cursor preserves rate-limit deadlines across refresh/restart, and GitHub CLI token lookup has a five-second deadline with bounded output and process cleanup. Local HTTP and subprocess tests exercise these boundaries without real credentials.
- Both platforms reject lookalike Grok issuer names. Windows chooses a live trusted session and leaves expired credentials for Grok to refresh.
- Windows Command Code no longer requires an organization: like the Mac app, a `whoami` reply without one omits `orgId` from billing queries. Live account validation remains pending.
- Disabling a provider now stops Windows activity log reads on the next two-second tick instead of after the minute-long presence refresh. Disabling Claude during a request discards the reply and skips the credential re-read retry. The Claude transcript watcher pauses while Claude is disabled and rescans when it is re-enabled. Hook events are still accepted and hidden with the disabled provider; the Mac session monitors do not consult the disconnected set, so the intended cross-platform behavior for disabled-provider session activity still needs a decision. Named Claude/Codex profiles have no Windows session-activity path yet; their usage polling runs only through the shared poll loop, which skips disabled providers.
- Windows carries known Claude/Codex/GLM durations and reported Cursor/Grok billing intervals. OpenCode monthly cycles use calendar arithmetic; Copilot pace requires its first-of-month UTC boundary. Unknown periods have no pace. Antigravity and Gemini duration coverage remains part of provider parity work.
- Codex rollout relative resets use the recorded timestamp, preventing stale logs from restarting their reset timer when reread.
- Session transitions drive peeks and optional Windows system sounds. Threshold alerts trigger at 80% and 100%; mute consumes crossings without replay on unmute.
- Invalid settings values preserve a recovery copy and disable providers. New profiles remain disabled during first-run or settings recovery until the user enables them.
- Window geometry clamps provider strips and cards to each edge. Browser fixture checks cover one/twelve providers, extreme offsets, and 50%/200% scale. These checks are not native screenshot qualification.
- Native Windows regions limit interaction to visible Notch controls. Real desktop click-through and mixed-DPI behavior still require validation.
- The Mac development DMG installs `Notch Preview.app` separately from the stable product.

## Remaining launch work

The Gemini budget/log slice passed both native lanes in the `553063d` run above. It adds an optional personal monthly token budget, local calendar month/day totals with per-source rows matching the Mac window ids, streamed CLI logs, and read-only OpenCode/Hermes queries. The settings save path was checked in a browser fixture: blank saves null, and zero, fractional or oversized budgets are rejected. See [Claude handoff](claude-handoff.md) for verification limits and continuation steps.

- Complete provider account metadata, credential-source coverage, Gemini budgets, provider-specific activity, and live-account validation.
- Complete profile session and hook isolation, all-display/visibility modes, focus behavior, provider duration coverage, and matching native settings and onboarding.
- Review both native interfaces against shared design tokens and capture separate screenshot baselines.
- Exercise signed updater consent, invalid signatures/platforms, offline recovery, cross-version installs, and publication/deployment failure scenarios.
- Qualify both platforms and configure owner signing/notarization before approving a public release. Every gate in `release/qualification.json` remains pending until that evidence exists.
