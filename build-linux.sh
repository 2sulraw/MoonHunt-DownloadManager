#!/bin/bash
set -e

echo "=== MoonHunt Linux Build ==="

# ─── Detect Linux distribution ─────────────────────────────────────
detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "${ID:-unknown}"
    else
        echo "unknown"
    fi
}

DISTRO=$(detect_distro)
echo "Detected distro: $DISTRO"

# ─── Install Rust if missing ───────────────────────────────────────
if ! command -v rustc &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# ─── Install Node.js if missing ───────────────────────────────────
if ! command -v node &>/dev/null; then
    echo "Installing Node.js..."
    case "$DISTRO" in
        arch|manjaro)
            sudo pacman -Sy --noconfirm nodejs npm
            ;;
        ubuntu|debian|linuxmint|pop)
            sudo apt-get update -qq
            sudo apt-get install -y -qq nodejs npm
            ;;
        fedora)
            sudo dnf install -y nodejs npm
            ;;
        opensuse*|sles)
            sudo zypper install -y nodejs npm
            ;;
        void)
            sudo xbps-install -Sy nodejs npm
            ;;
        *)
            echo "WARNING: Could not install Node.js automatically for distro '$DISTRO'."
            echo "Please install Node.js manually: https://nodejs.org/"
            ;;
    esac
fi

# ─── Install Tauri system dependencies ─────────────────────────────
echo "Checking system dependencies..."
case "$DISTRO" in
    arch|manjaro)
        sudo pacman -Sy --noconfirm \
            webkit2gtk-4.1 libsoup3 gtk3 \
            libappindicator-gtk3 librsvg patchelf base-devel
        ;;
    ubuntu|debian|linuxmint|pop)
        sudo apt-get update -qq
        sudo apt-get install -y -qq \
            libwebkit2gtk-4.1-dev libsoup-3.0-dev \
            libgtk-3-dev libappindicator3-dev librsvg2-dev \
            patchelf build-essential curl
        ;;
    fedora)
        sudo dnf install -y \
            webkit2gtk4.1 libsoup3 gtk3 \
            libappindicator-gtk3 librsvg2 patchelf \
            gcc make
        ;;
    opensuse*|sles)
        sudo zypper install -y \
            webkit2gtk4.1-devel libsoup-3_0-devel gtk3-devel \
            libappindicator-gtk3-devel librsvg2-devel patchelf
        ;;
    void)
        sudo xbps-install -Sy \
            webkit2gtk-4.1 libsoup3 gtk+3 \
            libappindicator librsvg patchelf base-devel
        ;;
    *)
        echo "WARNING: Unknown distro '$DISTRO'. Skipping system dependency installation."
        echo "You may need to install webkit2gtk-4.1, libsoup3, gtk3, and build tools manually."
        ;;
esac

echo "Rust: $(rustc --version)"
echo "Node: $(node --version)"

# ─── Install npm dependencies ─────────────────────────────────────
echo "=== Installing npm dependencies ==="
npm install

# ─── Install cargo-tauri if missing ────────────────────────────────
if ! command -v cargo-tauri &>/dev/null; then
    echo "Installing cargo-tauri..."
    cargo install tauri-cli
fi

# ─── Download Linux aria2c binary if missing ───────────────────────
BINARIES_DIR="src-tauri/binaries"
LINUX_ARIA2C="$BINARIES_DIR/aria2c-x86_64-unknown-linux-gnu"
if [ ! -f "$LINUX_ARIA2C" ]; then
    echo "Setting up aria2c sidecar for Linux..."
    mkdir -p "$BINARIES_DIR"

    # Try to find system aria2c first, then install if needed
    if command -v aria2c &>/dev/null; then
        cp "$(which aria2c)" "$LINUX_ARIA2C"
        chmod +x "$LINUX_ARIA2C"
    else
        echo "aria2c not found. Installing..."
        case "$DISTRO" in
            arch|manjaro)
                sudo pacman -S --noconfirm aria2
                ;;
            ubuntu|debian|linuxmint|pop)
                sudo apt-get install -y -qq aria2
                ;;
            fedora)
                sudo dnf install -y aria2
                ;;
            opensuse*|sles)
                sudo zypper install -y aria2
                ;;
            void)
                sudo xbps-install -Sy aria2
                ;;
            *)
                echo "WARNING: Could not install aria2 automatically."
                echo "Please install aria2 manually, then re-run this script."
                exit 1
                ;;
        esac
        cp "$(which aria2c)" "$LINUX_ARIA2C"
        chmod +x "$LINUX_ARIA2C"
    fi
    echo "aria2c sidecar ready."
fi

# ─── Build ─────────────────────────────────────────────────────────
echo "=== Building MoonHunt for Linux ==="
cargo tauri build

echo ""
echo "=== Build complete! ==="
echo "Output: src-tauri/target/release/bundle/"
echo "  .deb — for Debian/Ubuntu-based distros"
echo "  .rpm — for Fedora/RHEL-based distros"
echo "  .AppImage — universal portable format"
