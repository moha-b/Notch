# Source provenance

Notch is an independent product derived from two MIT-licensed projects.

- Windows history: Im-Midi/codenotch-windows at `481aafa`, plus the installer contribution through `2444ca14a1d0555c5db409bbca083fd247f043fb`. The original contribution branch remains unchanged.
- Native macOS snapshot: vinzdg/codenotch at `6f0eb970201d895627e9c8ed9792d7befa27f809`. Its Windows copy, hosted site, workflows, and generated releases were excluded. The source import is recorded as its own commit.
- The initial local repository remains on `local-starter`.

Upstream licenses are retained in each native app. Future upstream security and crash fixes require reviewed patches and regression tests. The feature baseline does not automatically follow upstream main.

Notch uses its own product identity and updater trust. It does not claim the original authors' signing identities, transfer their installed users, or import their application settings automatically.
