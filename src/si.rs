//! Emulates the subset of the `systeminformation` npm package that eDEX-UI uses.
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use sysinfo::{Components, Disks, MemoryRefreshKind, Networks, ProcessesToUpdate, System, Users};

pub static TZ_OFFSET_MIN: AtomicI32 = AtomicI32::new(0); // JS getTimezoneOffset() (minutes behind UTC)

struct State {
    sys: System,
    nets: Networks,
    prev_net: HashMap<String, (u64, u64, Instant)>,
    users: Users,
}

fn st() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Mutex::new(State {
            sys,
            nets: Networks::new_with_refreshed_list(),
            prev_net: HashMap::new(),
            users: Users::new_with_refreshed_list(),
        })
    })
}

pub fn call(name: &str, arg: Option<&Value>) -> Value {
    match name {
        "cpu" => cpu(),
        "currentLoad" => current_load(),
        "cpuCurrentSpeed" => cpu_speed(),
        "cpuTemperature" => cpu_temp(),
        "mem" => mem(),
        "processes" => processes(),
        "battery" => battery(),
        "system" => system_info(),
        "chassis" => chassis(),
        "blockDevices" => block_devices(),
        "fsSize" => fs_size(),
        "networkInterfaces" => network_interfaces(),
        "networkStats" => network_stats(arg.and_then(|a| a.as_str()).unwrap_or("")),
        "networkConnections" => network_connections(),
        _ => Value::Null,
    }
}

fn r1(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn cpu() -> Value {
    let s = st().lock().unwrap();
    let cpus = s.sys.cpus();
    let brand = cpus.first().map(|c| c.brand().to_string()).unwrap_or_default();
    let clean = brand.replace("(R)", "").replace("(TM)", "").replace("(tm)", "").replace(" CPU", "");
    let (manu, rest) = if let Some(r) = clean.strip_prefix("Intel") {
        ("Intel", r.trim().to_string())
    } else if let Some(r) = clean.strip_prefix("AMD") {
        ("AMD", r.trim().to_string())
    } else {
        ("", clean.trim().to_string())
    };
    let ghz = cpus.first().map(|c| c.frequency() as f64 / 1000.0).unwrap_or(0.0);
    json!({
        "manufacturer": manu, "brand": rest, "speed": r1(ghz), "speedMax": r1(ghz),
        "cores": cpus.len(), "physicalCores": System::physical_core_count().unwrap_or(cpus.len())
    })
}

fn current_load() -> Value {
    let mut s = st().lock().unwrap();
    s.sys.refresh_cpu_usage();
    let cpus: Vec<Value> = s.sys.cpus().iter().map(|c| json!({ "load": c.cpu_usage() })).collect();
    json!({ "currentLoad": s.sys.global_cpu_usage(), "cpus": cpus })
}

fn cpu_speed() -> Value {
    let mut s = st().lock().unwrap();
    s.sys.refresh_cpu_frequency();
    let freqs: Vec<f64> = s.sys.cpus().iter().map(|c| c.frequency() as f64 / 1000.0).collect();
    let avg = if freqs.is_empty() { 0.0 } else { freqs.iter().sum::<f64>() / freqs.len() as f64 };
    let max = freqs.iter().cloned().fold(0.0, f64::max);
    let min = freqs.iter().cloned().fold(f64::MAX, f64::min);
    json!({ "avg": r1(avg), "max": r1(max), "min": r1(if min == f64::MAX { 0.0 } else { min }), "cores": freqs })
}

fn cpu_temp() -> Value {
    let comps = Components::new_with_refreshed_list();
    let mut max = 0.0f32;
    for c in comps.list() {
        let l = c.label().to_lowercase();
        if l.contains("cpu") || l.contains("core") || l.contains("package") || l.contains("tctl") || l.contains("k10temp") {
            if let Some(t) = c.temperature() {
                if t.is_finite() && t > max {
                    max = t;
                }
            }
        }
    }
    json!({ "max": max.round() })
}

fn mem() -> Value {
    let mut s = st().lock().unwrap();
    s.sys.refresh_memory_specifics(MemoryRefreshKind::everything());
    let total = s.sys.total_memory();
    let free = s.sys.free_memory().min(total);
    let avail = s.sys.available_memory().max(free).min(total);
    let used = total - free; // the UI asserts free + used == total
    let active = total - avail;
    json!({
        "total": total, "free": free, "used": used, "active": active, "available": avail,
        "swaptotal": s.sys.total_swap(), "swapused": s.sys.used_swap()
    })
}

fn civil(ts: i64) -> String {
    let days = ts.div_euclid(86400);
    let secs = ts.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, secs / 3600, (secs % 3600) / 60, secs % 60)
}

fn processes() -> Value {
    let mut s = st().lock().unwrap();
    s.sys.refresh_processes(ProcessesToUpdate::All, true);
    s.sys.refresh_memory();
    let total_mem = s.sys.total_memory().max(1) as f64;
    let ncpu = s.sys.cpus().len().max(1) as f32;
    let off = TZ_OFFSET_MIN.load(Ordering::Relaxed) as i64;
    let (mut running, mut sleeping, mut unknown) = (0, 0, 0);
    let mut list = vec![];
    for (pid, p) in s.sys.processes() {
        if p.thread_kind().is_some() {
            continue;
        }
        let state = match format!("{:?}", p.status()).to_lowercase().as_str() {
            "run" => {
                running += 1;
                "running"
            }
            "sleep" | "idle" => {
                sleeping += 1;
                "sleeping"
            }
            _ => {
                unknown += 1;
                "unknown"
            }
        };
        let user = p
            .user_id()
            .and_then(|u| s.users.get_user_by_id(u))
            .map(|u| u.name().to_string())
            .unwrap_or_default();
        list.push(json!({
            "pid": pid.as_u32(),
            "parentPid": p.parent().map(|x| x.as_u32()).unwrap_or(0),
            "name": p.name().to_string_lossy(),
            "cpu": (p.cpu_usage() / ncpu) as f64,
            "mem": p.memory() as f64 / total_mem * 100.0,
            "user": user,
            "state": state,
            "started": civil(p.start_time() as i64 - off * 60),
        }));
    }
    json!({ "all": list.len(), "running": running, "blocked": 0, "sleeping": sleeping, "unknown": unknown, "list": list })
}

// ---------- battery ----------
#[cfg(target_os = "linux")]
fn battery() -> Value {
    let rd = match std::fs::read_dir("/sys/class/power_supply") {
        Ok(r) => r,
        Err(_) => return json!({ "hasBattery": false, "isCharging": false, "acConnected": true, "percent": 0 }),
    };
    let (mut has, mut pct, mut charging, mut ac) = (false, 0.0, false, false);
    for e in rd.flatten() {
        let p = e.path();
        let ty = std::fs::read_to_string(p.join("type")).unwrap_or_default();
        match ty.trim() {
            "Battery" => {
                has = true;
                pct = std::fs::read_to_string(p.join("capacity")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                let status = std::fs::read_to_string(p.join("status")).unwrap_or_default();
                charging = status.trim() == "Charging";
            }
            "Mains" | "USB" => {
                if std::fs::read_to_string(p.join("online")).map(|s| s.trim() == "1").unwrap_or(false) {
                    ac = true;
                }
            }
            _ => {}
        }
    }
    json!({ "hasBattery": has, "isCharging": charging, "acConnected": ac || !has, "percent": pct })
}
#[cfg(windows)]
fn battery() -> Value {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    unsafe {
        let mut s: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut s) == 0 {
            return json!({ "hasBattery": false, "isCharging": false, "acConnected": true, "percent": 0 });
        }
        let has = s.BatteryFlag != 128 && s.BatteryFlag != 255;
        let pct = if s.BatteryLifePercent == 255 { 0 } else { s.BatteryLifePercent };
        json!({ "hasBattery": has, "isCharging": s.BatteryFlag & 8 != 0, "acConnected": s.ACLineStatus == 1 || !has, "percent": pct })
    }
}
#[cfg(target_os = "macos")]
fn battery() -> Value {
    let out = std::process::Command::new("pmset").args(["-g", "batt"]).output().ok();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let has = text.contains("InternalBattery");
    let pct = text.split('%').next().and_then(|s| s.rsplit(|c: char| !c.is_ascii_digit()).next()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    json!({ "hasBattery": has, "isCharging": text.contains("; charging") || text.contains("finishing charge"), "acConnected": text.contains("AC Power") || !has, "percent": pct })
}
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
fn battery() -> Value {
    json!({ "hasBattery": false, "isCharging": false, "acConnected": true, "percent": 0 })
}

fn has_battery() -> bool {
    battery()["hasBattery"].as_bool().unwrap_or(false)
}

// ---------- system / chassis ----------
#[allow(dead_code)]
fn run_hidden(cmd: &str, args: &[&str]) -> String {
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c.output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default()
}

#[cfg(windows)]
fn reg_bios(value: &str) -> String {
    let out = run_hidden("reg", &["query", r"HKLM\HARDWARE\DESCRIPTION\System\BIOS", "/v", value]);
    out.lines()
        .find(|l| l.contains(value))
        .and_then(|l| l.splitn(2, "REG_SZ").nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[allow(unused_assignments)]
fn system_info() -> Value {
    #[allow(unused_mut)]
    let (mut manu, mut model) = (String::new(), String::new());
    #[cfg(target_os = "linux")]
    {
        manu = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default().trim().to_string();
        model = std::fs::read_to_string("/sys/class/dmi/id/product_name").unwrap_or_default().trim().to_string();
    }
    #[cfg(windows)]
    {
        manu = reg_bios("SystemManufacturer");
        model = reg_bios("SystemProductName");
    }
    #[cfg(target_os = "macos")]
    {
        manu = "Apple".into();
        model = run_hidden("sysctl", &["-n", "hw.model"]).trim().to_string();
    }
    if manu.is_empty() {
        manu = "Unknown".into();
    }
    if model.is_empty() {
        model = "Unknown".into();
    }
    json!({ "manufacturer": manu, "model": model })
}

fn chassis() -> Value {
    #[allow(unused_mut)]
    let mut ty = if has_battery() { "Laptop" } else { "Desktop" }.to_string();
    #[cfg(target_os = "linux")]
    if let Ok(n) = std::fs::read_to_string("/sys/class/dmi/id/chassis_type") {
        ty = match n.trim().parse::<u32>().unwrap_or(0) {
            8 | 9 | 10 | 14 | 31 => "Laptop",
            30 => "Tablet",
            3 | 4 | 5 | 6 | 7 | 13 | 15 | 16 | 17 | 23 | 24 | 35 | 36 => "Desktop",
            _ => ty.as_str(),
        }
        .to_string();
    }
    json!({ "type": ty })
}

// ---------- disks ----------
fn block_devices() -> Value {
    let disks = Disks::new_with_refreshed_list();
    let list: Vec<Value> = disks
        .list()
        .iter()
        .map(|d| {
            let mount = d.mount_point().to_string_lossy().to_string();
            let name = d.name().to_string_lossy().to_string();
            json!({
                "name": if name.is_empty() { mount.clone() } else { name },
                "label": "",
                "mount": mount,
                "type": "disk",
                "fsType": d.file_system().to_string_lossy(),
                "removable": d.is_removable()
            })
        })
        .collect();
    Value::Array(list)
}

fn fs_size() -> Value {
    let disks = Disks::new_with_refreshed_list();
    let list: Vec<Value> = disks
        .list()
        .iter()
        .map(|d| {
            let size = d.total_space();
            let used = size.saturating_sub(d.available_space());
            json!({
                "mount": d.mount_point().to_string_lossy(),
                "size": size, "used": used,
                "use": if size == 0 { 0.0 } else { used as f64 / size as f64 * 100.0 }
            })
        })
        .collect();
    Value::Array(list)
}

// ---------- network ----------
fn network_interfaces() -> Value {
    let mut s = st().lock().unwrap();
    s.nets.refresh(true);
    let mut rows: Vec<(bool, bool, u64, Value)> = vec![];
    for (name, n) in s.nets.iter() {
        let ip4 = n
            .ip_networks()
            .iter()
            .find_map(|ip| if let std::net::IpAddr::V4(v) = ip.addr { Some(v) } else { None });
        let internal = name == "lo" || name.starts_with("Loopback") || ip4.map(|i| i.is_loopback()).unwrap_or(false);
        let up = ip4.map(|i| !i.is_link_local() && !i.is_unspecified()).unwrap_or(false);
        let bytes = n.total_received() + n.total_transmitted();
        rows.push((
            internal,
            up,
            bytes,
            json!({
                "iface": name, "ip4": ip4.map(|i| i.to_string()).unwrap_or_default(),
                "mac": n.mac_address().to_string(), "internal": internal,
                "operstate": if up { "up" } else { "down" }
            }),
        ));
    }
    rows.sort_by(|a, b| (a.0, !a.1, std::cmp::Reverse(a.2)).cmp(&(b.0, !b.1, std::cmp::Reverse(b.2))));
    Value::Array(rows.into_iter().map(|r| r.3).collect())
}

fn network_stats(iface: &str) -> Value {
    let mut s = st().lock().unwrap();
    s.nets.refresh(true);
    let now = Instant::now();
    let mut out = vec![];
    let names: Vec<String> = s.nets.iter().map(|(n, _)| n.clone()).collect();
    for name in names {
        if !iface.is_empty() && iface != "*" && name != iface {
            continue;
        }
        let (rx, tx) = {
            let n = s.nets.iter().find(|(k, _)| **k == name).unwrap().1;
            (n.total_received(), n.total_transmitted())
        };
        let (rx_sec, tx_sec) = match s.prev_net.get(&name) {
            Some((prx, ptx, t)) => {
                let dt = now.duration_since(*t).as_secs_f64().max(0.001);
                ((rx.saturating_sub(*prx)) as f64 / dt, (tx.saturating_sub(*ptx)) as f64 / dt)
            }
            None => (0.0, 0.0),
        };
        s.prev_net.insert(name.clone(), (rx, tx, now));
        out.push(json!({ "iface": name, "rx_bytes": rx, "tx_bytes": tx, "rx_sec": rx_sec, "tx_sec": tx_sec }));
    }
    Value::Array(out)
}

#[allow(dead_code)]
fn strip_port(addr: &str) -> String {
    let a = addr.trim().trim_start_matches('[');
    if let Some(i) = a.rfind("]:") {
        return a[..i].to_string();
    }
    // 1.2.3.4:443 (win/linux) or 1.2.3.4.443 (macOS)
    if a.matches(':').count() == 1 {
        return a.rsplit_once(':').map(|x| x.0.to_string()).unwrap_or_default();
    }
    if a.matches('.').count() == 4 {
        return a.rsplit_once('.').map(|x| x.0.to_string()).unwrap_or_default();
    }
    a.to_string()
}

fn network_connections() -> Value {
    let mut peers: Vec<String> = vec![];
    #[cfg(target_os = "linux")]
    for f in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(t) = std::fs::read_to_string(f) {
            for l in t.lines().skip(1) {
                let c: Vec<&str> = l.split_whitespace().collect();
                if c.len() > 3 && c[3] == "01" {
                    if let Some((hex, _)) = c[2].split_once(':') {
                        if hex.len() == 8 {
                            if let Ok(v) = u32::from_str_radix(hex, 16) {
                                let b = v.to_le_bytes();
                                peers.push(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                            }
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let args: &[&str] = if cfg!(windows) { &["-n", "-p", "tcp"] } else { &["-an", "-p", "tcp"] };
        let text = run_hidden("netstat", args);
        for l in text.lines() {
            let t: Vec<&str> = l.split_whitespace().collect();
            if let Some(i) = t.iter().position(|x| *x == "ESTABLISHED") {
                if i > 0 {
                    peers.push(strip_port(t[i - 1]));
                }
            }
        }
    }
    let list: Vec<Value> = peers.into_iter().map(|p| json!({ "peeraddress": p, "state": "ESTABLISHED" })).collect();
    Value::Array(list)
}
