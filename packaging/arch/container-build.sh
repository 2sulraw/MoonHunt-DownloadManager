#!/usr/bin/env bash
set -euo pipefail

pacman -Syu --noconfirm
pacman -S --noconfirm --needed \
  base-devel rust pkgconf openssl xz libx11 libxkbcommon libxkbcommon-x11 \
  libxcursor libxrandr libxi libxinerama mesa wayland fontconfig \
  aria2 hicolor-icon-theme sudo

useradd -m builder 2>/dev/null || true
echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers

rm -rf /home/builder/pkg
mkdir -p /home/builder/pkg
cp /pkg/PKGBUILD /pkg/moonhunt.install /out/moonhunt-0.0.7-source.tar.gz /home/builder/pkg/
chown -R builder:builder /home/builder/pkg

su builder -c 'cd /home/builder/pkg && makepkg -s --noconfirm'

cp /home/builder/pkg/moonhunt-0.0.7-1-x86_64.pkg.tar.zst /out/
ls -l /out/moonhunt-0.0.7-1-x86_64.pkg.tar.zst
