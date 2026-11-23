//! Terminal back-end: replaces node-pty + ws from eDEX-UI's terminal.class.js (server role).
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub type Notify = Arc<dyn Fn(String, Vec<Value>) + Send + Sync>;

struct Client {
    tx: Option<UnboundedSender<Vec<u8>>>,
    backlog: Vec<u8>,
}

pub struct Tty {
    pub id: u16,
    pub pid: Option<u32>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    client: Mutex<Client>,
    pub cwd: Mutex<Option<String>>,
    process: Mutex<String>,
    dirty_cwd: AtomicBool,
    dirty_proc: AtomicBool,
    disable_cwd: AtomicBool,
    pub closed: AtomicBool,
    renderer_up: AtomicBool,
    warm_until: Mutex<std::time::Instant>,
    fallback_cwd: String,
    cwd_file: Option<std::path::PathBuf>,
    notify: Notify,
}

pub struct SpawnOpts {
    pub shell: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: Vec<(String, String)>,
    /// File the shell's prompt hook writes its current directory to (PowerShell on Windows).
    pub cwd_file: Option<std::path::PathBuf>,
}

impl Tty {
    pub fn spawn(id: u16, o: SpawnOpts, notify: Notify, on_exit: Arc<dyn Fn(u16) + Send + Sync>) -> Result<Arc<Tty>, String> {
        let pair = native_pty_system()
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
        let mut cmd = CommandBuilder::new(&o.shell);
        cmd.args(&o.args);
        cmd.cwd(&o.cwd);
        for (k, v) in &o.env {
            cmd.env(k, v);
        }
        let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);
        let pid = child.process_id();
        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let tty = Arc::new(Tty {
            id,
            pid,
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            killer: Mutex::new(killer),
            client: Mutex::new(Client { tx: None, backlog: vec![] }),
            cwd: Mutex::new(None),
            process: Mutex::new(String::new()),
            dirty_cwd: AtomicBool::new(false),
            dirty_proc: AtomicBool::new(false),
            // Windows has no cheap cwd lookup, unless the shell reports it through `cwd_file` (PowerShell hook).
            disable_cwd: AtomicBool::new(cfg!(windows) && o.cwd_file.is_none()),
            closed: AtomicBool::new(false),
            renderer_up: AtomicBool::new(false),
            warm_until: Mutex::new(std::time::Instant::now()),
            fallback_cwd: o.cwd.clone(),
            cwd_file: o.cwd_file.clone(),
            notify,
        });
        if let Some(f) = &o.cwd_file {
            let _ = std::fs::write(f, &o.cwd); // initial value, so the file browser has something right away
        }

        // Output pump
        {
            let t = tty.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 16384];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            t.dirty_cwd.store(true, Ordering::Relaxed);
                            t.dirty_proc.store(true, Ordering::Relaxed);
                            let mut c = t.client.lock().unwrap();
                            let data = buf[..n].to_vec();
                            let sent = match &c.tx {
                                Some(tx) => tx.send(data.clone()).is_ok(),
                                None => false,
                            };
                            if !sent {
                                c.tx = None;
                                c.backlog.extend_from_slice(&data);
                                if c.backlog.len() > 262_144 {
                                    let cut = c.backlog.len() - 262_144;
                                    c.backlog.drain(..cut);
                                }
                            }
                        }
                    }
                }
            });
        }
        // Exit watcher
        {
            let t = tty.clone();
            std::thread::spawn(move || {
                let _ = child.wait();
                t.closed.store(true, Ordering::SeqCst);
                t.client.lock().unwrap().tx = None; // closes the websocket
                on_exit(t.id);
            });
        }
        // cwd / process tracker (1 Hz, only when output happened)
        {
            let t = tty.clone();
            std::thread::spawn(move || {
                let mut sys = sysinfo::System::new();
                while !t.closed.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_secs(1));
                    if !t.renderer_up.load(Ordering::Relaxed) {
                        continue;
                    }
                    // The front-end installs its handlers a few seconds after connecting, so keep
                    // re-sending for a short warm-up period after "Renderer startup".
                    let warm = std::time::Instant::now() < *t.warm_until.lock().unwrap();
                    let ch = format!("terminal_channel-{}", t.id);
                    if t.disable_cwd.load(Ordering::Relaxed) {
                        if warm {
                            (t.notify)(ch.clone(), vec![json!("Fallback cwd"), json!(t.fallback_cwd)]);
                        }
                    } else if t.dirty_cwd.swap(false, Ordering::Relaxed) || warm {
                        let found = match &t.cwd_file {
                            Some(f) => std::fs::read_to_string(f).ok().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()),
                            None => t.pid.and_then(tty_cwd),
                        };
                        match found {
                            Some(cwd) => {
                                let mut cur = t.cwd.lock().unwrap();
                                if warm || cur.as_deref() != Some(cwd.as_str()) {
                                    *cur = Some(cwd.clone());
                                    (t.notify)(ch.clone(), vec![json!("New cwd"), json!(cwd)]);
                                }
                            }
                            None => {
                                if t.cwd_file.is_none() {
                                    t.disable_cwd.store(true, Ordering::Relaxed);
                                    (t.notify)(ch.clone(), vec![json!("Fallback cwd"), json!(t.fallback_cwd)]);
                                }
                            }
                        }
                    }
                    if t.dirty_proc.swap(false, Ordering::Relaxed) || warm {
                        let name = t.pid.map(|p| tty_process(&mut sys, p)).unwrap_or_default();
                        let mut cur = t.process.lock().unwrap();
                        if warm || *cur != name {
                            *cur = name.clone();
                            (t.notify)(ch, vec![json!("New process"), json!(name)]);
                        }
                    }
                }
            });
        }
        Ok(tty)
    }

    /// Called when the renderer announces itself ("Renderer startup").
    pub fn renderer_startup(&self) {
        self.renderer_up.store(true, Ordering::Relaxed);
        *self.warm_until.lock().unwrap() = std::time::Instant::now() + Duration::from_secs(20);
        let ch = format!("terminal_channel-{}", self.id);
        if self.disable_cwd.load(Ordering::Relaxed) {
            (self.notify)(ch, vec![json!("Fallback cwd"), json!(self.fallback_cwd)]);
        } else if let Some(c) = self.cwd.lock().unwrap().clone() {
            (self.notify)(ch, vec![json!("New cwd"), json!(c)]);
        } else if let Some(f) = &self.cwd_file {
            if let Ok(c) = std::fs::read_to_string(f) {
                let c = c.trim().to_string();
                if !c.is_empty() {
                    (self.notify)(ch, vec![json!("New cwd"), json!(c)]);
                }
            }
        }
        self.dirty_cwd.store(true, Ordering::Relaxed);
        self.dirty_proc.store(true, Ordering::Relaxed);
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.lock().unwrap().resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
    }

    pub fn write(&self, data: &[u8]) {
        let mut w = self.writer.lock().unwrap();
        let _ = w.write_all(data);
        let _ = w.flush();
    }

    /// Attach a websocket client; only one at a time (like the original).
    pub fn attach(&self) -> Option<UnboundedReceiver<Vec<u8>>> {
        let mut c = self.client.lock().unwrap();
        if c.tx.is_some() || self.closed.load(Ordering::SeqCst) {
            return None;
        }
        let (tx, rx) = unbounded_channel();
        if !c.backlog.is_empty() {
            let _ = tx.send(std::mem::take(&mut c.backlog));
        }
        c.tx = Some(tx);
        Some(rx)
    }

    pub fn detach(&self) {
        self.client.lock().unwrap().tx = None;
    }

    pub fn kill(&self) {
        self.closed.store(true, Ordering::SeqCst);
        if let Some(f) = &self.cwd_file {
            let _ = std::fs::remove_file(f);
        }
        let _ = self.killer.lock().unwrap().kill();
    }
}

#[cfg(target_os = "linux")]
fn tty_cwd(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok().map(|p| p.to_string_lossy().into_owned())
}
#[cfg(target_os = "macos")]
fn tty_cwd(pid: u32) -> Option<String> {
    let out = std::process::Command::new("lsof").args(["-a", "-d", "cwd", "-Fn", "-p", &pid.to_string()]).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().find_map(|l| l.strip_prefix('n').map(|s| s.to_string()))
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn tty_cwd(_pid: u32) -> Option<String> {
    None
}

/// Name of the most recently started descendant of the shell (what the user is running), or the shell itself.
fn tty_process(sys: &mut sysinfo::System, shell_pid: u32) -> String {
    use sysinfo::{Pid, ProcessesToUpdate};
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let root = Pid::from_u32(shell_pid);
    let mut best: Option<(u32, String)> = None;
    for (pid, p) in sys.processes() {
        let mut cur = p.parent();
        let mut depth = 0;
        let mut is_desc = false;
        while let Some(par) = cur {
            if par == root {
                is_desc = true;
                break;
            }
            depth += 1;
            if depth > 16 {
                break;
            }
            cur = sys.process(par).and_then(|x| x.parent());
        }
        if is_desc && p.thread_kind().is_none() {
            let id = pid.as_u32();
            if best.as_ref().map(|b| id > b.0).unwrap_or(true) {
                best = Some((id, p.name().to_string_lossy().into_owned()));
            }
        }
    }
    match best {
        Some((_, n)) => n,
        None => sys.process(root).map(|p| p.name().to_string_lossy().into_owned()).unwrap_or_default(),
    }
}
