//! geo.rs — thin MaxMind GeoLite2-City wrapper.
//!
//! Opens `assets/GeoLite2-City.mmdb` (relative to the crate root) and
//! exposes a single function:
//!
//! ```
//! let (lat, lon) = geo::lookup(ip)?;
//! ```
//!
//! Returns `None` if the database is absent or the IP has no record.

use std::{net::IpAddr, path::Path, sync::OnceLock};
use maxminddb::{geoip2, Reader};

static DB: OnceLock<Option<Reader<Vec<u8>>>> = OnceLock::new();

const MMDB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/GeoLite2-City.mmdb");

fn db() -> Option<&'static Reader<Vec<u8>>> {
    DB.get_or_init(|| {
        if !Path::new(MMDB_PATH).exists() {
            eprintln!(
                "[geo] GeoLite2-City.mmdb not found at {MMDB_PATH}.\n\
                 [geo] Set MAXMIND_LICENSE_KEY and rebuild, or place the file there manually.\n\
                 [geo] Geo-fallback will be disabled for this run."
            );
            return None;
        }
        match Reader::open_readfile(MMDB_PATH) {
            Ok(r)  => { eprintln!("[geo] Opened {MMDB_PATH}"); Some(r) }
            Err(e) => { eprintln!("[geo] Failed to open mmdb: {e}"); None }
        }
    })
    .as_ref()
}

/// Look up the latitude and longitude for an IP address.
///
/// Returns `Some((lat, lon))` on success, `None` if the database is
/// unavailable or the IP has no city-level record.
pub fn lookup(ip: IpAddr) -> Option<(f64, f64)> {
    let reader = db()?;
    // maxminddb 0.27+ returns LookupResult — call .record() to get the typed value.
    let result = reader.lookup::<geoip2::City>(ip).ok()?;
    let record = result.record?;
    let loc = record.location.as_ref()?;
    let lat = loc.latitude?;
    let lon = loc.longitude?;
    Some((lat, lon))
}
