# Notch

One usage notch for macOS and Windows, built with native platform integrations.

**In development. No stable release has been published.** Development candidates are isolated from stable installations and do not receive public updates.

- macOS: Swift, AppKit and SwiftUI; Apple Silicon, macOS 15+.
- Windows: Rust, Tauri and WebView2; Windows 11 x64.
- Shared design tokens and behavioral fixtures keep both native interfaces aligned.
- Releases require explicit owner approval. Update checks notify; downloading and installing require user action.

The frozen feature target includes twelve provider families, multi-account profiles, screen-edge placement, multi-display behavior, activity, alerts and matching settings. See [provider feasibility](docs/provider-feasibility.md) and [source provenance](docs/provenance.md).

Public release is gated by feature parity, native validation, and Notch-owned signing/notarization credentials. Upstream credentials and signing identities are never reused.

## Source and credits

Derived from [vinzdg/codenotch](https://github.com/vinzdg/codenotch) and [Im-Midi/codenotch-windows](https://github.com/Im-Midi/codenotch-windows). Original MIT notices are retained in the native app trees. Notch is maintained independently.
