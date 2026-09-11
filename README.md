# Notch

One usage notch for macOS and Windows, built with native platform integrations.

**In development. No stable release has been published.** Development candidates are isolated from stable installations and do not receive public updates.

- macOS: Swift, AppKit and SwiftUI; Apple Silicon, macOS 15+.
- Windows: Rust, Tauri and WebView2; Windows 11 x64.
- Shared design tokens and behavioral fixtures keep both native interfaces aligned.
- Releases require explicit owner approval. Update checks notify; downloading and installing require user action.

The frozen feature target includes twelve provider families, multi-account profiles, screen-edge placement, multi-display behavior, activity, alerts and matching settings. See [provider parity](https://github.com/moha-b/Notch/issues/3) for the remaining coverage work.

See the [1.0.0 launch checklist](https://github.com/moha-b/Notch/issues/6) for verified milestones and remaining work, including [installer and update qualification](https://github.com/moha-b/Notch/issues/4) and [signing and public downloads](https://github.com/moha-b/Notch/issues/5). The root `docs/` folder contains local notes and is not tracked by Git.

Public release is gated by feature parity, native validation, and Notch-owned signing/notarization credentials. Upstream credentials and signing identities are never reused.

## Source and credits

Derived from [vinzdg/codenotch](https://github.com/vinzdg/codenotch) and [Im-Midi/codenotch-windows](https://github.com/Im-Midi/codenotch-windows). Original MIT notices are retained in the native app trees. Notch is maintained independently.

- Windows history starts from upstream `481aafa`, with installer contributions through `2444ca14a1d0555c5db409bbca083fd247f043fb`. The original contribution branch remains separate.
- The native macOS source is a snapshot of `vinzdg/codenotch` at `6f0eb970201d895627e9c8ed9792d7befa27f809`. Its duplicate Windows tree, hosted site, workflows, and generated releases were excluded from the import.
- The initial local repository is preserved on `local-starter`.

Upstream security and crash fixes require reviewed patches; new upstream features do not automatically expand the launch scope. Notch uses independent signing identities and update feeds, and does not silently migrate existing Codenotch installations or settings.
