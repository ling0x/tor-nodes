//! build.rs — download build-time assets once.
//!
//! Assets downloaded:
//!   1. Natural Earth 110m country GeoJSON  → assets/world.geojson
//!   2. MaxMind GeoLite2-City database      → assets/GeoLite2-City.mmdb
//!
//! For the GeoJSON no credentials are required.
//!
//! For the MaxMind database a free licence key is required:
//!   • Sign up at https://www.maxmind.com/en/geolite2/signup
//!   • Set the environment variable MAXMIND_LICENSE_KEY before building:
//!       MAXMIND_LICENSE_KEY=YOUR_KEY cargo build
//!   • Once the .mmdb is committed you can build without the key.

use std::{env, fs, io::Read, path::Path};

const GEOJSON_URL: &str =
    "https://raw.githubusercontent.com/datasets/geo-countries/master/data/countries.geojson";
const GEOJSON_PATH: &str = "assets/world.geojson";

const MMDB_PATH: &str = "assets/GeoLite2-City.mmdb";
// MaxMind permalink — substitutes {KEY} at runtime.
const MMDB_URL_TMPL: &str =
    "https://download.maxmind.com/app/geoip_download\
     ?edition_id=GeoLite2-City&license_key={KEY}&suffix=tar.gz";

fn main() {
    println!("cargo:rerun-if-changed={GEOJSON_PATH}");
    println!("cargo:rerun-if-changed={MMDB_PATH}");
    println!("cargo:rerun-if-env-changed=MAXMIND_LICENSE_KEY");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let assets = Path::new(&manifest_dir).join("assets");
    fs::create_dir_all(&assets).expect("could not create assets/ directory");

    // --- 1. GeoJSON ---
    let geojson_dest = assets.join("world.geojson");
    if geojson_dest.exists() {
        eprintln!("[build] {GEOJSON_PATH} already exists, skipping.");
    } else {
        eprintln!("[build] Downloading {GEOJSON_URL} ...");
        let body = fetch(GEOJSON_URL);
        fs::write(&geojson_dest, &body).expect("failed to write world.geojson");
        eprintln!("[build] Saved {} bytes to {GEOJSON_PATH}", body.len());
    }

    // --- 2. GeoLite2-City.mmdb ---
    let mmdb_dest = assets.join("GeoLite2-City.mmdb");
    if mmdb_dest.exists() {
        eprintln!("[build] {MMDB_PATH} already exists, skipping.");
        return;
    }

    let key = match env::var("MAXMIND_LICENSE_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!(
                "[build] MAXMIND_LICENSE_KEY not set — skipping GeoLite2-City download.\n\
                 [build] Set the variable and rebuild, or place the .mmdb file at {MMDB_PATH} manually.\n\
                 [build] Free sign-up: https://www.maxmind.com/en/geolite2/signup"
            );
            return;
        }
    };

    let url = MMDB_URL_TMPL.replace("{KEY}", &key);
    eprintln!("[build] Downloading GeoLite2-City.tar.gz ...");
    let tar_gz = fetch(&url);

    // Decompress the tar.gz in-memory and extract the .mmdb file.
    let gz   = flate2::read::GzDecoder::new(tar_gz.as_slice());
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries().expect("failed to read tar entries") {
        let mut entry = entry.expect("bad tar entry");
        let entry_path = entry.path().expect("bad tar path").into_owned();
        if entry_path.extension().map(|e| e == "mmdb").unwrap_or(false) {
            eprintln!("[build] Extracting {:?} → {MMDB_PATH}", entry_path);
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("failed to read mmdb from tar");
            fs::write(&mmdb_dest, &buf).expect("failed to write GeoLite2-City.mmdb");
            eprintln!("[build] Saved {} bytes to {MMDB_PATH}", buf.len());
            return;
        }
    }
    eprintln!("[build] WARNING: .mmdb not found inside the downloaded archive.");
}

fn fetch(url: &str) -> Vec<u8> {
    let resp = ureq::get(url)
        .set("Accept-Encoding", "identity")
        .call()
        .unwrap_or_else(|e| panic!("GET {url} failed: {e}"));
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .expect("failed to read response body");
    buf
}
