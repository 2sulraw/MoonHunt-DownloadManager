#!/usr/bin/env bash
set -euo pipefail

VER=0.0.7
BIN=/home/werlos/moonhunt-src/target/release/moonhunt
OUT=/mnt/d/MoonHunt-Rust/dist
OLD_ICONS=/mnt/d/MoonHunt/src-tauri/icons
ARIA=/mnt/d/MoonHunt/src-tauri/binaries/aria2c-x86_64-unknown-linux-gnu
STAGE=/tmp/moonhunt-stage
RPMTOP=/tmp/rpmbuild
APPDIR=/tmp/MoonHunt.AppDir

rm -rf "$STAGE" "$RPMTOP" "$APPDIR"
mkdir -p "$OUT"

# ---- shared install tree ----
mkdir -p "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/icons/hicolor/32x32/apps" \
         "$STAGE/usr/share/icons/hicolor/128x128/apps" \
         "$STAGE/usr/share/icons/hicolor/256x256/apps"
install -m 0755 "$BIN"  "$STAGE/usr/bin/moonhunt"
install -m 0755 "$ARIA" "$STAGE/usr/bin/aria2c"
install -m 0644 "$OLD_ICONS/32x32.png"     "$STAGE/usr/share/icons/hicolor/32x32/apps/moonhunt.png"
install -m 0644 "$OLD_ICONS/128x128.png"   "$STAGE/usr/share/icons/hicolor/128x128/apps/moonhunt.png"
install -m 0644 "$OLD_ICONS/128x128@2x.png" "$STAGE/usr/share/icons/hicolor/256x256/apps/moonhunt.png"

cat > "$STAGE/usr/share/applications/moonhunt.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=MoonHunt
Comment=MoonHunt download manager
Exec=moonhunt %U
Icon=moonhunt
Terminal=false
Categories=Network;FileTransfer;
MimeType=x-scheme-handler/moonhunt;
StartupWMClass=moonhunt
EOF

# ---- .deb ----
DEB=/tmp/moonhunt-deb
rm -rf "$DEB"
cp -a "$STAGE" "$DEB"
mkdir -p "$DEB/DEBIAN"
SIZE_KB=$(du -sk "$DEB" | cut -f1)
cat > "$DEB/DEBIAN/control" <<EOF
Package: moonhunt
Version: $VER
Architecture: amd64
Maintainer: werlos <werlos@users.noreply.github.com>
Installed-Size: $SIZE_KB
Section: net
Priority: optional
Depends: libaria2-0, libgtk-3-0, libx11-6, libxkbcommon0, libxkbcommon-x11-0, libxcursor1, libxrandr2, libxi6, libxinerama1, libgl1, libegl1, libwayland-client0, libwayland-cursor0, libwayland-egl1, libfontconfig1, libssl3, libbz2-1.0, libzstd1
Description: MoonHunt download manager
 MoonHunt is a desktop download manager with a browser-extension
 bridge, aria2 RPC backend, and a Spotify playlist add-on.
EOF
( cd "$DEB" && find . -type f ! -path './DEBIAN/*' -printf '%P\0' | xargs -0 md5sum > DEBIAN/md5sums )
dpkg-deb --root-owner-group --build "$DEB" "$OUT/moonhunt_${VER}_amd64.deb"

# ---- .rpm ----
mkdir -p "$RPMTOP/BUILD" "$RPMTOP/RPMS" "$RPMTOP/SOURCES" "$RPMTOP/SPECS" "$RPMTOP/SRPMS"
cat > "$RPMTOP/SPECS/moonhunt.spec" <<EOF
Name: moonhunt
Version: $VER
Release: 1
Summary: MoonHunt download manager
License: MIT
BuildArch: x86_64

%description
MoonHunt is a desktop download manager with a browser-extension
bridge, aria2 RPC backend, and a Spotify playlist add-on.

%install
mkdir -p %{buildroot}
cp -a "$STAGE/." %{buildroot}/

%files
/usr/bin/moonhunt
/usr/bin/aria2c
/usr/share/applications/moonhunt.desktop
/usr/share/icons/hicolor/32x32/apps/moonhunt.png
/usr/share/icons/hicolor/128x128/apps/moonhunt.png
/usr/share/icons/hicolor/256x256/apps/moonhunt.png

%changelog
* Thu Nov 26 2026 werlos - $VER-1
- MoonHunt $VER
EOF
rpmbuild -bb --define "_topdir $RPMTOP" --define "_build_id_links none" "$RPMTOP/SPECS/moonhunt.spec" > /tmp/rpmbuild.log 2>&1 || { tail -40 /tmp/rpmbuild.log; exit 1; }
cp "$RPMTOP/RPMS/x86_64/moonhunt-$VER-1.x86_64.rpm" "$OUT/"

# ---- AppImage ----
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/32x32/apps" \
         "$APPDIR/usr/share/icons/hicolor/128x128/apps" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"
install -m 0755 "$BIN" "$APPDIR/usr/bin/moonhunt"
install -m 0644 "$OLD_ICONS/32x32.png"     "$APPDIR/usr/share/icons/hicolor/32x32/apps/moonhunt.png"
install -m 0644 "$OLD_ICONS/128x128.png"   "$APPDIR/usr/share/icons/hicolor/128x128/apps/moonhunt.png"
install -m 0644 "$OLD_ICONS/128x128@2x.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/moonhunt.png"
cp "$STAGE/usr/share/applications/moonhunt.desktop" "$APPDIR/moonhunt.desktop"
cp "$STAGE/usr/share/applications/moonhunt.desktop" "$APPDIR/usr/share/applications/moonhunt.desktop"
cp "$OLD_ICONS/128x128@2x.png" "$APPDIR/moonhunt.png"
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/moonhunt" "$@"
EOF
chmod +x "$APPDIR/AppRun"
if [ ! -x /tmp/appimagetool ]; then
  curl -L --fail --silent --show-error -o /tmp/appimagetool \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x /tmp/appimagetool
fi
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 /tmp/appimagetool "$APPDIR" "$OUT/moonhunt_${VER}_amd64.AppImage"

echo '--- artifacts ---'
ls -l "$OUT/moonhunt_${VER}_amd64.deb" "$OUT/moonhunt-${VER}-1.x86_64.rpm" "$OUT/moonhunt_${VER}_amd64.AppImage"
