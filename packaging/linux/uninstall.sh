#!/bin/sh
# Removes RootUI (whatever install.sh put in place).   Run:  sh uninstall.sh
if [ "$(id -u)" = "0" ]; then
    rm -rf /opt/rootui /usr/local/bin/rootui /usr/share/applications/rootui.desktop /usr/share/icons/hicolor/256x256/apps/rootui.png
fi
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
rm -rf "$DATA/rootui" "$HOME/.local/bin/rootui" "$DATA/applications/rootui.desktop" "$DATA/icons/hicolor/256x256/apps/rootui.png"
rm -f "$HOME/Desktop/RootUI.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DATA/applications" >/dev/null 2>&1
echo "RootUI removed. Your settings and custom themes are still in ~/.config/RootUI (delete that folder to remove them too)."
