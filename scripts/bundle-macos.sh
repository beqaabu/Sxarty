#!/usr/bin/env bash
#
# Builds Sxarty.app and a DMG.
#
#   scripts/bundle-macos.sh              universal binary, unsigned
#   scripts/bundle-macos.sh --host-only  skip the second architecture (faster)
#
# Signing and notarisation happen only when the credentials are in the
# environment, so the same script works on a laptop with none of them and in CI
# with all of them:
#
#   MACOS_SIGN_IDENTITY   "Developer ID Application: Name (TEAMID)"
#   MACOS_NOTARY_PROFILE  a profile stored with `xcrun notarytool store-credentials`
#
set -euo pipefail

cd "$(dirname "$0")/.."

BUNDLE_ID="io.github.beqaabu.sxarty"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
DIST="target/dist"
APP="$DIST/Sxarty.app"
DMG="$DIST/Sxarty-$VERSION.dmg"

HOST_ONLY=0
[ "${1:-}" = "--host-only" ] && HOST_ONLY=1

say() { printf '\033[1m==>\033[0m %s\n' "$1"; }

# ── Binary ──────────────────────────────────────────────────────────────────
if [ "$HOST_ONLY" -eq 1 ]; then
  say "Building for the host architecture only"
  cargo build --release
  BINARY="target/release/sxarty"
else
  say "Building a universal binary"
  for target in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$target" >/dev/null 2>&1 || true
    cargo build --release --target "$target"
  done
  mkdir -p "$DIST"
  lipo -create -output "$DIST/sxarty" \
    target/aarch64-apple-darwin/release/sxarty \
    target/x86_64-apple-darwin/release/sxarty
  BINARY="$DIST/sxarty"
fi

# ── Bundle ──────────────────────────────────────────────────────────────────
say "Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/sxarty"
chmod +x "$APP/Contents/MacOS/sxarty"
cp assets/icon/sxarty.icns "$APP/Contents/Resources/sxarty.icns"

sed -e "s/@VERSION@/$VERSION/g" -e "s/@BUNDLE_ID@/$BUNDLE_ID/g" \
  packaging/macos/Info.plist.in > "$APP/Contents/Info.plist"

# Finder caches bundle metadata aggressively; without this a rebuilt app keeps
# showing the old icon until you log out.
touch "$APP"

# ── Signing ─────────────────────────────────────────────────────────────────
if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
  say "Signing with $MACOS_SIGN_IDENTITY"
  codesign --force --deep --options runtime --timestamp \
    --entitlements packaging/macos/entitlements.plist \
    --sign "$MACOS_SIGN_IDENTITY" "$APP"
  codesign --verify --strict --verbose=2 "$APP"
else
  # An ad-hoc signature is not a Developer ID and will not clear Gatekeeper for
  # anyone else, but it does let the app run locally without being re-signed on
  # every launch, which unsigned arm64 binaries otherwise need.
  say "No MACOS_SIGN_IDENTITY set - signing ad hoc (local use only)"
  codesign --force --deep --sign - "$APP"
fi

# ── Disk image ──────────────────────────────────────────────────────────────
say "Building $DMG"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

rm -f "$DMG"
hdiutil create -volname "Sxarty $VERSION" -srcfolder "$STAGE" \
  -ov -format UDZO -quiet "$DMG"

if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
  codesign --force --sign "$MACOS_SIGN_IDENTITY" "$DMG"
fi

# ── Notarisation ────────────────────────────────────────────────────────────
if [ -n "${MACOS_NOTARY_PROFILE:-}" ]; then
  say "Notarising (this takes a few minutes)"
  xcrun notarytool submit "$DMG" --keychain-profile "$MACOS_NOTARY_PROFILE" --wait
  # Stapling lets the DMG verify without a network round trip on first launch.
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"
else
  say "No MACOS_NOTARY_PROFILE set - skipping notarisation"
  say "Gatekeeper will block this DMG on other people's machines."
fi

say "Done: $DMG"
