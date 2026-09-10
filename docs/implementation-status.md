# Implementation and qualification status

Notch remains a development product. Passing build lanes do not establish feature parity or authorize publication.

## Verified milestones

- Native tests, Windows installer lifecycle and Mac development DMG passed for `496e92d` in [candidate run 34478073633](https://github.com/moha-b/Notch/actions/runs/34478073633).
- Named Windows Claude and Codex profiles use separate credentials and cached readings. Tests cover credential separation and profile-specific stale rollout fallback. Profile session activity and hooks still need parity work.
- The GitHub `production` environment requires `moha-b` review and permits only the `main` branch. Apple and updater signing credentials remain owner setup requirements.
- Release preparation rejects existing versions and tags. Publication creates its source tag atomically; a failed feed publication can resume only after verifying the same public build. This recovery path has policy tests; a complete signed publication exercise remains pending.

## Latest development changes

- Shared palette and design scale generate native Swift constants and CSS. Windows usage pace follows the Mac calculation and the same JSON behavior fixtures; its settings toggle is verified in a browser fixture. Native pace tests are pending the next candidate run.
- Windows carries known Claude/Codex/GLM durations and reported Cursor/Grok billing intervals. OpenCode monthly cycles use calendar arithmetic; Copilot pace requires its first-of-month UTC boundary. Unknown periods have no pace. Antigravity and Gemini duration coverage remains part of provider parity work.
- Codex rollout relative resets use the recorded timestamp, preventing stale logs from restarting their reset timer when reread.
- Session transitions drive peeks and optional Windows system sounds. Threshold alerts trigger at 80% and 100%; mute consumes crossings without replay on unmute.
- Invalid settings values preserve a recovery copy and disable providers. New profiles remain disabled during first-run or settings recovery until the user enables them.
- Window geometry clamps provider strips and cards to each edge. Browser fixture checks cover one/twelve providers, extreme offsets, and 50%/200% scale. These checks are not native screenshot qualification.
- Native Windows regions limit interaction to visible Notch controls. Real desktop click-through and mixed-DPI behavior still require validation.
- The Mac development DMG installs `Notch Preview.app` separately from the stable product.

## Remaining launch work

- Complete provider account metadata, credential-source coverage, Gemini budgets, provider-specific activity, and live-account validation.
- Complete profile session and hook isolation, all-display/visibility modes, focus behavior, provider duration coverage, and matching native settings and onboarding.
- Review both native interfaces against shared design tokens and capture separate screenshot baselines.
- Exercise signed updater consent, invalid signatures/platforms, offline recovery, cross-version installs, and publication/deployment failure scenarios.
- Qualify both platforms and configure owner signing/notarization before approving a public release. Every gate in `release/qualification.json` remains pending until that evidence exists.
