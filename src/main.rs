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
    /// Returns CSV rows for every OR address:
    /// `<fingerprint>, <ip>, <port>`
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
    // IPv6 addresses come wrapped in brackets: [dead:beef::1]:443
    if addr.starts_with('[') {
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

/// The top-level JSON object returned by the Onionoo API.
/// We only care about the `relays` array; everything else is ignored.
#[derive(Debug, Deserialize)]
struct OnionooResponse {
    relays: Vec<TorNode>,
}

fn main() -> anyhow::Result<()> {
    eprintln!("[*] Fetching relay list from Onionoo…");
    let body = ureq::get(ONIONOO_URL).call()?.into_string()?;

    eprintln!("[*] Parsing JSON…");
    let response: OnionooResponse = serde_json::from_str(&body)?;
    let nodes = response.relays;
    eprintln!("[*] Got {} relays.", nodes.len());

    // Write to temp files then atomically rename.
    let all_tmp = format!("{}.tmp", ALL_CSV);
    let guards_tmp = format!("{}.tmp", GUARDS_CSV);
    let exits_tmp = format!("{}.tmp", EXITS_CSV);

    {
        let mut all_w = csv_writer(&all_tmp)?;
        let mut guards_w = csv_writer(&guards_tmp)?;
        let mut exits_w = csv_writer(&exits_tmp)?;

        for node in &nodes {
            let rows = node.to_csv_rows();
            for row in &rows {
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

    fs::rename(&all_tmp, ALL_CSV)?;
    fs::rename(&guards_tmp, GUARDS_CSV)?;
    fs::rename(&exits_tmp, EXITS_CSV)?;

    eprintln!("[*] Done — wrote {}, {}, {}.", ALL_CSV, GUARDS_CSV, EXITS_CSV);
    Ok(())
}

fn csv_writer(path: &str) -> anyhow::Result<BufWriter<File>> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    writeln!(w, "{}", CSV_HEADER)?;
    Ok(w)
}
