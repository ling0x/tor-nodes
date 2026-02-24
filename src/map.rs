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
    fmt::Write as FmtWrite,
    fs,
};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true";

/// SVG canvas dimensions (px).
const W: f64 = 1200.0;
const H: f64 = 600.0;

/// Dot radius per relay (px).
const R: f64 = 2.2;

/// Dot opacity — overlapping dots stay visible but don't fully saturate.
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
    /// Decimal latitude — present on most relays, absent on a small minority.
    latitude: Option<f64>,
    /// Decimal longitude.
    longitude: Option<f64>,
    /// ISO 3166-1 alpha-2 country code (lower-case).
    country: Option<String>,
}

impl Relay {
    fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case(flag))
    }

    fn dot_color(&self) -> &'static str {
        if self.has_flag("guard") {
            "#a855f7" // purple
        } else if self.has_flag("exit") {
            "#ef4444" // red
        } else {
            "#22d3ee" // cyan — middle
        }
    }
}

// ---------------------------------------------------------------------------
// Projection  (equirectangular / plate carrée)
// ---------------------------------------------------------------------------

/// Map (lon, lat) in degrees to SVG (x, y) pixel coordinates.
///
/// lon ∈ [-180, 180] → x ∈ [0, W]
/// lat ∈ [ -90,  90] → y ∈ [H, 0]  (SVG y-axis is inverted)
fn project(lon: f64, lat: f64) -> (f64, f64) {
    let x = (lon + 180.0) / 360.0 * W;
    let y = (90.0 - lat) / 180.0 * H;
    (x, y)
}

// ---------------------------------------------------------------------------
// Country relay counts for tooltip / legend
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

fn render_svg(relays: &[Relay]) -> String {
    let mut svg = String::with_capacity(1 << 20); // 1 MB initial

    // ---- header ------------------------------------------------------------
    writeln!(
        svg,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     width="{W}" height="{H}"
     viewBox="0 0 {W} {H}">
  <title>Tor Relay World Map</title>
  <desc>Live Tor relay positions from Onionoo. Guards: purple, Exits: red, Middles: cyan.</desc>"""
    )
    .unwrap();

    // ---- background --------------------------------------------------------
    writeln!(
        svg,
        "  <rect width=\"{W}\" height=\"{H}\" fill=\"#0f172a\"/>"
    )
    .unwrap();

    // ---- graticule (grid lines every 30°) ----------------------------------
    svg.push_str("  <g stroke=\"#1e293b\" stroke-width=\"0.5\">\n");
    for lon in (-180..=180).step_by(30) {
        let (x, _) = project(lon as f64, 0.0);
        writeln!(svg, "    <line x1=\"{x:.1}\" y1=\"0\" x2=\"{x:.1}\" y2=\"{H}\"/>").unwrap();
    }
    for lat in (-90..=90).step_by(30) {
        let (_, y) = project(0.0, lat as f64);
        writeln!(svg, "    <line x1=\"0\" y1=\"{y:.1}\" x2=\"{W}\" y2=\"{y:.1}\"/>").unwrap();
    }
    svg.push_str("  </g>\n");

    // ---- relay dots --------------------------------------------------------
    svg.push_str("  <g>\n");
    // Draw middles first so guards/exits render on top.
    for pass in [false, true] {
        for relay in relays {
            let is_notable = relay.has_flag("guard") || relay.has_flag("exit");
            if is_notable != pass {
                continue;
            }
            let (lon, lat) = match (relay.longitude, relay.latitude) {
                (Some(lo), Some(la)) => (lo, la),
                _ => continue,
            };
            let (x, y) = project(lon, lat);
            let color = relay.dot_color();
            writeln!(
                svg,
                "    <circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{R}\" fill=\"{color}\" opacity=\"{OPACITY}\"/>"
            )
            .unwrap();
        }
    }
    svg.push_str("  </g>\n");

    // ---- legend ------------------------------------------------------------
    let legend = [
        ("#22d3ee", "Middle"),
        ("#a855f7", "Guard"),
        ("#ef4444", "Exit"),
    ];
    let lx = 16.0_f64;
    let mut ly = H - 70.0;
    svg.push_str("  <g font-family=\"monospace\" font-size=\"11\" fill=\"#cbd5e1\">\n");
    for (color, label) in &legend {
        writeln!(
            svg,
            "    <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"5\" fill=\"{color}\"/>",
            lx + 5.0,
            ly
        )
        .unwrap();
        writeln!(
            svg,
            "    <text x=\"{:.1}\" y=\"{:.1}\">{label}</text>",
            lx + 14.0,
            ly + 4.0
        )
        .unwrap();
        ly += 18.0;
    }

    // relay counts
    let total   = relays.len();
    let guards  = relays.iter().filter(|r| r.has_flag("guard")).count();
    let exits   = relays.iter().filter(|r| r.has_flag("exit")).count();
    let middles = total - guards - exits;
    writeln!(
        svg,
        "    <text x=\"{:.1}\" y=\"{:.1}\" fill=\"#64748b\">total: {total}  guards: {guards}  exits: {exits}  middles: {middles}</text>",
        lx,
        H - 10.0
    )
    .unwrap();
    svg.push_str("  </g>\n");

    // ---- top-10 countries sidebar -----------------------------------------
    let counts = country_counts(relays);
    let cx = W - 90.0;
    let mut cy = 20.0_f64;
    svg.push_str("  <g font-family=\"monospace\" font-size=\"10\" fill=\"#94a3b8\">\n");
    writeln!(svg, "    <text x=\"{cx:.1}\" y=\"{cy:.1}\" font-size=\"11\" fill=\"#cbd5e1\">Top countries</text>").unwrap();
    cy += 14.0;
    for (cc, count) in counts.iter().take(10) {
        writeln!(svg, "    <text x=\"{cx:.1}\" y=\"{cy:.1}\">{cc}  {count}</text>").unwrap();
        cy += 12.0;
    }
    svg.push_str("  </g>\n");

    // ---- footer ------------------------------------------------------------
    svg.push_str("</svg>\n");
    svg
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
