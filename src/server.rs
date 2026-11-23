//! Local backend: serves the UI, a small fs/system API, the IPC channel and the PTY websockets.
//! Bound to 127.0.0.1 on a random port and protected by a per-launch secret token.
use crate::{geo::Geo, pty, si};
use axum::{
    body::Body,
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tao::event_loop::EventLoopProxy;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub enum UserEvent {
    Window(Value),
    Quit,
    Relaunch,
}

pub struct AppState {
    pub token: String,
    pub port: u16,
    pub ui_dir: PathBuf,
    pub user_dir: PathBuf,
    pub settings: Value,
    pub proxy: Mutex<EventLoopProxy<UserEvent>>,
    pub theme_override: Mutex<Value>,
    pub kb_override: Mutex<Value>,
    pub ipc_tx: Mutex<Option<UnboundedSender<String>>>,
    pub ttys: Mutex<HashMap<u16, Arc<pty::Tty>>>,
    pub main_tty: u16,
    pub geo: Geo,
    pub displays: usize,
    pub args: Vec<String>,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub fn send_ipc(&self, channel: &str, args: Vec<Value>) {
        let msg = json!({ "c": channel, "a": args }).to_string();
        if let Some(tx) = self.ipc_tx.lock().unwrap().as_ref() {
            let _ = tx.send(msg);
        }
    }
    pub fn event(&self, e: UserEvent) {
        let _ = self.proxy.lock().unwrap().send_event(e);
    }
    pub fn kill_all(&self) {
        for t in self.ttys.lock().unwrap().values() {
            t.kill();
        }
    }
}

/// Map the virtual paths used by the web UI to real paths.
pub fn real_path(s: &AppState, p: &str) -> PathBuf {
    let p = p.replace('\\', "/");
    for (prefix, base) in [("/__userdata", &s.user_dir), ("/__app", &s.ui_dir)] {
        if p == prefix {
            return base.clone();
        }
        if let Some(rest) = p.strip_prefix(&format!("{prefix}/")) {
            return base.join(rest);
        }
    }
    PathBuf::from(p)
}

pub fn spawn_tty(s: &Shared, id: u16) -> Result<Arc<pty::Tty>, String> {
    let set = &s.settings;
    let shell_name = set["shell"].as_str().unwrap_or(if cfg!(windows) { "powershell.exe" } else { "bash" });
    let shell = crate::config::which(shell_name).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| {
        eprintln!("[warning] Shell '{shell_name}' not found in PATH, falling back.");
        if cfg!(windows) { "cmd.exe".into() } else { "sh".into() }
    });
    let args_str = set["shellArgs"].as_str().unwrap_or("");
    let mut args: Vec<String> = if args_str.trim().is_empty() {
        if cfg!(windows) { vec![] } else { vec!["-l".into()] }
    } else {
        args_str.split_whitespace().map(|x| x.to_string()).collect()
    };
    // PowerShell (Windows PowerShell / pwsh): install a prompt hook that reports the current directory
    // (so the file browser follows `cd`) and switch the console colours to plain black/grey, which the
    // terminal then maps onto the active theme.
    let shell_stem = std::path::Path::new(&shell)
        .file_stem()
        .map(|x| x.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let is_ps = shell_stem == "powershell" || shell_stem == "pwsh";
    let cwd_file = if is_ps {
        let _ = std::fs::create_dir_all(&s.user_dir);
        Some(s.user_dir.join(format!("cwd-{id}.txt")))
    } else {
        None
    };
    if is_ps && args_str.trim().is_empty() {
        args = vec!["-NoLogo".into(), "-NoExit".into(), "-EncodedCommand".into(), ps_bootstrap()];
    }
    let mut cwd = set["cwd"].as_str().map(|x| x.to_string()).unwrap_or_else(|| s.user_dir.to_string_lossy().to_string());
    if id != s.main_tty {
        if let Some(main) = s.ttys.lock().unwrap().get(&s.main_tty) {
            if let Some(c) = main.cwd.lock().unwrap().clone() {
                cwd = c;
            }
        }
    }
    if !std::path::Path::new(&cwd).is_dir() {
        cwd = dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_else(|| ".".into());
    }
    let mut env = vec![
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("COLORTERM".to_string(), "truecolor".to_string()),
        ("TERM_PROGRAM".to_string(), "RootUI".to_string()),
        ("TERM_PROGRAM_VERSION".to_string(), crate::config::VERSION.to_string()),
    ];
    if let Some(o) = set["env"].as_object() {
        for (k, v) in o {
            if let Some(v) = v.as_str() {
                env.push((k.clone(), v.to_string()));
            }
        }
    }
    if let Some(f) = &cwd_file {
        env.push(("ROOTUI_CWD_FILE".to_string(), f.to_string_lossy().to_string()));
    }
    let s2 = s.clone();
    let notify: pty::Notify = Arc::new(move |ch, args| s2.send_ipc(&ch, args));
    let s3 = s.clone();
    let on_exit: Arc<dyn Fn(u16) + Send + Sync> = Arc::new(move |id| {
        if id == s3.main_tty {
            println!("[info] Main terminal exited, closing RootUI.");
            s3.event(UserEvent::Quit);
        } else {
            s3.ttys.lock().unwrap().remove(&id);
        }
    });
    let tty = pty::Tty::spawn(id, pty::SpawnOpts { shell, args, cwd, env, cwd_file }, notify, on_exit)?;
    s.ttys.lock().unwrap().insert(id, tty.clone());
    Ok(tty)
}

/// Startup script for PowerShell: reports the current directory after every prompt and uses plain colours.
const PS_BOOTSTRAP: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$global:__rootui_file = $env:ROOTUI_CWD_FILE
$global:__rootui_prompt = $function:prompt
function global:prompt {
    try {
        $l = Get-Location
        if ($l.Provider.Name -eq 'FileSystem' -and $global:__rootui_file) {
            [System.IO.File]::WriteAllText($global:__rootui_file, $l.ProviderPath)
        }
    } catch {}
    if ($global:__rootui_prompt) { & $global:__rootui_prompt } else { 'PS ' + $PWD + '> ' }
}
try {
    $ui = $Host.UI.RawUI
    $ui.BackgroundColor = 'Black'
    $ui.ForegroundColor = 'Gray'
} catch {}
"#;

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | (*c.get(2).unwrap_or(&0) as u32);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// PowerShell's -EncodedCommand wants base64 of the UTF-16LE script text.
fn ps_bootstrap() -> String {
    let bytes: Vec<u8> = PS_BOOTSTRAP.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    b64(&bytes)
}

pub async fn run(listener: std::net::TcpListener, state: Shared) {
    listener.set_nonblocking(true).unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let app = Router::new()
        .route("/api/info", get(api_info))
        .route("/api/fs", get(api_fs).post(api_fs_write))
        .route("/api/clipboard", get(api_clip_get).post(api_clip_set))
        .route("/api/window", post(api_window))
        .route("/api/shell/open", post(api_open))
        .route("/api/geoip", get(api_geoip))
        .route("/api/geoip/init", get(api_geoip_init))
        .route("/api/externalip", get(api_externalip))
        .route("/api/ping", get(api_ping))
        .route("/api/gist", get(api_gist))
        .route("/api/fslist", get(api_fslist))
        .route("/ipc", get(ws_ipc))
        .route("/pty/{id}", get(ws_pty))
        .route("/__file/{*p}", get(file_any))
        .fallback(static_files)
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state);
    axum::serve(listener, app).await.unwrap();
}

// ---------- auth ----------
async fn auth(State(s): State<Shared>, req: Request, next: Next) -> Response {
    let cookie_ok = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|c| c.split(';').any(|kv| kv.trim() == format!("rt={}", s.token)))
        .unwrap_or(false);
    let query_ok = req.uri().query().map(|q| q.split('&').any(|kv| kv == format!("t={}", s.token))).unwrap_or(false);
    if !cookie_ok && !query_ok {
        return (StatusCode::FORBIDDEN, "RootUI: forbidden").into_response();
    }
    let mut resp = next.run(req).await;
    if query_ok {
        let c = format!("rt={}; Path=/; HttpOnly; SameSite=Strict", s.token);
        resp.headers_mut().append(header::SET_COOKIE, HeaderValue::from_str(&c).unwrap());
    }
    resp.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    resp
}

fn origin_ok(s: &AppState, h: &HeaderMap) -> bool {
    match h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        None => true,
        Some(o) => o == format!("http://127.0.0.1:{}", s.port),
    }
}

// ---------- static ----------
fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn serve_path(path: PathBuf, req: Request) -> Response {
    match ServeFile::new(path).oneshot(req).await {
        Ok(r) => r.map(Body::new).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn static_files(State(s): State<Shared>, uri: Uri, req: Request) -> Response {
    let p = pct_decode(uri.path());
    let rel = if p == "/" { "/ui.html".to_string() } else { p };
    if rel.split('/').any(|c| c == "..") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = if rel.starts_with("/__userdata/") || rel.starts_with("/__app/") {
        real_path(&s, &rel)
    } else {
        s.ui_dir.join(rel.trim_start_matches('/'))
    };
    if !path.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_path(path, req).await
}

async fn file_any(State(_s): State<Shared>, Path(p): Path<String>, req: Request) -> Response {
    let p = pct_decode(&p);
    let path = if cfg!(windows) { PathBuf::from(p) } else { PathBuf::from(format!("/{p}")) };
    if !path.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_path(path, req).await
}

// ---------- api ----------
async fn api_info(State(s): State<Shared>) -> Json<Value> {
    let platform = if cfg!(windows) { "win32" } else if cfg!(target_os = "macos") { "darwin" } else { "linux" };
    let mut sys_name = sysinfo::System::name().unwrap_or_default();
    if sys_name.is_empty() {
        sys_name = platform.into();
    }
    Json(json!({
        "platform": platform,
        "type": if cfg!(windows) { "Windows_NT" } else if cfg!(target_os = "macos") { "Darwin" } else { "Linux" },
        "release": sysinfo::System::kernel_version().unwrap_or_default(),
        "arch": std::env::consts::ARCH,
        "osName": sys_name,
        "version": crate::config::VERSION,
        "userData": "/__userdata",
        "userDataReal": s.user_dir.to_string_lossy(),
        "cwd": std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "home": dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "hostname": sysinfo::System::host_name().unwrap_or_default(),
        "username": std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_else(|_| "user".into()),
        "uptime": sysinfo::System::uptime(),
        "argv": s.args,
        "displays": s.displays,
        "pid": std::process::id(),
        "fullscreen": s.settings["forceFullscreen"].as_bool().unwrap_or(true),
        "mainTty": s.main_tty,
    }))
}

fn err_json(e: std::io::Error) -> Json<Value> {
    use std::io::ErrorKind::*;
    let code = match e.kind() {
        NotFound => "ENOENT",
        PermissionDenied => "EACCES",
        AlreadyExists => "EEXIST",
        _ => "EIO",
    };
    Json(json!({ "ok": false, "code": code, "message": format!("{code}: {e}") }))
}

fn stat_json(m: &std::fs::Metadata) -> Value {
    let mtime = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
    json!({
        "ok": true, "isFile": m.is_file(), "isDirectory": m.is_dir(),
        "isSymbolicLink": m.file_type().is_symlink(), "size": m.len(), "mtimeMs": mtime
    })
}

async fn api_fs(State(s): State<Shared>, Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let op = q.get("op").map(|x| x.as_str()).unwrap_or("");
    let path = real_path(&s, q.get("path").map(|x| x.as_str()).unwrap_or(""));
    match op {
        "readdir" => match std::fs::read_dir(&path) {
            Ok(rd) => {
                let names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
                Json(json!({ "ok": true, "entries": names }))
            }
            Err(e) => err_json(e),
        },
        "lstat" => std::fs::symlink_metadata(&path).map(|m| Json(stat_json(&m))).unwrap_or_else(err_json),
        "stat" => std::fs::metadata(&path).map(|m| Json(stat_json(&m))).unwrap_or_else(err_json),
        "exists" => Json(json!({ "ok": true, "exists": path.exists() })),
        "read" => match std::fs::read(&path) {
            Ok(b) => Json(json!({ "ok": true, "data": String::from_utf8_lossy(&b) })),
            Err(e) => err_json(e),
        },
        "readlink" => match std::fs::read_link(&path) {
            Ok(p) => Json(json!({ "ok": true, "target": p.to_string_lossy() })),
            Err(e) => err_json(e),
        },
        "mkdir" => std::fs::create_dir_all(&path).map(|_| Json(json!({ "ok": true }))).unwrap_or_else(err_json),
        // Only used to delete custom themes: refuses anything outside RootUI's own config folder.
        "unlink" => {
            let inside = match (path.canonicalize(), s.user_dir.canonicalize()) {
                (Ok(p), Ok(u)) => p.starts_with(u),
                _ => false,
            };
            if !inside {
                return Json(json!({ "ok": false, "code": "EACCES", "message": "EACCES: only files in the RootUI config folder can be deleted" }));
            }
            std::fs::remove_file(&path).map(|_| Json(json!({ "ok": true }))).unwrap_or_else(err_json)
        }
        _ => Json(json!({ "ok": false, "code": "EINVAL", "message": "unknown op" })),
    }
}

async fn api_fs_write(State(s): State<Shared>, Query(q): Query<HashMap<String, String>>, body: String) -> Json<Value> {
    let path = real_path(&s, q.get("path").map(|x| x.as_str()).unwrap_or(""));
    std::fs::write(&path, body).map(|_| Json(json!({ "ok": true }))).unwrap_or_else(err_json)
}

async fn api_clip_get() -> String {
    arboard::Clipboard::new().and_then(|mut c| c.get_text()).unwrap_or_default()
}

async fn api_clip_set(body: String) -> StatusCode {
    let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(body));
    StatusCode::OK
}

async fn api_window(State(s): State<Shared>, Json(v): Json<Value>) -> StatusCode {
    match v["cmd"].as_str() {
        Some("quit") => s.event(UserEvent::Quit),
        Some("relaunch") => s.event(UserEvent::Relaunch),
        _ => s.event(UserEvent::Window(v)),
    }
    StatusCode::OK
}

async fn api_open(State(s): State<Shared>, Json(v): Json<Value>) -> StatusCode {
    if let Some(p) = v["path"].as_str() {
        if p.starts_with("http://") || p.starts_with("https://") {
            let _ = open::that_detached(p);
        } else {
            let _ = open::that_detached(real_path(&s, p));
        }
    }
    StatusCode::OK
}

async fn api_geoip(State(s): State<Shared>, Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let ip = q.get("ip").cloned().unwrap_or_default();
    match s.geo.lookup(&ip) {
        Some((la, lo)) => Json(json!({ "location": { "latitude": la, "longitude": lo } })),
        None => Json(Value::Null),
    }
}

async fn api_geoip_init(State(s): State<Shared>) -> Json<Value> {
    // Download / load the database in the background; lookups use the online fallback until it is ready.
    let _ = tokio::task::spawn_blocking(move || s.geo.init());
    Json(json!({ "ok": true }))
}

async fn api_externalip(State(s): State<Shared>) -> Response {
    let res = tokio::task::spawn_blocking(move || {
        let t = crate::geo::external_ip();
        if let Some(txt) = &t {
            if let Some(ip) = serde_json::from_str::<Value>(txt).ok().and_then(|j| j["ip"].as_str().map(|x| x.to_string())) {
                s.geo.warm(&ip);
            }
        }
        t
    })
    .await
    .ok()
    .flatten();
    match res {
        Some(t) => ([(header::CONTENT_TYPE, "application/json")], t).into_response(),
        None => StatusCode::BAD_GATEWAY.into_response(),
    }
}

fn probe(host: &str, port: u16, timeout_ms: u64) -> Option<f64> {
    use std::net::ToSocketAddrs;
    let addr = (host, port).to_socket_addrs().ok()?.find(|a| a.is_ipv4())?;
    let t = std::time::Instant::now();
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(timeout_ms)).ok()?;
    Some(t.elapsed().as_secs_f64() * 1000.0)
}

async fn api_ping(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let host = q.get("host").cloned().unwrap_or_default();
    let port: u16 = q.get("port").and_then(|p| p.parse().ok()).unwrap_or(80);
    let r = tokio::task::spawn_blocking(move || {
        // The configured address first, then a few well-known ones (some ISPs block individual addresses).
        let mut targets: Vec<(String, u16)> = vec![(host, port)];
        for (h, p) in [("1.1.1.1", 443u16), ("8.8.8.8", 53), ("8.8.8.8", 443), ("9.9.9.9", 443), ("208.67.222.222", 443)] {
            targets.push((h.to_string(), p));
        }
        let (tx, rx) = std::sync::mpsc::channel::<f64>();
        for (h, p) in targets {
            let tx = tx.clone();
            std::thread::spawn(move || {
                if let Some(ms) = probe(&h, p, 2000) {
                    let _ = tx.send(ms);
                }
            });
        }
        drop(tx);
        rx.recv_timeout(std::time::Duration::from_millis(2500)).ok()
    })
    .await
    .ok()
    .flatten();
    match r {
        Some(ms) => Json(json!({ "ok": true, "ms": ms })),
        None => Json(json!({ "ok": false })),
    }
}

/// Text of a (GitHub gist raw) URL - used for the Discord invite link that can be changed without a new release.
async fn api_gist(Query(q): Query<HashMap<String, String>>) -> Response {
    let url = q.get("url").cloned().unwrap_or_default();
    if !(url.starts_with("https://gist.githubusercontent.com/") || url.starts_with("https://gist.github.com/")) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let res = tokio::task::spawn_blocking(move || {
        ureq::get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .call()
            .ok()
            .and_then(|r| r.into_string().ok())
    })
    .await
    .ok()
    .flatten();
    match res {
        Some(t) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], t).into_response(),
        None => StatusCode::BAD_GATEWAY.into_response(),
    }
}

/// Directory listing with stat info in one request (the file browser). Entries that cannot be read are
/// still listed (as "system" items) instead of failing the whole directory.
async fn api_fslist(State(s): State<Shared>, Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let path = real_path(&s, q.get("path").map(|x| x.as_str()).unwrap_or(""));
    match std::fs::read_dir(&path) {
        Ok(rd) => {
            let mut items: Vec<Value> = vec![];
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                match std::fs::symlink_metadata(e.path()) {
                    Ok(m) => {
                        let mut v = stat_json(&m);
                        v["name"] = json!(name);
                        items.push(v);
                    }
                    Err(_) => items.push(json!({ "ok": false, "name": name })),
                }
            }
            Json(json!({ "ok": true, "entries": items }))
        }
        Err(e) => err_json(e),
    }
}

// ---------- websockets ----------
async fn ws_ipc(State(s): State<Shared>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, ws: WebSocketUpgrade) -> Response {
    if !origin_ok(&s, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Some(tz) = q.get("tz").and_then(|t| t.parse::<i32>().ok()) {
        si::TZ_OFFSET_MIN.store(tz, std::sync::atomic::Ordering::Relaxed);
    }
    ws.on_upgrade(move |sock| ipc_session(s, sock))
}

async fn ipc_session(s: Shared, sock: WebSocket) {
    let (mut sink, mut stream) = sock.split();
    let (tx, mut rx) = unbounded_channel::<String>();
    *s.ipc_tx.lock().unwrap() = Some(tx.clone());
    let writer = tokio::spawn(async move {
        while let Some(m) = rx.recv().await {
            if sink.send(Message::Text(m.into())).await.is_err() {
                break;
            }
        }
    });
    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(t) = msg {
            if let Ok(v) = serde_json::from_str::<Value>(t.as_str()) {
                let ch = v["c"].as_str().unwrap_or("").to_string();
                let args = v["a"].as_array().cloned().unwrap_or_default();
                handle_ipc(&s, &ch, args);
            }
        }
    }
    {
        let mut g = s.ipc_tx.lock().unwrap();
        if g.as_ref().map(|c| c.same_channel(&tx)).unwrap_or(false) {
            *g = None;
        }
    }
    writer.abort();
}

fn handle_ipc(s: &Shared, ch: &str, a: Vec<Value>) {
    match ch {
        "log" => println!("[{}] {}", a.first().and_then(|x| x.as_str()).unwrap_or("info"), a.get(1).and_then(|x| x.as_str()).unwrap_or("")),
        "getThemeOverride" => s.send_ipc("getThemeOverride", vec![s.theme_override.lock().unwrap().clone()]),
        "getKbOverride" => s.send_ipc("getKbOverride", vec![s.kb_override.lock().unwrap().clone()]),
        "setThemeOverride" => *s.theme_override.lock().unwrap() = a.first().cloned().unwrap_or(Value::Null),
        "setKbOverride" => *s.kb_override.lock().unwrap() = a.first().cloned().unwrap_or(Value::Null),
        "ttyspawn" => {
            let s = s.clone();
            std::thread::spawn(move || {
                let base = s.main_tty;
                let free = (base + 2..=base + 5).find(|p| !s.ttys.lock().unwrap().contains_key(p));
                match free {
                    None => s.send_ipc("ttyspawn-reply", vec![json!("ERROR: max number of ttys reached")]),
                    Some(id) => match spawn_tty(&s, id) {
                        Ok(_) => s.send_ipc("ttyspawn-reply", vec![json!(format!("SUCCESS: {id}"))]),
                        Err(e) => s.send_ipc("ttyspawn-reply", vec![json!(format!("ERROR: {e}"))]),
                    },
                }
            });
        }
        "systeminformation-call" => {
            let s = s.clone();
            let name = a.first().and_then(|x| x.as_str()).unwrap_or("").to_string();
            let id = a.get(1).and_then(|x| x.as_str()).unwrap_or("").to_string();
            let arg = a.get(2).cloned();
            std::thread::spawn(move || {
                let res = si::call(&name, arg.as_ref());
                s.send_ipc(&format!("systeminformation-reply-{id}"), vec![res]);
            });
        }
        c if c.starts_with("terminal_channel-") => {
            let id: u16 = c["terminal_channel-".len()..].parse().unwrap_or(0);
            let tty = s.ttys.lock().unwrap().get(&id).cloned();
            if let Some(t) = tty {
                match a.first().and_then(|x| x.as_str()) {
                    Some("Renderer startup") => t.renderer_startup(),
                    Some("Resize") => {
                        let n = |i: usize| a.get(i).and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0) as u16;
                        let (cols, rows) = (n(1), n(2));
                        if cols > 0 && rows > 0 {
                            t.resize(cols, rows);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

async fn ws_pty(State(s): State<Shared>, Path(id): Path<u16>, headers: HeaderMap, ws: WebSocketUpgrade) -> Response {
    if !origin_ok(&s, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let tty = match s.ttys.lock().unwrap().get(&id).cloned() {
        Some(t) => t,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    match tty.attach() {
        Some(rx) => ws.on_upgrade(move |sock| pty_session(s, tty, rx, sock)),
        None => StatusCode::CONFLICT.into_response(),
    }
}

async fn pty_session(s: Shared, tty: Arc<pty::Tty>, mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>, sock: WebSocket) {
    let (mut sink, mut stream) = sock.split();
    let mut pending: Vec<u8> = Vec::new();
    loop {
        tokio::select! {
            out = rx.recv() => match out {
                Some(d) => {
                    pending.extend_from_slice(&d);
                    let text = take_utf8(&mut pending);
                    if !text.is_empty() && sink.send(Message::Text(text.into())).await.is_err() { break; }
                }
                None => break,
            },
            inp = stream.next() => match inp {
                Some(Ok(Message::Text(t))) => tty.write(t.as_bytes()),
                Some(Ok(Message::Binary(b))) => tty.write(&b),
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
    tty.detach();
    if tty.id != s.main_tty {
        tty.kill();
        s.ttys.lock().unwrap().remove(&tty.id);
    }
    let _ = sink.close().await;
}

/// Drain the longest valid UTF-8 prefix of `buf` (keeps an incomplete trailing sequence for next time).
fn take_utf8(buf: &mut Vec<u8>) -> String {
    let mut out = String::new();
    loop {
        match std::str::from_utf8(buf) {
            Ok(v) => {
                out.push_str(v);
                buf.clear();
                return out;
            }
            Err(e) => {
                let valid = e.valid_up_to();
                out.push_str(std::str::from_utf8(&buf[..valid]).unwrap());
                match e.error_len() {
                    None => {
                        buf.drain(..valid); // incomplete tail: wait for more bytes
                        return out;
                    }
                    Some(n) => {
                        out.push('\u{FFFD}');
                        buf.drain(..valid + n);
                    }
                }
            }
        }
    }
}
