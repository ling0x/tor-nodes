//! build.rs — download build-time assets once.
//!
//! Assets downloaded:
//!   1. Natural Earth 110m country GeoJSON  → assets/world.geojson
//!   2. MaxMind GeoLite2-City database      → assets/GeoLite2-City.mmdb
//!
//! For the MaxMind database a free licence key is required:
//!   • Sign up at https://www.maxmind.com/en/geolite2/signup
//!   • Export MAXMIND_LICENSE_KEY=<your_key> then run `cargo build`
//!   • Once assets/GeoLite2-City.mmdb exists the key is no longer needed.

use std::{env, fs, io::Read, path::Path};

const GEOJSON_URL: &str =
    "https://raw.githubusercontent.com/datasets/geo-countries/master/data/countries.geojson";
const GEOJSON_PATH: &str = "assets/world.geojson";
const MMDB_PATH:    &str = "assets/GeoLite2-City.mmdb";
const MMDB_URL_TMPL: &str =
    "https://download.maxmind.com/app/geoip_download\
     ?edition_id=GeoLite2-City&license_key={KEY}&suffix=tar.gz";

fn main() {
    // Re-run whenever the key changes OR either asset file changes/appears.
    println!("cargo:rerun-if-env-changed=MAXMIND_LICENSE_KEY");
    println!("cargo:rerun-if-changed={GEOJSON_PATH}");
    println!("cargo:rerun-if-changed={MMDB_PATH}");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let assets = Path::new(&manifest_dir).join("assets");
    fs::create_dir_all(&assets).expect("could not create assets/ directory");

    // ── 1. GeoJSON ────────────────────────────────────────────────────────
    let geojson_dest = assets.join("world.geojson");
    if geojson_dest.exists() {
        eprintln!("[build] world.geojson already present, skipping.");
    } else {
        eprintln!("[build] Downloading world.geojson ...");
        let body = fetch(GEOJSON_URL);
        fs::write(&geojson_dest, &body).expect("failed to write world.geojson");
        eprintln!("[build] Saved {} bytes → {GEOJSON_PATH}", body.len());
    }

    // ── 2. GeoLite2-City.mmdb ─────────────────────────────────────────────
    let mmdb_dest = assets.join("GeoLite2-City.mmdb");
    if mmdb_dest.exists() {
        eprintln!("[build] GeoLite2-City.mmdb already present, skipping.");
        return;
    }

    let key = match env::var("MAXMIND_LICENSE_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!(
                "[build] ⚠  MAXMIND_LICENSE_KEY is not set."
            );
            eprintln!("[build]    GeoLite2-City fallback will be disabled.");
            eprintln!("[build]    Free sign-up: https://www.maxmind.com/en/geolite2/signup");
            eprintln!("[build]    Then re-run: MAXMIND_LICENSE_KEY=<key> cargo build --release");
            return;
        }
    };

    let url = MMDB_URL_TMPL.replace("{KEY}", &key);
    eprintln!("[build] Downloading GeoLite2-City.tar.gz (this may take a moment) ...");

    let tar_gz = match ureq::get(&url)
        .set("Accept-Encoding", "identity")
        .call()
    {
        Ok(resp) => {
            let mut buf = Vec::new();
            resp.into_reader()
                .read_to_end(&mut buf)
                .expect("failed to read mmdb tar.gz body");
            buf
        }
        Err(e) => {
            eprintln!("[build] ✗ Failed to download GeoLite2-City: {e}");
            eprintln!("[build]   Check that your MAXMIND_LICENSE_KEY is valid.");
            return;
        }
    };

    eprintln!("[build] Downloaded {} bytes, extracting .mmdb ...", tar_gz.len());

    let gz      = flate2::read::GzDecoder::new(tar_gz.as_slice());
    let mut archive = tar::Archive::new(gz);

    for entry in archive.entries().expect("failed to iterate tar entries") {
        let mut entry = entry.expect("bad tar entry");
        let path = entry.path().expect("bad tar path").into_owned();
        if path.extension().map_or(false, |e| e == "mmdb") {
            eprintln!("[build] Extracting {:?} ...", path.file_name().unwrap_or_default());
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("failed to read mmdb bytes");
            fs::write(&mmdb_dest, &buf).expect("failed to write GeoLite2-City.mmdb");
            eprintln!("[build] ✓ Saved {} bytes → {MMDB_PATH}", buf.len());
            return;
        }
    }

    eprintln!("[build] ✗ No .mmdb file found inside the archive — check the download URL format.");
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
