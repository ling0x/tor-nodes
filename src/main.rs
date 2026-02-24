use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    net::IpAddr,
    str::FromStr,
};

use serde::Deserialize;

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true";

const ALL_CSV: &str = "all.csv";
const GUARDS_CSV: &str = "guards.csv";
const EXITS_CSV: &str = "exits.csv";

const CSV_HEADER: &str = "fingerprint, ipaddr, port";

#[derive(Debug, Deserialize)]
struct TorNode {
    fingerprint: String,
    or_addresses: Vec<String>,
    flags: Vec<String>,
}

impl TorNode {
    fn to_csv_rows(&self) -> Vec<String> {
        self.or_addresses
            .iter()
            .filter_map(|addr| parse_or_address(addr))
            .map(|(ip, port)| format!("{}, {}, {}", self.fingerprint, ip, port))
            .collect()
    }

    fn is_guard(&self) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case("guard"))
    }

    fn is_exit(&self) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case("exit"))
    }
}

/// Parse an OR address string like `"1.2.3.4:9001"` or `"[::1]:9001"`
/// into `(IpAddr, u16)`.
fn parse_or_address(addr: &str) -> Option<(IpAddr, u16)> {
    if addr.starts_with('[') {
        // IPv6: [dead:beef::1]:443
        let close = addr.find(']')?;
        let ip_str = &addr[1..close];
        let rest = &addr[close + 1..];
        let port_str = rest.strip_prefix(':')?.trim();
        let ip = IpAddr::from_str(ip_str).ok()?;
        let port = port_str.parse::<u16>().ok()?;
        Some((ip, port))
    } else {
        // IPv4: 1.2.3.4:9001
        let mut parts = addr.rsplitn(2, ':');
        let port_str = parts.next()?.trim();
        let ip_str = parts.next()?.trim();
        let ip = IpAddr::from_str(ip_str).ok()?;
        let port = port_str.parse::<u16>().ok()?;
        Some((ip, port))
    }
}

/// Top-level Onionoo response — only `relays` is needed.
#[derive(Debug, Deserialize)]
struct OnionooResponse {
    relays: Vec<TorNode>,
}

fn main() -> anyhow::Result<()> {
    eprintln!("[*] Fetching relay list from Onionoo...");

    // Stream the response body directly into serde_json so we never
    // materialise the entire payload as a String (avoids ureq's
    // default 10 MB into_string() cap on a ~15 MB response).
    let response = ureq::get(ONIONOO_URL).call()?;
    let reader = response.into_reader();
    let parsed: OnionooResponse = serde_json::from_reader(reader)?;
    let nodes = parsed.relays;

    eprintln!("[*] Got {} relays.", nodes.len());

    // Write to .tmp files first, then atomically rename.
    let all_tmp     = format!("{}.tmp", ALL_CSV);
    let guards_tmp  = format!("{}.tmp", GUARDS_CSV);
    let exits_tmp   = format!("{}.tmp", EXITS_CSV);

    {
        let mut all_w    = csv_writer(&all_tmp)?;
        let mut guards_w = csv_writer(&guards_tmp)?;
        let mut exits_w  = csv_writer(&exits_tmp)?;

        for node in &nodes {
            for row in node.to_csv_rows() {
                writeln!(all_w, "{}", row)?;
                if node.is_guard() {
                    writeln!(guards_w, "{}", row)?;
                }
                if node.is_exit() {
                    writeln!(exits_w, "{}", row)?;
                }
            }
        }

        all_w.flush()?;
        guards_w.flush()?;
        exits_w.flush()?;
    }

    fs::rename(&all_tmp,    ALL_CSV)?;
    fs::rename(&guards_tmp, GUARDS_CSV)?;
    fs::rename(&exits_tmp,  EXITS_CSV)?;

    eprintln!("[*] Done - wrote {}, {}, {}.", ALL_CSV, GUARDS_CSV, EXITS_CSV);
    Ok(())
}

fn csv_writer(path: &str) -> anyhow::Result<BufWriter<File>> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    writeln!(w, "{}", CSV_HEADER)?;
    Ok(w)
}
