#!/usr/bin/env bash
set -euo pipefail
: "${APPLE_CERTIFICATE_BASE64:?}" "${APPLE_CERTIFICATE_PASSWORD:?}" "${APPLE_TEAM_ID:?}"
: "${APPLE_ID:?}" "${APPLE_PASSWORD:?}" "${SPARKLE_PRIVATE_KEY:?}"
ROOT="$(pwd)"
RELEASE_DIR="$ROOT/work/release"
KEYCHAIN="$RUNNER_TEMP/notch-signing.keychain-db"
CERTIFICATE="$RUNNER_TEMP/notch-certificate.p12"
KEYCHAIN_PASSWORD="$(openssl rand -hex 24)"
trap 'security delete-keychain "$KEYCHAIN" >/dev/null 2>&1 || true; rm -f "$CERTIFICATE" "$RUNNER_TEMP/notch-sparkle.key"' EXIT
printf '%s' "$APPLE_CERTIFICATE_BASE64" | base64 --decode > "$CERTIFICATE"
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 21600 "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security import "$CERTIFICATE" -P "$APPLE_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$KEYCHAIN"
security set-key-partition-list -S apple-tool:,apple: -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
security list-keychain -d user -s "$KEYCHAIN"
cd apps/macos
xcodegen generate
xcodebuild -project Notch.xcodeproj -scheme Notch -destination 'platform=macOS,arch=arm64' \
  -configuration Release -derivedDataPath build/Release \
  CODE_SIGN_IDENTITY='Developer ID Application' DEVELOPMENT_TEAM="$APPLE_TEAM_ID" \
  OTHER_CODE_SIGN_FLAGS='--timestamp' build
APP="$PWD/build/Release/Build/Products/Release/Notch.app"
codesign --verify --deep --strict "$APP"
ditto -c -k --keepParent "$APP" "$RELEASE_DIR/notarize.zip"
xcrun notarytool submit "$RELEASE_DIR/notarize.zip" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
xcrun stapler staple "$APP"
spctl --assess --type execute "$APP"
mkdir -p "$RELEASE_DIR/stage"
cp -R "$APP" "$RELEASE_DIR/stage/"
ln -s /Applications "$RELEASE_DIR/stage/Applications"
VERSION="$(node -p "JSON.parse(require('fs').readFileSync('$RELEASE_DIR/release-manifest.json','utf8')).version")"
DMG="$RELEASE_DIR/Notch-$VERSION-macos-arm64.dmg"
hdiutil create -volname Notch -srcfolder "$RELEASE_DIR/stage" -format UDZO "$DMG"
codesign --sign 'Developer ID Application' --timestamp "$DMG"
xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
xcrun stapler staple "$DMG"
xcrun stapler validate "$DMG"
SIGN_UPDATE="$(find build/Release/SourcePackages/artifacts -type f -name sign_update | head -1)"
test -n "$SIGN_UPDATE"
umask 077
printf '%s' "$SPARKLE_PRIVATE_KEY" > "$RUNNER_TEMP/notch-sparkle.key"
"$SIGN_UPDATE" --ed-key-file "$RUNNER_TEMP/notch-sparkle.key" "$DMG" > "$DMG.sig"
