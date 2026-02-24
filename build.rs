//! build.rs — download Natural Earth 110m country GeoJSON once at build time.
//!
//! The file is saved to `assets/world.geojson` inside the repo so it is
//! committed and subsequent builds are instant (no network needed).
//! If the file already exists it is NOT re-downloaded.

use std::{env, fs, path::Path};

const GEOJSON_URL: &str =
    "https://raw.githubusercontent.com/datasets/geo-countries/master/data/countries.geojson";

const LOCAL_PATH: &str = "assets/world.geojson";

fn main() {
    // Tell Cargo to re-run this script only when the asset is missing.
    println!("cargo:rerun-if-changed={LOCAL_PATH}");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dest = Path::new(&manifest_dir).join(LOCAL_PATH);

    if dest.exists() {
        eprintln!("[build] {LOCAL_PATH} already exists, skipping download.");
        return;
    }

    fs::create_dir_all(dest.parent().unwrap())
        .expect("could not create assets/ directory");

    eprintln!("[build] Downloading {GEOJSON_URL} ...");
    let resp = ureq::get(GEOJSON_URL)
        .set("Accept-Encoding", "identity")
        .call()
        .expect("failed to download world.geojson");

    let mut body = String::new();
    resp.into_reader()
        .read_to_string(&mut body)
        .expect("failed to read world.geojson response");

    fs::write(&dest, &body).expect("failed to write assets/world.geojson");
    eprintln!("[build] Saved {} bytes to {LOCAL_PATH}", body.len());
}
