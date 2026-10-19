# Maintainer: MoonHunt Contributors
pkgname=moonhunt
pkgver=0.0.6
pkgrel=1
pkgdesc="A download manager powered by aria2 and yt-dlp"
arch=('x86_64')
url="https://github.com/2sulraw/MoonHunt-DownloadManager"
license=('GPL-3.0-only')
depends=(
  'webkit2gtk-4.1'
  'libsoup3'
  'gtk3'
  'libappindicator-gtk3'
  'librsvg'
  'hicolor-icon-theme'
  'gcc-libs'
  'glibc'
  'aria2'
)
makedepends=(
  'cargo'
  'nodejs'
  'npm'
  'patchelf'
  'pkgconf'
  'base-devel'
)
optdepends=(
  'yt-dlp: media / YouTube downloads'
  'ffmpeg: merge separate video+audio streams'
)
install=moonhunt.install
# The bundled sqlite3 C code (rusqlite `bundled` feature) is compiled with
# makepkg CFLAGS; default `lto` injects -flto=auto, producing GCC LTO
# bytecode objects that rust-lld cannot link. Disable LTO for this package.
options=('!lto')
source=("$pkgname-$pkgver.tar.gz::https://github.com/2sulraw/MoonHunt-DownloadManager/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
  cd "$srcdir/MoonHunt-DownloadManager-$pkgver"

  # Install npm dependencies
  npm ci --prefer-offline || npm install

  # Ensure the Linux aria2c sidecar exists
  mkdir -p src-tauri/binaries
  if [ ! -f "src-tauri/binaries/aria2c-x86_64-unknown-linux-gnu" ]; then
    cp "$(which aria2c)" "src-tauri/binaries/aria2c-x86_64-unknown-linux-gnu"
    chmod +x "src-tauri/binaries/aria2c-x86_64-unknown-linux-gnu"
  fi
}

build() {
  cd "$srcdir/MoonHunt-DownloadManager-$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  export npm_config_cache="$srcdir/npm-cache"
  export CARGO_HOME="$srcdir/cargo-home"
  # Build only the binary — the pacman package below installs
  # the binary, .desktop file and icons manually.
  # (deb/rpm bundles are useless inside a pacman package.)
  npm run tauri -- build --no-bundle --ci
}

package() {
  cd "$srcdir/MoonHunt-DownloadManager-$pkgver"

  # Binaries
  install -Dm755 "src-tauri/target/release/moonhunt" "$pkgdir/usr/bin/moonhunt"
  install -Dm755 "src-tauri/binaries/aria2c-x86_64-unknown-linux-gnu" \
    "$pkgdir/usr/lib/moonhunt/aria2c"

  # Desktop file (Tauri generates only a minimal one inside deb/rpm
  # bundles, which we no longer build — so ship our own)
  install -Dm644 /dev/stdin "$pkgdir/usr/share/applications/moonhunt.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Version=1.0
Name=MoonHunt
GenericName=Download Manager
Comment=A download manager powered by aria2 and yt-dlp
Exec=/usr/bin/moonhunt %U
Icon=moonhunt
Terminal=false
Categories=Network;FileTransfer;Utility;
StartupWMClass=moonhunt
MimeType=x-scheme-handler/moonhunt;
EOF

  # Icons
  install -Dm644 "src-tauri/icons/32x32.png" \
    "$pkgdir/usr/share/icons/hicolor/32x32/apps/moonhunt.png"
  install -Dm644 "src-tauri/icons/128x128.png" \
    "$pkgdir/usr/share/icons/hicolor/128x128/apps/moonhunt.png"
  install -Dm644 "src-tauri/icons/128x128@2x.png" \
    "$pkgdir/usr/share/icons/hicolor/256x256/apps/moonhunt.png"

  # License
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
