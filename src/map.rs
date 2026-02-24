//! world-map — fetch live Tor relay positions from Onionoo and render
//! a self-contained SVG world map coloured by relay type.
//!
//! Fetches Natural Earth 110m GeoJSON at runtime for country polygons.
//! Output: `map.svg`  (equirectangular / plate carrée projection)
//!
//! Dot colours:
//!   purple (#a855f7) — guard
//!   red    (#ef4444) — exit
//!   yellow (#facc15) — middle

use std::{collections::HashMap, fs};
use serde::Deserialize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true";

const GEOJSON_URL: &str =
    "https://raw.githubusercontent.com/holtzy/D3-graph-gallery/master/DATA/world.geojson";

const W: f64 = 1200.0;
const H: f64 = 600.0;

// Dot sizes: notable (guard/exit) are larger than middles
const R_MIDDLE:  f64 = 3.0;
const R_NOTABLE: f64 = 4.0;

// ---------------------------------------------------------------------------
// Onionoo data model
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OnionooResponse {
    relays: Vec<Relay>,
}

#[derive(Debug, Deserialize)]
struct Relay {
    flags:     Vec<String>,
    latitude:  Option<f64>,
    longitude: Option<f64>,
    country:   Option<String>,
}

impl Relay {
    fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case(flag))
    }

    fn is_guard(&self)  -> bool { self.has_flag("guard") }
    fn is_exit(&self)   -> bool { self.has_flag("exit")  }

    fn dot_color(&self) -> &'static str {
        if   self.is_guard() { "#c084fc" }  // bright purple
        elif self.is_exit()  { "#f87171" }  // bright red
        else                 { "#fde047" }  // bright yellow — high contrast on dark blue land
    }

    fn dot_radius(&self) -> f64 {
        if self.is_guard() || self.is_exit() { R_NOTABLE } else { R_MIDDLE }
    }
}

// ---------------------------------------------------------------------------
// Projection (equirectangular)
// ---------------------------------------------------------------------------

#[inline]
fn project(lon: f64, lat: f64) -> (f64, f64) {
    ((lon + 180.0) / 360.0 * W, (90.0 - lat) / 180.0 * H)
}

// ---------------------------------------------------------------------------
// GeoJSON → SVG paths
// ---------------------------------------------------------------------------

fn ring_to_path(coords: &[Value]) -> String {
    let mut d = String::new();
    for (i, pt) in coords.iter().enumerate() {
        let arr = match pt.as_array() { Some(a) => a, None => continue };
        let lon = match arr.first().and_then(|v| v.as_f64()) { Some(v) => v, None => continue };
        let lat = match arr.get(1).and_then(|v| v.as_f64())  { Some(v) => v, None => continue };
        let (x, y) = project(lon, lat);
        if i == 0 { d.push_str(&format!("M{x:.2},{y:.2}")) }
        else       { d.push_str(&format!("L{x:.2},{y:.2}")) }
    }
    d.push('Z');
    d
}

fn geometry_paths(geom: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    match geom["type"].as_str().unwrap_or("") {
        "Polygon" => {
            if let Some(rings) = geom["coordinates"].as_array() {
                for ring in rings {
                    if let Some(pts) = ring.as_array() { paths.push(ring_to_path(pts)); }
                }
            }
        }
        "MultiPolygon" => {
            if let Some(polys) = geom["coordinates"].as_array() {
                for poly in polys {
                    if let Some(rings) = poly.as_array() {
                        for ring in rings {
                            if let Some(pts) = ring.as_array() { paths.push(ring_to_path(pts)); }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    paths
}

// ---------------------------------------------------------------------------
// Country relay counts
// ---------------------------------------------------------------------------

fn country_counts(relays: &[Relay]) -> Vec<(String, usize)> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for r in relays {
        if let Some(cc) = &r.country {
            *map.entry(cc.to_uppercase()).or_insert(0) += 1;
        }
    }
    let mut counts: Vec<_> = map.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(relays: &[Relay], geojson: &Value) -> String {
    let mut s = String::with_capacity(4 << 20);

    // header
    s.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
  <title>Tor Relay World Map</title>
  <desc>Live Tor relay positions. Guards: purple, Exits: red, Middles: yellow.</desc>
"#
    ));

    // background (ocean)
    s.push_str(&format!("  <rect width='{W}' height='{H}' fill='#0c1a2e'/>\n"));

    // graticule
    s.push_str("  <g stroke='#162032' stroke-width='0.5'>\n");
    for lon in (-180..=180).step_by(30) {
        let (x, _) = project(lon as f64, 0.0);
        s.push_str(&format!("    <line x1='{x:.1}' y1='0' x2='{x:.1}' y2='{H}'/>\n"));
    }
    for lat in (-90..=90).step_by(30) {
        let (_, y) = project(0.0, lat as f64);
        s.push_str(&format!("    <line x1='0' y1='{y:.1}' x2='{W}' y2='{y:.1}'/>\n"));
    }
    s.push_str("  </g>\n");

    // country polygons — noticeably lighter than the ocean so dots are visible on both
    s.push_str("  <g fill='#1d3461' stroke='#2d4a7a' stroke-width='0.5'>\n");
    if let Some(features) = geojson["features"].as_array() {
        for feature in features {
            for d in geometry_paths(&feature["geometry"]) {
                s.push_str(&format!("    <path d='{d}'/>\n"));
            }
        }
    }
    s.push_str("  </g>\n");

    // relay dots
    // Pass 0: middles (bottom layer)
    // Pass 1: guards/exits (top layer, larger)
    s.push_str("  <g stroke='#0c1a2e' stroke-width='0.6'>\n");
    for pass in [false, true] {
        for relay in relays {
            let notable = relay.is_guard() || relay.is_exit();
            if notable != pass { continue; }
            let (lon, lat) = match (relay.longitude, relay.latitude) {
                (Some(lo), Some(la)) => (lo, la),
                _ => continue,
            };
            let (x, y) = project(lon, lat);
            let color  = relay.dot_color();
            let r      = relay.dot_radius();
            s.push_str(&format!(
                "    <circle cx='{x:.1}' cy='{y:.1}' r='{r}' fill='{color}'/>\n"
            ));
        }
    }
    s.push_str("  </g>\n");

    // legend
    let legend = [
        ("#fde047", "Middle"),
        ("#c084fc", "Guard"),
        ("#f87171", "Exit"),
    ];
    let lx = 16.0_f64;
    let mut ly = H - 70.0;
    s.push_str("  <g font-family='monospace' font-size='12' fill='#e2e8f0'>\n");
    for (color, label) in &legend {
        s.push_str(&format!("    <circle cx='{:.1}' cy='{ly:.1}' r='6' fill='{color}' stroke='#0c1a2e' stroke-width='0.8'/>\n", lx + 6.0));
        s.push_str(&format!("    <text x='{:.1}' y='{:.1}'>{label}</text>\n", lx + 16.0, ly + 4.5));
        ly += 20.0;
    }
    let total   = relays.len();
    let guards  = relays.iter().filter(|r| r.is_guard()).count();
    let exits   = relays.iter().filter(|r| r.is_exit()).count();
    let middles = total.saturating_sub(guards + exits);
    s.push_str(&format!(
        "    <text x='{lx:.1}' y='{:.1}' font-size='10' fill='#64748b'>total: {total}  guards: {guards}  exits: {exits}  middles: {middles}</text>\n",
        H - 8.0
    ));
    s.push_str("  </g>\n");

    // top-10 countries
    let counts = country_counts(relays);
    let cx = W - 95.0;
    let mut cy = 20.0_f64;
    s.push_str("  <g font-family='monospace' font-size='10' fill='#94a3b8'>\n");
    s.push_str(&format!("    <text x='{cx:.1}' y='{cy:.1}' font-size='11' fill='#cbd5e1'>Top countries</text>\n"));
    cy += 14.0;
    for (cc, count) in counts.iter().take(10) {
        s.push_str(&format!("    <text x='{cx:.1}' y='{cy:.1}'>{cc}  {count}</text>\n"));
        cy += 13.0;
    }
    s.push_str("  </g>\n");

    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    eprintln!("[*] Fetching country polygons...");
    let geo_resp = ureq::get(GEOJSON_URL).call()?;
    let geojson: Value = serde_json::from_reader(geo_resp.into_reader())?;

    eprintln!("[*] Fetching relay list from Onionoo...");
    let onionoo_resp = ureq::get(ONIONOO_URL).call()?;
    let parsed: OnionooResponse = serde_json::from_reader(onionoo_resp.into_reader())?;
    let relays = parsed.relays;
    eprintln!("[*] Got {} relays.", relays.len());

    let svg = render_svg(&relays, &geojson);
    fs::write("map.svg", &svg)?;
    eprintln!("[*] Written map.svg ({} bytes)", svg.len());
    Ok(())
}
