#!/bin/sh
# RootUI 1.0V - build the Linux app.
#
#   Usage:   sh BuildForLinux.sh              build the package into dist/
#            sh BuildForLinux.sh --install    build, then install it for your user (menu entry)
#
# Installs the missing build tools by itself (Debian/Ubuntu, Fedora, Arch) and Rust (rustup) if needed.
# Result: dist/RootUI-1.0V-linux-<arch>.zip   (rootui + ui + install.sh / uninstall.sh)
set -e
cd "$(dirname "$0")"
VERSION="1.0V"
DO_INSTALL=0
[ "$1" = "--install" ] && DO_INSTALL=1

say() { printf '%s\n' "[RootUI] $*"; }
die() { printf '%s\n' "[RootUI] $*" >&2; exit 1; }

[ "$(uname -s)" = "Linux" ] || die "This script builds the Linux app and must be run on Linux."

# Never build as a half-finished tree
[ -f Cargo.toml ] && [ -f ui/ui.html ] || die "Run this from the RootUI folder (Cargo.toml and ui/ must be next to it)."

SUDO=""
if [ "$(id -u)" != "0" ]; then
    command -v sudo >/dev/null 2>&1 && SUDO="sudo"
fi

[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

# ---------------------------------------------------------------- 1) system packages
need_pkgs() {
    command -v cc >/dev/null 2>&1 || return 0
    command -v pkg-config >/dev/null 2>&1 || return 0
    command -v curl >/dev/null 2>&1 || return 0
    command -v zip >/dev/null 2>&1 || return 0
    pkg-config --exists webkit2gtk-4.1 gtk+-3.0 || return 0
    [ -f /usr/include/xdo.h ] || [ -f /usr/local/include/xdo.h ] || return 0
    return 1
}

if need_pkgs; then
    say "Installing the Linux build tools (may ask for your password)..."
    if [ "$(id -u)" != "0" ] && [ -z "$SUDO" ]; then
        die "sudo is not available. Run this script as root once, or install the build tools yourself."
    fi
    if command -v apt-get >/dev/null 2>&1; then
        $SUDO apt-get update
        $SUDO apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev zip curl
    elif command -v dnf >/dev/null 2>&1; then
        $SUDO dnf install -y gcc gcc-c++ make pkgconf-pkg-config webkit2gtk4.1-devel gtk3-devel libxdo-devel openssl-devel zip curl
    elif command -v pacman >/dev/null 2>&1; then
        $SUDO pacman -S --needed --noconfirm base-devel pkgconf webkit2gtk-4.1 gtk3 xdotool openssl zip curl
    else
        die "Unknown Linux distribution. Install: gcc, pkg-config, curl, zip, and the dev packages of webkit2gtk-4.1, gtk3 and libxdo."
    fi
    need_pkgs && die "The build tools are still incomplete after the install. Check the messages above."
fi

# ---------------------------------------------------------------- 2) Rust
if ! command -v cargo >/dev/null 2>&1; then
    say "Installing Rust (rustup)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
fi
command -v cargo >/dev/null 2>&1 || die "Rust (cargo) is not available. Install it from https://rustup.rs"

# ---------------------------------------------------------------- 3) build
say "Building RootUI $VERSION (the first build takes a few minutes)..."
cargo build --release

TARGET_DIR="${CARGO_TARGET_DIR:-target}"
BIN="$TARGET_DIR/release/rootui"
[ -f "$BIN" ] || die "Build finished but $BIN was not found."

# ---------------------------------------------------------------- 4) package
ARCH="$(uname -m | sed 's/amd64/x86_64/;s/arm64/aarch64/')"
NAME="RootUI-$VERSION-linux-$ARCH"
OUT="dist/$NAME"
rm -rf "$OUT" "dist/$NAME.zip" "dist/$NAME.tar.gz"
mkdir -p "$OUT"
cp "$BIN" "$OUT/rootui"
cp -R ui "$OUT/ui"
cp packaging/assets/icon_256.png "$OUT/rootui.png"
cp packaging/linux/install.sh packaging/linux/uninstall.sh packaging/linux/README.txt "$OUT/"
chmod 755 "$OUT/rootui" "$OUT/install.sh" "$OUT/uninstall.sh"

if command -v zip >/dev/null 2>&1; then
    (cd dist && zip -qry "$NAME.zip" "$NAME")
    PKG="dist/$NAME.zip"
else
    tar -czf "dist/$NAME.tar.gz" -C dist "$NAME"
    PKG="dist/$NAME.tar.gz"
fi

# every library the app links to must exist on this machine
if command -v ldd >/dev/null 2>&1 && ldd "$OUT/rootui" 2>/dev/null | grep -q 'not found'; then
    say "Warning: missing libraries on this system:"
    ldd "$OUT/rootui" | grep 'not found'
fi

say "Done:  $PKG"
say "Test it:  ./$OUT/rootui        Install it:  sh $OUT/install.sh --desktop"

if [ "$DO_INSTALL" = "1" ]; then
    say "Installing for the current user..."
    sh "$OUT/install.sh"
fi
