#!/bin/sh
# RootUI installer for Linux.   Run:  sh install.sh        (add --desktop for a Desktop shortcut too)
# Normal user: installs to ~/.local  (no password needed).   As root / sudo: installs for everybody (/opt).
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"

if [ ! -f "$HERE/rootui" ] || [ ! -d "$HERE/ui" ]; then
    echo "Run this script from the unzipped RootUI folder (it needs 'rootui' and 'ui' next to it)."
    exit 1
fi

if [ "$(id -u)" = "0" ]; then
    PREFIX=/opt/rootui; BIN=/usr/local/bin; APPS=/usr/share/applications; ICONDIR=/usr/share/icons/hicolor/256x256/apps
else
    PREFIX="$HOME/.local/share/rootui"; BIN="$HOME/.local/bin"
    APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"; ICONDIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
fi

echo "Installing RootUI to $PREFIX ..."
rm -rf "$PREFIX"
mkdir -p "$PREFIX" "$BIN" "$APPS" "$ICONDIR"
cp -R "$HERE/rootui" "$HERE/ui" "$PREFIX/"
chmod 755 "$PREFIX/rootui"
[ -f "$HERE/rootui.png" ] && cp "$HERE/rootui.png" "$ICONDIR/rootui.png"
ln -sf "$PREFIX/rootui" "$BIN/rootui"

# Entry in the applications menu ("program list")
cat > "$APPS/rootui.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=RootUI
Comment=Sci-fi terminal desktop
Exec="$PREFIX/rootui"
Path=$PREFIX
Icon=$ICONDIR/rootui.png
Terminal=false
Categories=System;TerminalEmulator;
Keywords=terminal;shell;monitor;
StartupNotify=false
DESK
chmod 644 "$APPS/rootui.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$(dirname "$(dirname "$ICONDIR")")" >/dev/null 2>&1 || true

# Optional Desktop shortcut
if [ "$1" = "--desktop" ]; then
    DESK_DIR="$(command -v xdg-user-dir >/dev/null 2>&1 && xdg-user-dir DESKTOP || echo "$HOME/Desktop")"
    if [ -d "$DESK_DIR" ]; then
        cp "$APPS/rootui.desktop" "$DESK_DIR/RootUI.desktop"
        chmod +x "$DESK_DIR/RootUI.desktop"
        command -v gio >/dev/null 2>&1 && gio set "$DESK_DIR/RootUI.desktop" metadata::trusted true >/dev/null 2>&1 || true
        echo "Desktop shortcut created."
    fi
fi

# Does this system have the libraries RootUI needs?
if command -v ldd >/dev/null 2>&1; then
    MISSING="$(ldd "$PREFIX/rootui" 2>/dev/null | grep 'not found' || true)"
    if [ -n "$MISSING" ]; then
        echo
        echo "RootUI is installed, but these libraries are missing on this system:"
        echo "$MISSING"
        echo
        echo "Debian / Ubuntu / Mint :  sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libxdo3"
        echo "Fedora                 :  sudo dnf install webkit2gtk4.1 gtk3 libxdo"
        echo "Arch / Manjaro         :  sudo pacman -S webkit2gtk-4.1 gtk3 xdotool"
    fi
fi

echo
echo "Done. Open your applications menu and search for \"RootUI\", or run:  rootui"
case ":$PATH:" in *":$BIN:"*) ;; *) echo "(Add $BIN to your PATH to use the 'rootui' command.)" ;; esac
