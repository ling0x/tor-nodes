//! geo.rs — thin MaxMind GeoLite2-City wrapper.

use std::{net::IpAddr, path::Path, sync::OnceLock};
use maxminddb::{geoip2, Reader};

static DB: OnceLock<Option<Reader<Vec<u8>>>> = OnceLock::new();

const MMDB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/GeoLite2-City.mmdb");

fn db() -> Option<&'static Reader<Vec<u8>>> {
    DB.get_or_init(|| {
        if !Path::new(MMDB_PATH).exists() {
            eprintln!("[geo] GeoLite2-City.mmdb not found at {MMDB_PATH} — geo-fallback disabled.");
            return None;
        }
        match Reader::open_readfile(MMDB_PATH) {
            Ok(r)  => { eprintln!("[geo] Opened {MMDB_PATH}"); Some(r) }
            Err(e) => { eprintln!("[geo] Failed to open mmdb: {e}"); None }
        }
    })
    .as_ref()
}

/// Returns `Some((lat, lon))` or `None` if unavailable.
pub fn lookup(ip: IpAddr) -> Option<(f64, f64)> {
    let reader = db()?;
    let result = reader.lookup(ip).ok()?;
    let city: geoip2::City = result.decode().ok()??;
    let lat = city.location.latitude?;
    let lon = city.location.longitude?;
    Some((lat, lon))
}
