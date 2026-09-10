# Notch release operations

Main merges run native validation and prepare SHA-named development installers. They do not publish a stable release. Development builds have separate bundle IDs, storage, hook ports and startup entries; the public updater is disabled.

## Qualification

Do not publish until both platforms satisfy every gate in `release/qualification.json`. Record the tested source SHA and attach the provider, screenshot, installer, upgrade and update-consent evidence to the release review. Update the qualification report in a later documentation commit. This avoids requiring a commit to contain its own hash. A passed report for another source is rejected.

The release workflow accepts a full SHA from main history and a stable X.Y.Z version. The selected SHA must have successful Mac and Windows candidate jobs on main. The first release is 1.0.0. Choose the next patch for routine fixes; recovery is a new, higher version containing the fix or revert. Public assets are never overwritten. A failed draft reserves its version until an owner explicitly inspects and removes the abandoned draft and tag.

## Owner setup

Use GitHub repository variables for `NOTCH_TAURI_PUBLIC_KEY`, `NOTCH_SPARKLE_PUBLIC_KEY`, and `APPLE_TEAM_ID`. Create independent Notch updater keys using the Tauri signer and Sparkle tools. Keep offline backups of the private keys; losing them breaks upgrades for existing installations.

Use GitHub Actions secrets for `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `SPARKLE_PRIVATE_KEY`, `APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, and `APPLE_PASSWORD` (an Apple app-specific password). The certificate must be the owner's Developer ID Application certificate exported with its private key. Never copy upstream developer identities or keys. The Mac signing job uses an ephemeral keychain and deletes its temporary credentials on exit.

Configure the `production` environment with the product owner's required review, restrict release workflow execution to trusted maintainers, and configure GitHub Pages to deploy from Actions. The Windows executable is initially Authenticode-unsigned; its updater signature is still mandatory.

## Publish and recover

Manually run **Publish Notch** with the qualified source and version. One generated manifest stamps both apps. Both are rebuilt and tested with final versions. Windows setup and Mac DMG, update signatures, checksums and release notes are uploaded to a draft. The production environment requires owner approval before publication.

Only after both signed packages are public are the Sparkle and Tauri feeds generated and deployed together as one Pages artifact. A signing, packaging, publication or deployment failure leaves the previous deployed feeds in place. If assets became public but Pages failed, rerun the failed feed deployment; do not rebuild or replace public assets. If a signed package itself is wrong, qualify and publish a higher version.

Installed apps check at launch and daily when checks are enabled. They require Update before downloading and restart confirmation before installation. Development builds never contact the stable update feed.
