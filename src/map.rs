//! world-map — fetch live Tor relay positions from Onionoo and render
//! a self-contained SVG world map coloured by relay type.
//!
//! Output: `map.svg`  (equirectangular / plate carrée projection)
//!
//! Dot colours:
//!   purple  (#a855f7) — guard
//!   red     (#ef4444) — exit
//!   cyan    (#22d3ee) — middle (neither guard nor exit)
//!
//! Guard+exit relays are coloured as guards (guard takes priority).

use std::{
    collections::HashMap,
    fs,
};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true";

const W: f64 = 1200.0;
const H: f64 = 600.0;
const R: f64 = 2.2;
const OPACITY: f64 = 0.75;

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
        if self.has_flag("guard")      { "#a855f7" }
        else if self.has_flag("exit")  { "#ef4444" }
        else                           { "#22d3ee" }
    }
}

// ---------------------------------------------------------------------------
// Projection (equirectangular)
// ---------------------------------------------------------------------------

fn project(lon: f64, lat: f64) -> (f64, f64) {
    let x = (lon + 180.0) / 360.0 * W;
    let y = (90.0  - lat) / 180.0 * H;
    (x, y)
}

// ---------------------------------------------------------------------------
// Country counts
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
// SVG helpers — every attribute value goes through these so there is
// no manual quote escaping anywhere in the rendering code.
// ---------------------------------------------------------------------------

/// Append a self-closing SVG element with pre-formatted attributes.
macro_rules! elem {
    ($buf:expr, $tag:expr, $attrs:expr) => {
        $buf.push_str(&format!("  <{} {}/>\n", $tag, $attrs));
    };
}

/// Append a full SVG element with inner text.
macro_rules! elem_text {
    ($buf:expr, $tag:expr, $attrs:expr, $inner:expr) => {
        $buf.push_str(&format!("  <{0} {1}>{2}</{0}>\n", $tag, $attrs, $inner));
    };
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(relays: &[Relay]) -> String {
    let mut s = String::with_capacity(1 << 20);

    // header
    s.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
  <title>Tor Relay World Map</title>
  <desc>Live Tor relay positions. Guards: purple, Exits: red, Middles: cyan.</desc>
"#
    ));

    // background
    elem!(s, "rect", format!("width='{W}' height='{H}' fill='#0f172a'"));

    // graticule
    s.push_str("  <g stroke='#1e293b' stroke-width='0.5'>\n");
    for lon in (-180..=180).step_by(30) {
        let (x, _) = project(lon as f64, 0.0);
        s.push_str(&format!("    <line x1='{x:.1}' y1='0' x2='{x:.1}' y2='{H}'/>\n"));
    }
    for lat in (-90..=90).step_by(30) {
        let (_, y) = project(0.0, lat as f64);
        s.push_str(&format!("    <line x1='0' y1='{y:.1}' x2='{W}' y2='{y:.1}'/>\n"));
    }
    s.push_str("  </g>\n");

    // relay dots — middles first, then guards/exits on top
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

    // legend
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

    // top-10 countries
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
    eprintln!("[*] Fetching relay list from Onionoo...");
    let response = ureq::get(ONIONOO_URL).call()?;
    let parsed: OnionooResponse = serde_json::from_reader(response.into_reader())?;
    let relays = parsed.relays;
    eprintln!("[*] Got {} relays.", relays.len());

    let svg = render_svg(&relays);
    fs::write("map.svg", &svg)?;
    eprintln!("[*] Written map.svg ({} bytes)", svg.len());
    Ok(())
}
