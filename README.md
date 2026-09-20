# MoonHunt

![Total Downloads](https://img.shields.io/github/downloads/2sulraw/MoonHunt-DownloadManager/total)
![GitHub Release](https://img.shields.io/github/v/release/2sulraw/MoonHunt-DownloadManager)

A desktop download manager built with [Tauri 2](https://v2.tauri.app/),
[Vue 3](https://vuejs.org/), and [aria2](https://aria2.github.io/).

MoonHunt runs your downloads through aria2 and handles media via
[yt-dlp](https://github.com/yt-dlp/yt-dlp).

![MoonHunt screenshot](screenshots/screenshot.png)

> **Note — early build:** this app is in an early stage, so there may be bugs.
> If you encounter one, please [open an issue](https://github.com/2sulraw/MoonHunt-DownloadManager/issues)
> and report it.

## Features

- **Media downloads** — videos and audio via yt-dlp, with format selection,
  JS-runtime solving, and optional FFmpeg merging
- **Torrent support** — add and manage `.torrent` downloads via aria2
- **Proxy support** — HTTP and SOCKS5 proxies with a live health check
- **Performance** — low usage of memory, CPU, disk

## Install

Download the latest installer from the
[Releases](https://github.com/2sulraw/MoonHunt-DownloadManager/releases) page.
aria2 is bundled — no extra setup required.

**Windows:** NSIS installer (`.exe`) or `.msi`.

**Linux:** `.deb` (Debian/Ubuntu), `.rpm` (Fedora/RHEL), or `.AppImage`
(universal).

**Arch Linux** — clone and install with `makepkg`:

```bash
git clone https://github.com/2sulraw/MoonHunt-DownloadManager.git
cd MoonHunt-DownloadManager
makepkg -si
```

This installs the binary to `/usr/bin/moonhunt` and all system dependencies.
After installation, launch from your app menu or run `moonhunt` in a terminal.

Or download the `.AppImage` from
[Releases](https://github.com/2sulraw/MoonHunt-DownloadManager/releases):

```bash
sudo pacman -S fuse2
chmod +x moonhunt_*.AppImage
./moonhunt_*.AppImage
```

Or build from source with the helper script:

```bash
./build-linux.sh
```

The in-app updater checks GitHub releases and will download the appropriate
Linux package for your system.

## Browser Extension

A companion browser extension (Chromium + Firefox) is available at
[MoonHunt-Extension](https://github.com/2sulraw/MoonHunt-Extension).

**Firefox:** install directly from
[Firefox Add-ons (AMO)](https://addons.mozilla.org/en-US/firefox/addon/moonhunt-download-manager/).

The Chromium version is available from the extension repo above.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) toolchain
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

**Windows:** WebView2 (ships with Windows 10/11).

**Linux** — system packages by distro:

| Distro | Packages |
|--------|----------|
| Arch / Manjaro | `webkit2gtk-4.1 libsoup3 gtk3 libappindicator-gtk3 librsvg patchelf base-devel` |
| Debian / Ubuntu / Mint / Pop | `libwebkit2gtk-4.1-dev libsoup-3.0-dev libgtk-3-dev libappindicator3-dev librsvg2-dev patchelf build-essential` |
| Fedora / RHEL / CentOS | `webkit2gtk4.1 libsoup3 gtk3 libappindicator-gtk3 librsvg2 patchelf gcc make` |
| openSUSE | `webkit2gtk4.1-devel libsoup-3_0-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel patchelf` |
| Void Linux | `webkit2gtk-4.1 libsoup3 gtk+3 libappindicator librsvg patchelf base-devel` |

The `build-linux.sh` script auto-detects your distro and installs all required
dependencies.

### Run

```bash
npm install
npm run tauri dev
```

### Build

```bash
# Quick build (installs all deps automatically)
./build-linux.sh

# Or manually
npm install
npm run tauri build
```

Output installers land in `src-tauri/target/release/bundle/`:
- `.deb` — Debian/Ubuntu-based
- `.rpm` — Fedora/RHEL-based
- `.AppImage` — universal portable format

The build bundles `aria2c` as a Tauri sidecar from `src-tauri/binaries/`.

## Architecture

```
moonhunt/
├── src/                  # Vue 3 frontend
│   ├── components/       # reusable UI components
│   ├── composables/      # state + logic hooks
│   ├── layouts/          # app shell (sidebar, titlebar)
│   ├── router/           # Vue Router config
│   ├── types/            # TypeScript definitions
│   ├── utils/            # shared formatters/helpers
│   └── views/            # Home, Downloads, Media, Settings
├── src-tauri/            # Rust backend
│   └── src/
│       ├── aria2/        # aria2 RPC client + process lifecycle
│       ├── db/           # SQLite persistence
│       ├── extension/    # HTTP bridge for the browser extension
│       ├── ytdlp/        # yt-dlp integration
│       ├── update.rs     # GitHub-releases update check + installer
│       └── ...
└── public/               # static assets (tutorial gifs, changelog)
```

## License

Copyright © 2sulraw. All rights reserved.