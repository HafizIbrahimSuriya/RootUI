RootUI 1.0V for Linux
====================

1. Unzip the download.
2. Open a terminal in the unzipped folder and run:

       sh install.sh

   RootUI is added to your applications menu ("program list"). Add  --desktop  to also get a Desktop icon:

       sh install.sh --desktop

   (Run it with sudo to install for every user on the computer.)

RootUI needs WebKitGTK and GTK3, which most desktops already have. If install.sh reports missing libraries:
   Debian / Ubuntu / Mint :  sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libxdo3
   Fedora                 :  sudo dnf install webkit2gtk4.1 gtk3 libxdo
   Arch / Manjaro         :  sudo pacman -S webkit2gtk-4.1 gtk3 xdotool

Uninstall:  sh uninstall.sh
Your settings and custom themes live in  ~/.config/RootUI
