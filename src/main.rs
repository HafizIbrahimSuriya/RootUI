//! RootUI - a sci-fi terminal emulator / system monitor desktop app, rebuilt in Rust.
//! Based on eDEX-UI by Gabriel 'Squared' Saillard (GPL-3.0).
// Release builds on Windows are GUI apps: no console / terminal window opens with the exe.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod geo;
mod pty;
mod server;
mod si;

use rand::Rng;
use server::{AppState, UserEvent};
use std::sync::{Arc, Mutex};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Fullscreen, WindowBuilder},
};
use wry::WebViewBuilder;

fn main() {
    // WebKitGTK's DMABUF renderer gives a blank window on some GPU drivers / Wayland setups; this avoids it.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ui_dir = config::find_ui_dir();
    let user_dir = config::user_dir();
    let settings = config::bootstrap(&ui_dir, &user_dir);
    println!("[info] RootUI {} starting. Config: {}", config::VERSION, user_dir.display());

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let displays = event_loop.available_monitors().count().max(1);

    // Local backend on a random loopback port, protected by a secret token.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("cannot bind loopback port");
    let port = listener.local_addr().unwrap().port();
    let token: String = (0..32).map(|_| format!("{:x}", rand::rng().random_range(0..16u8))).collect();
    let main_tty = settings["port"].as_u64().unwrap_or(3000).clamp(1024, 60000) as u16;

    let state = Arc::new(AppState {
        token: token.clone(),
        port,
        ui_dir: ui_dir.clone(),
        user_dir: user_dir.clone(),
        settings: settings.clone(),
        proxy: Mutex::new(event_loop.create_proxy()),
        theme_override: Mutex::new(serde_json::Value::Null),
        kb_override: Mutex::new(serde_json::Value::Null),
        ipc_tx: Mutex::new(None),
        ttys: Mutex::new(Default::default()),
        main_tty,
        geo: geo::Geo::new(&user_dir),
        displays,
        args,
    });

    if let Err(e) = server::spawn_tty(&state, main_tty) {
        config::fatal(&format!("RootUI: could not start the shell: {e}"));
    }

    {
        let st = state.clone();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(server::run(listener, st));
        });
    }

    let allow_windowed = settings["allowWindowed"].as_bool().unwrap_or(false);
    let force_fs = settings["forceFullscreen"].as_bool().unwrap_or(true);
    // `monitor` setting: index of the display to start on (defaults to the primary one)
    let monitor = settings["monitor"]
        .as_u64()
        .and_then(|i| event_loop.available_monitors().nth(i as usize))
        .or_else(|| event_loop.primary_monitor());
    let mut wb = WindowBuilder::new()
        .with_title("RootUI")
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .with_min_inner_size(LogicalSize::new(800.0, 450.0))
        .with_window_icon(tao::window::Icon::from_rgba(include_bytes!("../icon64.rgba").to_vec(), 64, 64).ok())
        .with_decorations(allow_windowed)
        .with_resizable(true)
        .with_visible(false); // shown once the page has loaded (no white box)
    if let Some(m) = &monitor {
        wb = wb.with_position(m.position());
    }
    let window = wb.build(&event_loop).expect("cannot create window");
    let fs_target = monitor.clone();
    if force_fs || !allow_windowed {
        window.set_fullscreen(Some(Fullscreen::Borderless(fs_target.clone())));
    }

    let mut url = format!("http://127.0.0.1:{port}/ui.html?t={token}");
    if std::env::var("ROOTUI_NOWEBGL").is_ok() {
        url.push_str("&nowebgl=1"); // use xterm's canvas/DOM renderer instead of WebGL
    }
    if cfg!(debug_assertions) || std::env::var("ROOTUI_DEBUG").is_ok() {
        println!("[debug] UI url: {url}");
    }
    let origin = format!("http://127.0.0.1:{port}");
    let mut ctx = wry::WebContext::new(Some(user_dir.join("webview")));
    let builder = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_url(&url)
        .with_devtools(false)
        .with_background_color((0, 0, 0, 255))
        .with_on_page_load_handler({
            let px = state.proxy.lock().unwrap().clone();
            move |ev, _| {
                if matches!(ev, wry::PageLoadEvent::Finished) {
                    let _ = px.send_event(UserEvent::Window(serde_json::json!({"cmd": "show"})));
                }
            }
        })
        .with_navigation_handler(move |u| {
            if u.starts_with(&origin) || u.starts_with("about:") || u.starts_with("blob:") || u.starts_with("data:") {
                true
            } else {
                if u.starts_with("http") {
                    let _ = open::that_detached(&u);
                }
                false
            }
        })
        .with_new_window_req_handler(|u, _| {
            if u.starts_with("http") {
                let _ = open::that_detached(&u);
            }
            wry::NewWindowResponse::Deny
        });
    #[cfg(windows)]
    let builder = {
        use wry::WebViewBuilderExtWindows;
        builder
            .with_additional_browser_args("--autoplay-policy=no-user-gesture-required --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection")
            .with_browser_accelerator_keys(false)
    };
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window).expect("cannot create webview");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().unwrap()).expect("cannot create webview")
    };
    let _keep = &webview;

    {
        let px = state.proxy.lock().unwrap().clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(4));
            let _ = px.send_event(UserEvent::Window(serde_json::json!({"cmd": "show"})));
        });
    }

    let size_before_fs = Mutex::new((1280.0f64, 720.0f64));
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } | Event::UserEvent(UserEvent::Quit) => {
                state.kill_all();
                *flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Relaunch) => {
                state.kill_all();
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).args(&state.args).spawn();
                }
                *flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Window(v)) => {
                let f = |k: &str| v[k].as_f64().unwrap_or(0.0);
                match v["cmd"].as_str().unwrap_or("") {
                    "show" => {
                        window.set_visible(true);
                        window.set_focus();
                    }
                    "minimize" => window.set_minimized(true),
                    "maximize" => window.set_maximized(true),
                    "unmaximize" => window.set_maximized(false),
                    "focus" => window.set_focus(),
                    "setFullScreen" => {
                        if v["value"].as_bool().unwrap_or(false) {
                            window.set_fullscreen(Some(Fullscreen::Borderless(fs_target.clone())));
                        } else {
                            window.set_fullscreen(None);
                            let (w, h) = *size_before_fs.lock().unwrap();
                            window.set_inner_size(LogicalSize::new(w, h));
                        }
                    }
                    "setSize" => {
                        if f("w") > 0.0 && f("h") > 0.0 {
                            window.set_inner_size(LogicalSize::new(f("w"), f("h")));
                            *size_before_fs.lock().unwrap() = (f("w"), f("h"));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
