#!/usr/bin/env bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

apt-get update -qq
apt-get install -y -qq \
  build-essential pkg-config libssl-dev libgtk-3-dev \
  libxkbcommon-dev libwayland-dev libx11-dev libxi-dev libgl1-mesa-dev \
  libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxcb-randr0-dev libxcb-xkb-dev libxkbcommon-x11-dev \
  libfontconfig1-dev libvulkan-dev libxcursor-dev libxrandr-dev libxinerama-dev \
  curl ca-certificates

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"

rm -rf /work && mkdir -p /work
cp -a /src/Cargo.toml /src/Cargo.lock /src/app /src/crates /work/
cd /work
cargo build --release -p moonhunt

cp target/release/moonhunt /out/moonhunt-linux-jammy
echo '--- built ---'
ls -l /out/moonhunt-linux-jammy
objdump -T /out/moonhunt-linux-jammy | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -3
