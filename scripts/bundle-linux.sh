#!/usr/bin/env bash
#
# Builds the Linux artefacts: a portable tarball, an AppImage, and a .deb.
#
#   scripts/bundle-linux.sh
#
# Anything whose tooling is missing is skipped with a note rather than failing,
# so this is usable on a workstation that only has some of it. AppImage needs
# `appimagetool` (downloaded automatically if absent); the .deb needs `dpkg-deb`.
#
set -euo pipefail

cd "$(dirname "$0")/.."

APP_ID="io.github.beqaabu.sxarty"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
ARCH="$(uname -m)"
DIST="target/dist"
ICON_SIZES="16 24 32 48 64 128 256 512"

say() { printf '\033[1m==>\033[0m %s\n' "$1"; }
note() { printf '    %s\n' "$1"; }

say "Building sxarty $VERSION for $ARCH"
cargo build --release
mkdir -p "$DIST"

# ── Shared tree ─────────────────────────────────────────────────────────────
# Laid out as it would be under /usr, so every format below is a copy of it.
ROOT="$DIST/usr-tree"
rm -rf "$ROOT"
install -Dm755 target/release/sxarty "$ROOT/usr/bin/sxarty"
install -Dm644 "packaging/linux/$APP_ID.desktop" "$ROOT/usr/share/applications/$APP_ID.desktop"
install -Dm644 "packaging/linux/$APP_ID.metainfo.xml" "$ROOT/usr/share/metainfo/$APP_ID.metainfo.xml"
install -Dm644 LICENSE "$ROOT/usr/share/licenses/sxarty/LICENSE"
install -Dm644 README.md "$ROOT/usr/share/doc/sxarty/README.md"

for size in $ICON_SIZES; do
  install -Dm644 "assets/icon/sxarty-$size.png" \
    "$ROOT/usr/share/icons/hicolor/${size}x${size}/apps/$APP_ID.png"
done
install -Dm644 assets/icon/sxarty.svg \
  "$ROOT/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"

# ── Portable tarball ────────────────────────────────────────────────────────
say "Tarball"
TAR="$DIST/sxarty-$VERSION-$ARCH-linux.tar.gz"
tar -czf "$TAR" -C "$ROOT" usr
note "$TAR"

# ── AppImage ────────────────────────────────────────────────────────────────
say "AppImage"
APPDIR="$DIST/Sxarty.AppDir"
rm -rf "$APPDIR"
cp -r "$ROOT" "$APPDIR"

# An AppImage expects the desktop file, icon and AppRun at the AppDir root.
cp "$APPDIR/usr/share/applications/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
cp "assets/icon/sxarty-256.png" "$APPDIR/$APP_ID.png"
cp "assets/icon/sxarty.svg" "$APPDIR/.DirIcon.svg" 2>/dev/null || true
cp "assets/icon/sxarty-256.png" "$APPDIR/.DirIcon"

cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="$HERE/usr/bin:$PATH"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$HERE/usr/bin/sxarty" "$@"
RUN
chmod +x "$APPDIR/AppRun"

TOOL="$(command -v appimagetool || true)"
if [ -z "$TOOL" ]; then
  CACHED="$DIST/appimagetool-$ARCH.AppImage"
  if [ ! -x "$CACHED" ]; then
    note "Fetching appimagetool"
    curl -fsSL -o "$CACHED" \
      "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage" || true
    chmod +x "$CACHED" 2>/dev/null || true
  fi
  [ -x "$CACHED" ] && TOOL="$CACHED"
fi

if [ -n "$TOOL" ]; then
  # ARCH is what appimagetool names the output with; it does not detect it from
  # the AppDir.
  ARCH="$ARCH" "$TOOL" "$APPDIR" "$DIST/Sxarty-$VERSION-$ARCH.AppImage"
  note "$DIST/Sxarty-$VERSION-$ARCH.AppImage"
else
  note "appimagetool unavailable - skipped"
fi

# ── Debian package ──────────────────────────────────────────────────────────
say "Debian package"
if command -v dpkg-deb >/dev/null 2>&1; then
  case "$ARCH" in
    x86_64)  DEB_ARCH=amd64 ;;
    aarch64) DEB_ARCH=arm64 ;;
    *)       DEB_ARCH="$ARCH" ;;
  esac

  DEBDIR="$DIST/deb"
  rm -rf "$DEBDIR"
  cp -r "$ROOT" "$DEBDIR"
  mkdir -p "$DEBDIR/DEBIAN"

  # Depends covers what a wgpu window needs at runtime beyond libc; the GPU
  # driver stack itself is pulled in by these.
  cat > "$DEBDIR/DEBIAN/control" <<CONTROL
Package: sxarty
Version: $VERSION
Section: text
Priority: optional
Architecture: $DEB_ARCH
Maintainer: Beqa Abuladze <beqaabuladze.00@gmail.com>
Depends: libc6, libgcc-s1, libx11-6, libxkbcommon0, libwayland-client0, libfontconfig1
Homepage: https://github.com/beqaabu/Sxarty
Description: Speed reader that shows one word at a time
 Sxarty presents a document one word at a time, held at the point the eye
 already looks for, so reading a line costs no eye movement.
 .
 A context panel shows the surrounding text and any word in it can be
 clicked to jump there, so a missed clause is recoverable.
CONTROL

  dpkg-deb --build --root-owner-group "$DEBDIR" "$DIST/sxarty_${VERSION}_${DEB_ARCH}.deb" >/dev/null
  note "$DIST/sxarty_${VERSION}_${DEB_ARCH}.deb"
else
  note "dpkg-deb unavailable - skipped"
fi

say "Done"
