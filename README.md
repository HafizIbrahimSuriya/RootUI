# RootUI

A sci-fi terminal emulator, file browser and system monitor — **rebuilt in Rust**.
RootUI is a port of [eDEX-UI](https://github.com/GitSquared/edex-ui) (GPL-3.0): the original
interface (themes, keyboard, globe, graphs, audio, file browser, tabs …) running on a Rust
back-end instead of Electron + Node.js.

## Run it (no .exe needed)

1. Install Rust once: <https://rustup.rs> (Windows: keep the default *MSVC* toolchain; it will ask for the
   Visual Studio C++ Build Tools — accept).
2. Windows 10/11 needs the *WebView2 runtime* (already installed on Windows 11 and most Windows 10).
3. Double-click **`run-rootui.bat`**. (Linux: `./run-rootui.sh`.)

The first run compiles the dependencies (a few minutes); after that it starts immediately.
Extra arguments are passed through, e.g. `run-rootui.bat --nointro`.

## Build the packages for your users (version 1.0V)

| Users on | Run | You get (in `dist\`) |
|----------|-----|-----------------------|
| Windows 64-bit + 32-bit | **`BuildFORwindows.bat`** | `windows-x64\RootUI.exe`, `windows-x86\RootUI.exe` and the setup files `installers\RootUI-1.0V-Setup-x64.exe` / `RootUI-1.0V-Setup-x86.exe` |
| Linux | **`sh BuildForLinux.sh`** | `RootUI-1.0V-linux-*.zip` - contains `install.sh` (adds RootUI to the applications menu, `--desktop` for a Desktop icon) |

- The Windows setup files are made by Inno Setup 6 (`packaging\RootUI-x64.iss` and `packaging\RootUI-x86.iss`) - install it once
  from <https://jrsoftware.org/isdl.php>. The 32-bit build needs the x86 MSVC build tools (Visual Studio Installer).
- `BuildForLinux.sh` runs on Linux (Debian/Ubuntu, Fedora, Arch): it installs the missing build tools and Rust on the first run
  (asks for your password once). Add `--install` to also install the app for your user right after the build.
- There is no macOS build.
- `.github/workflows/build.yml` builds the same Linux and Windows packages on GitHub when you push.

## Theme maker

Gear icon (above terminal tab #1) -> **+ Create theme**. Pick colours and fonts, draw a 12x12 pixel icon, and watch the
live preview (tick *preview on the real app* to see it on the whole interface). **Save**, **Save & use**, **Export**
(writes `<name>.rootui-theme.json` to your Downloads folder and copies it to the clipboard) and **Import** (file or pasted text).
Custom themes get a pencil / export / delete button in the theme list and are stored in `themes/` next to the built-in ones.

## How it works

| eDEX-UI (Electron)                         | RootUI (Rust)                                                    |
|--------------------------------------------|------------------------------------------------------------------|
| Electron window / BrowserWindow            | `tao` window + `wry` webview (WebView2 / WebKit)                 |
| `node-pty` + `ws` terminal server          | `portable-pty` (ConPTY on Windows) + websocket (`axum`)          |
| `systeminformation`                        | `sysinfo` + small OS helpers (`src/si.rs`)                       |
| `geolite2-redist` / `maxmind`              | `maxminddb` with the same GeoLite2 DB (+ online fallback)        |
| Node `fs`, `os`, `path`, `require`         | `ui/shim.js` (talks to the local Rust server)                    |
| Electron IPC, `remote`, global shortcuts   | one websocket + in-page shortcut handling                        |

The Rust server listens only on `127.0.0.1` (random port) and requires a per-launch secret token, so
other programs/web pages cannot reach your terminal.

## Files

- `src/main.rs` — window, webview, event loop
- `src/server.rs` — local HTTP/WebSocket server, fs API, IPC
- `src/pty.rs` — shells / terminals (tabs), cwd + process tracking
- `src/si.rs` — CPU, RAM, processes, network, disks, battery
- `src/geo.rs` — GeoIP for the globe
- `src/config.rs` — settings bootstrap (same files as eDEX-UI)
- `ui/` — the web front-end (original eDEX-UI UI + `shim.js` + prebuilt `vendor/vendor.js`)

## Settings

Stored in `%APPDATA%\RootUI` (Linux: `~/.config/RootUI`): `settings.json`, `shortcuts.json`, `themes/`,
`keyboards/`, `fonts/`. Same format as eDEX-UI — open them from the in-app Settings dialog.

## What's new in this build

- Fixed the grey-teal terminal (the frame's border layer was painting over it); the terminal now uses the theme colours.
- PowerShell starts with plain Black/Gray colours and reports its folder after every prompt, so the file browser follows `cd` on Windows.
- File browser lists a folder in one request and no longer fails on a single unreadable entry.
- Fixed "OFFLINE" / missing globe (a `net.Socket` compatibility bug); connectivity check tries several endpoints; IP and geo lookups use several providers and don't wait for the GeoIP download.
- Clean L-shaped Enter key, uniform arrow keys, properly placed Close/modal buttons.
- Version 1.0V: in-app theme maker (see above), one-click installers for Windows (64-bit and 32-bit) / Linux.
- 13 new themes. Gear button opens a theme picker + social links (edit `ui/assets/misc/socials.json`); the Discord invite is read live from your gist.

## Known differences

- Terminal font ligatures are not available (the original used a Node-only add-on).
- On Windows the file browser does not follow `cd` automatically (same limitation as eDEX-UI).
- DevTools and the update check (it pointed at eDEX-UI's releases) are disabled.

## License

GPL-3.0 — see `LICENSE`. Based on eDEX-UI © Gabriel "Squared" Saillard and contributors.
