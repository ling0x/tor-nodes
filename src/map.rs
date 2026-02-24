//! world-map — fetch live Tor relay positions from Onionoo and render
//! a self-contained SVG world map coloured by relay type.
//!
//! Fetches Natural Earth 110m GeoJSON at runtime for country polygons.
//! Output: `map.svg`  (equirectangular / plate carrée projection)
//!
//! Dot colours:
//!   purple (#a855f7) — guard
//!   red    (#ef4444) — exit
//!   cyan   (#22d3ee) — middle

use std::{collections::HashMap, fs};
use serde::Deserialize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true";

// Natural Earth 110m country polygons (GeoJSON, ~400 KB)
const GEOJSON_URL: &str =
    "https://raw.githubusercontent.com/holtzy/D3-graph-gallery/master/DATA/world.geojson";

const W: f64 = 1200.0;
const H: f64 = 600.0;
const R: f64 = 2.5;
const OPACITY: f64 = 0.85;

// ---------------------------------------------------------------------------
// Onionoo data model
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OnionooResponse {
    relays: Vec<Relay>,
}

#[derive(Debug, Deserialize)]
struct Relay {
    flags: Vec<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    country: Option<String>,
}

impl Relay {
    fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case(flag))
    }

    fn dot_color(&self) -> &'static str {
        if self.has_flag("guard")     { "#a855f7" }
        else if self.has_flag("exit") { "#ef4444" }
        else                          { "#22d3ee" }
    }
}

// ---------------------------------------------------------------------------
// Projection (equirectangular / plate carrée)
// ---------------------------------------------------------------------------

#[inline]
fn project(lon: f64, lat: f64) -> (f64, f64) {
    ((lon + 180.0) / 360.0 * W, (90.0 - lat) / 180.0 * H)
}

// ---------------------------------------------------------------------------
// GeoJSON polygon → SVG path
// ---------------------------------------------------------------------------

/// Convert a GeoJSON coordinate ring `[[lon,lat], ...]` into an SVG
/// path data string using absolute M/L commands and a closing Z.
fn ring_to_path(coords: &[Value]) -> String {
    let mut d = String::new();
    for (i, pt) in coords.iter().enumerate() {
        let arr = match pt.as_array() { Some(a) => a, None => continue };
        let lon = match arr.get(0).and_then(|v| v.as_f64()) { Some(v) => v, None => continue };
        let lat = match arr.get(1).and_then(|v| v.as_f64()) { Some(v) => v, None => continue };
        let (x, y) = project(lon, lat);
        if i == 0 {
            d.push_str(&format!("M{x:.2},{y:.2}"));
        } else {
            d.push_str(&format!("L{x:.2},{y:.2}"));
        }
    }
    d.push('Z');
    d
}

/// Walk a GeoJSON geometry and collect all SVG path `d` strings.
fn geometry_paths(geom: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    let geo_type = geom["type"].as_str().unwrap_or("");
    let coords   = &geom["coordinates"];

    match geo_type {
        "Polygon" => {
            // coords = [ outer_ring, ...hole_rings ]
            if let Some(rings) = coords.as_array() {
                for ring in rings {
                    if let Some(pts) = ring.as_array() {
                        paths.push(ring_to_path(pts));
                    }
                }
            }
        }
        "MultiPolygon" => {
            // coords = [ [ [ [lon,lat], ... ], ... ], ... ]
            if let Some(polys) = coords.as_array() {
                for poly in polys {
                    if let Some(rings) = poly.as_array() {
                        for ring in rings {
                            if let Some(pts) = ring.as_array() {
                                paths.push(ring_to_path(pts));
                            }
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
    let mut s = String::with_capacity(4 << 20); // 4 MB

    // --- header ---
    s.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
  <title>Tor Relay World Map</title>
  <desc>Live Tor relay positions. Guards: purple, Exits: red, Middles: cyan.</desc>
"#
    ));

    // --- background ---
    s.push_str(&format!("  <rect width='{W}' height='{H}' fill='#0f172a'/>\n"));

    // --- graticule ---
    s.push_str("  <g stroke='#1e293b' stroke-width='0.4'>\n");
    for lon in (-180..=180).step_by(30) {
        let (x, _) = project(lon as f64, 0.0);
        s.push_str(&format!("    <line x1='{x:.1}' y1='0' x2='{x:.1}' y2='{H}'/>\n"));
    }
    for lat in (-90..=90).step_by(30) {
        let (_, y) = project(0.0, lat as f64);
        s.push_str(&format!("    <line x1='0' y1='{y:.1}' x2='{W}' y2='{y:.1}'/>\n"));
    }
    s.push_str("  </g>\n");

    // --- country polygons ---
    s.push_str("  <g fill='#1e3a5f' stroke='#334155' stroke-width='0.4'>\n");
    if let Some(features) = geojson["features"].as_array() {
        for feature in features {
            let geometry = &feature["geometry"];
            for path_d in geometry_paths(geometry) {
                s.push_str(&format!("    <path d='{path_d}'/>\n"));
            }
        }
    }
    s.push_str("  </g>\n");

    // --- relay dots (middles first, then guards/exits on top) ---
    s.push_str("  <g>\n");
    for pass in [false, true] {
        for relay in relays {
            let notable = relay.has_flag("guard") || relay.has_flag("exit");
            if notable != pass { continue; }
            let (lon, lat) = match (relay.longitude, relay.latitude) {
                (Some(lo), Some(la)) => (lo, la),
                _ => continue,
            };
            let (x, y) = project(lon, lat);
            let color  = relay.dot_color();
            s.push_str(&format!(
                "    <circle cx='{x:.1}' cy='{y:.1}' r='{R}' fill='{color}' opacity='{OPACITY}'/>\n"
            ));
        }
    }
    s.push_str("  </g>\n");

    // --- legend ---
    let legend = [("#22d3ee", "Middle"), ("#a855f7", "Guard"), ("#ef4444", "Exit")];
    let lx = 16.0_f64;
    let mut ly = H - 70.0;
    s.push_str("  <g font-family='monospace' font-size='11' fill='#cbd5e1'>\n");
    for (color, label) in &legend {
        s.push_str(&format!("    <circle cx='{:.1}' cy='{ly:.1}' r='5' fill='{color}'/>\n", lx + 5.0));
        s.push_str(&format!("    <text x='{:.1}' y='{:.1}'>{label}</text>\n", lx + 14.0, ly + 4.0));
        ly += 18.0;
    }
    let total   = relays.len();
    let guards  = relays.iter().filter(|r| r.has_flag("guard")).count();
    let exits   = relays.iter().filter(|r| r.has_flag("exit")).count();
    let middles = total.saturating_sub(guards + exits);
    s.push_str(&format!(
        "    <text x='{lx:.1}' y='{:.1}' fill='#64748b'>total: {total}  guards: {guards}  exits: {exits}  middles: {middles}</text>\n",
        H - 10.0
    ));
    s.push_str("  </g>\n");

    // --- top-10 countries ---
    let counts = country_counts(relays);
    let cx = W - 90.0;
    let mut cy = 20.0_f64;
    s.push_str("  <g font-family='monospace' font-size='10' fill='#94a3b8'>\n");
    s.push_str(&format!("    <text x='{cx:.1}' y='{cy:.1}' font-size='11' fill='#cbd5e1'>Top countries</text>\n"));
    cy += 14.0;
    for (cc, count) in counts.iter().take(10) {
        s.push_str(&format!("    <text x='{cx:.1}' y='{cy:.1}'>{cc}  {count}</text>\n"));
        cy += 12.0;
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
