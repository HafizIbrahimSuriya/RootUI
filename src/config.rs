//! Settings bootstrap: mirrors what eDEX-UI's `_boot.js` did in the Electron main process.
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Version shown in the app ("RootUI 1.0V"). Cargo.toml needs a full 1.0.0 for itself.
pub const VERSION: &str = "1.0V";

pub fn user_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ROOTUI_CONFIG_DIR") {
        return PathBuf::from(p);
    }
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("RootUI")
}

/// Find the folder holding the web UI (ui.html, assets, classes ...).
/// Report a fatal startup error (message box on Windows, since the GUI build has no console) and exit.
pub fn fatal(msg: &str) -> ! {
    eprintln!("{msg}");
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let title: Vec<u16> = "RootUI".encode_utf16().chain(Some(0)).collect();
        let text: Vec<u16> = msg.encode_utf16().chain(Some(0)).collect();
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
    std::process::exit(1);
}

pub fn find_ui_dir() -> PathBuf {
    let mut cands: Vec<PathBuf> = vec![];
    if let Ok(p) = std::env::var("ROOTUI_UI_DIR") {
        cands.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            cands.push(d.join("ui"));
            cands.push(d.join("../Resources/ui")); // macOS: RootUI.app/Contents/MacOS/rootui -> Contents/Resources/ui
            cands.push(d.join("../../ui"));
            cands.push(d.join("../../../ui"));
        }
    }
    cands.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui"));
    cands.push(PathBuf::from("ui"));
    for c in cands {
        if c.join("ui.html").exists() {
            return c.canonicalize().unwrap_or(c);
        }
    }
    fatal("RootUI: cannot find the 'ui' folder (ui.html). Keep the 'ui' folder next to RootUI.exe.");
}

fn write_if_missing(path: &Path, v: &Value) {
    if !path.exists() {
        let _ = fs::write(path, serde_json::to_string_pretty(v).unwrap());
        println!("[info] Default file written to {}", path.display());
    }
}

fn mirror(src: &Path, dst: &Path) {
    let _ = fs::create_dir_all(dst);
    if let Ok(rd) = fs::read_dir(src) {
        for e in rd.flatten() {
            if e.path().is_file() {
                let _ = fs::copy(e.path(), dst.join(e.file_name()));
            }
        }
    }
}

pub fn bootstrap(ui_dir: &Path, user: &Path) -> Value {
    let _ = fs::create_dir_all(user);
    let settings_file = user.join("settings.json");
    write_if_missing(
        &settings_file,
        &json!({
            "shell": if cfg!(windows) { "powershell.exe" } else if cfg!(target_os = "macos") { "zsh" } else { "bash" },
            "shellArgs": "",
            "cwd": user.to_string_lossy(),
            "keyboard": "en-US",
            "theme": "tron",
            "termFontSize": 15,
            "audio": true,
            "audioVolume": 1.0,
            "disableFeedbackAudio": false,
            "clockHours": 24,
            "pingAddr": "1.1.1.1",
            "port": 3000,
            "nointro": false,
            "nocursor": false,
            "forceFullscreen": true,
            "allowWindowed": false,
            "excludeThreadsFromToplist": true,
            "hideDotfiles": false,
            "fsListView": false,
            "experimentalGlobeFeatures": false,
            "experimentalFeatures": false
        }),
    );
    write_if_missing(
        &user.join("shortcuts.json"),
        &json!([
            { "type": "app", "trigger": "Ctrl+Shift+C", "action": "COPY", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+V", "action": "PASTE", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Tab", "action": "NEXT_TAB", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+Tab", "action": "PREVIOUS_TAB", "enabled": true },
            { "type": "app", "trigger": "Ctrl+X", "action": "TAB_X", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+S", "action": "SETTINGS", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+K", "action": "SHORTCUTS", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+F", "action": "FUZZY_SEARCH", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+L", "action": "FS_LIST_VIEW", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+H", "action": "FS_DOTFILES", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+P", "action": "KB_PASSMODE", "enabled": true },
            { "type": "app", "trigger": "Ctrl+Shift+I", "action": "DEV_DEBUG", "enabled": false },
            { "type": "app", "trigger": "Ctrl+Shift+F5", "action": "DEV_RELOAD", "enabled": true },
            { "type": "shell", "trigger": "Ctrl+Shift+Alt+Space", "action": "neofetch", "linebreak": true, "enabled": false }
        ]),
    );
    write_if_missing(&user.join("lastWindowState.json"), &json!({ "useFullscreen": true }));

    // Mirror internal assets (themes / keyboard layouts / fonts) like the original.
    mirror(&ui_dir.join("assets/themes"), &user.join("themes"));
    mirror(&ui_dir.join("assets/kb_layouts"), &user.join("keyboards"));
    mirror(&ui_dir.join("assets/fonts"), &user.join("fonts"));

    let text = fs::read_to_string(&settings_file).unwrap_or_else(|_| "{}".into());
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("RootUI: settings.json is invalid JSON ({e}). Fix or delete {}", settings_file.display());
        std::process::exit(1);
    })
}

/// Find an executable in PATH (tiny `which`).
pub fn which(cmd: &str) -> Option<PathBuf> {
    let p = Path::new(cmd);
    if p.components().count() > 1 {
        return if p.exists() { Some(p.to_path_buf()) } else { None };
    }
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or(".EXE;.CMD;.BAT;.COM".into())
            .split(';')
            .map(|s| s.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        for ext in &exts {
            let mut name = cmd.to_string();
            if cfg!(windows) && !name.to_lowercase().ends_with(ext.as_str()) {
                name.push_str(ext);
            }
            let full = dir.join(&name);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}
