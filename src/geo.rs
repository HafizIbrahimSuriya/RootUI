//! GeoIP lookups for the globe. Uses a GeoLite2-City database (downloaded like the original
//! `geolite2-redist` did) and falls back to a public lookup API when the DB is unavailable.
use maxminddb::{geoip2, Reader};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

type Coord = Option<(f64, f64)>;

pub struct Geo {
    reader: RwLock<Option<Reader<Vec<u8>>>>,
    cache: Mutex<HashMap<String, (Instant, Coord)>>,
    dir: PathBuf,
}

const DB_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/GitSquared/node-geolite2-redist/master/redist/GeoLite2-City.tar.gz",
    "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-City.mmdb",
];

impl Geo {
    pub fn new(user_dir: &std::path::Path) -> Geo {
        Geo { reader: RwLock::new(None), cache: Mutex::new(HashMap::new()), dir: user_dir.join("geoIPcache") }
    }

    fn db_path(&self) -> PathBuf {
        self.dir.join("GeoLite2-City.mmdb")
    }

    fn load(&self) -> bool {
        match Reader::open_readfile(self.db_path()) {
            Ok(r) => {
                *self.reader.write().unwrap() = Some(r);
                true
            }
            Err(_) => false,
        }
    }

    /// Blocking: make sure a DB is present (download when missing / older than 7 days).
    pub fn init(&self) {
        if self.reader.read().unwrap().is_some() {
            return;
        }
        let p = self.db_path();
        let fresh = std::fs::metadata(&p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .map(|e| e < Duration::from_secs(7 * 86400))
            .unwrap_or(false);
        if fresh && self.load() {
            return;
        }
        let _ = std::fs::create_dir_all(&self.dir);
        for url in DB_URLS {
            if let Some(bytes) = download(url) {
                let data = if url.ends_with(".tar.gz") { extract_mmdb(&bytes) } else { Some(bytes) };
                if let Some(d) = data {
                    if d.len() > 1_000_000 && std::fs::write(&p, &d).is_ok() && self.load() {
                        println!("[info] GeoIP database ready.");
                        return;
                    }
                }
            }
        }
        if !self.load() {
            println!("[warning] GeoIP database unavailable; using online lookups for the globe.");
        }
    }

    pub fn lookup(&self, ip: &str) -> Coord {
        let addr: IpAddr = ip.trim().parse().ok()?;
        if is_private(&addr) {
            return None;
        }
        if let Some(r) = self.reader.read().unwrap().as_ref() {
            if let Ok(Some(c)) = r.lookup::<geoip2::City>(addr) {
                if let Some(loc) = c.location {
                    if let (Some(la), Some(lo)) = (loc.latitude, loc.longitude) {
                        return Some((la, lo));
                    }
                }
            }
            return None;
        }
        // Online fallback, cached; unknown entries are fetched in the background.
        {
            let cache = self.cache.lock().unwrap();
            if let Some((t, v)) = cache.get(ip) {
                if t.elapsed() < Duration::from_secs(3600) {
                    return *v;
                }
            }
        }
        None
    }

    /// Blocking online lookup that fills the cache (used when no DB is present).
    pub fn warm(&self, ip: &str) {
        if self.reader.read().unwrap().is_some() {
            return;
        }
        if self.cache.lock().unwrap().contains_key(ip) {
            return;
        }
        // Several free providers, in order; the first one that answers with coordinates wins.
        let res = [format!("https://ipwho.is/{ip}"), format!("https://get.geojs.io/v1/ip/geo/{ip}.json"), format!("https://ipapi.co/{ip}/json/")]
            .iter()
            .find_map(|url| {
                let j = ureq::get(url)
                    .timeout(Duration::from_secs(4))
                    .call()
                    .ok()
                    .and_then(|r| r.into_string().ok())
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())?;
                let num = |k: &str| j.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())));
                Some((num("latitude")?, num("longitude")?))
            });
        self.cache.lock().unwrap().insert(ip.to_string(), (Instant::now(), res));
    }
}

fn is_private(a: &IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v.is_private() || v.is_loopback() || v.is_link_local() || v.is_unspecified() || v.is_broadcast(),
        IpAddr::V6(v) => v.is_loopback() || v.is_unspecified() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80,
    }
}

fn download(url: &str) -> Option<Vec<u8>> {
    let resp = ureq::get(url).timeout(Duration::from_secs(60)).call().ok()?;
    let mut buf = vec![];
    use std::io::Read;
    resp.into_reader().take(200_000_000).read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn extract_mmdb(targz: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let gz = flate2::read::GzDecoder::new(targz);
    let mut ar = tar::Archive::new(gz);
    for e in ar.entries().ok()? {
        let mut e = e.ok()?;
        if e.path().ok()?.to_string_lossy().ends_with(".mmdb") {
            let mut v = vec![];
            e.read_to_end(&mut v).ok()?;
            return Some(v);
        }
    }
    None
}

pub fn external_ip() -> Option<String> {
    for url in [
        "https://api.ipify.org?format=json",
        "https://ipwho.is/",
        "https://get.geojs.io/v1/ip.json",
        "https://api64.ipify.org?format=json",
        "https://myexternalip.com/json",
    ] {
        if let Ok(r) = ureq::get(url).timeout(Duration::from_secs(4)).call() {
            if let Ok(t) = r.into_string() {
                if t.contains("\"ip\"") {
                    return Some(t);
                }
            }
        }
    }
    None
}
