use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    net::IpAddr,
    str::FromStr,
};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true";

const CSV_HEADER: &str = "fingerprint,ipaddr,port";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OnionooResponse {
    relays: Vec<TorNode>,
}

#[derive(Debug, Deserialize)]
struct TorNode {
    fingerprint: String,
    or_addresses: Vec<String>,
    flags: Vec<String>,
}

impl TorNode {
    fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f.eq_ignore_ascii_case(flag))
    }

    /// Yields one CSV row per OR address: `fingerprint,ipaddr,port`
    /// No spaces — compliant with RFC 4180 / Wikipedia CSV basic rules.
    fn csv_rows(&self) -> impl Iterator<Item = String> + '_ {
        self.or_addresses
            .iter()
            .filter_map(|addr| parse_or_address(addr))
            .map(|(ip, port)| format!("{},{},{}", self.fingerprint, ip, port))
    }
}

// ---------------------------------------------------------------------------
// Address parsing
// ---------------------------------------------------------------------------

/// Parse an Onionoo OR-address string into `(IpAddr, port)`.
///
/// Onionoo uses two formats:
///   IPv4 — `"1.2.3.4:9001"`
///   IPv6 — `"[dead:beef::1]:443"`
fn parse_or_address(addr: &str) -> Option<(IpAddr, u16)> {
    if let Some(addr) = addr.strip_prefix('[') {
        // IPv6
        let (ip_str, rest) = addr.split_once(']')?;
        let port_str = rest.strip_prefix(':')?;
        Some((IpAddr::from_str(ip_str).ok()?, port_str.parse().ok()?))
    } else {
        // IPv4
        let (ip_str, port_str) = addr.rsplit_once(':')?;
        Some((IpAddr::from_str(ip_str).ok()?, port_str.parse().ok()?))
    }
}

// ---------------------------------------------------------------------------
// CSV output
// ---------------------------------------------------------------------------

struct CsvOutput {
    path: &'static str,
    tmp_path: String,
    writer: BufWriter<File>,
}

impl CsvOutput {
    fn create(path: &'static str) -> anyhow::Result<Self> {
        let tmp_path = format!("{path}.tmp");
        let mut writer = BufWriter::new(File::create(&tmp_path)?);
        writeln!(writer, "{CSV_HEADER}")?;
        Ok(Self { path, tmp_path, writer })
    }

    fn write_row(&mut self, row: &str) -> anyhow::Result<()> {
        writeln!(self.writer, "{row}")?;
        Ok(())
    }

    fn finalise(mut self) -> anyhow::Result<()> {
        self.writer.flush()?;
        fs::rename(&self.tmp_path, self.path)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    eprintln!("[*] Fetching relay list from Onionoo...");
    let response = ureq::get(ONIONOO_URL).call()?;
    let parsed: OnionooResponse = serde_json::from_reader(response.into_reader())?;
    let nodes = parsed.relays;
    eprintln!("[*] Got {} relays.", nodes.len());

    let mut all    = CsvOutput::create("all.csv")?;
    let mut guards = CsvOutput::create("guards.csv")?;
    let mut exits  = CsvOutput::create("exits.csv")?;

    for node in &nodes {
        let is_guard = node.has_flag("guard");
        let is_exit  = node.has_flag("exit");

        for row in node.csv_rows() {
            all.write_row(&row)?;
            if is_guard { guards.write_row(&row)?; }
            if is_exit  { exits.write_row(&row)?;  }
        }
    }

    all.finalise()?;
    guards.finalise()?;
    exits.finalise()?;

    eprintln!("[*] Done - wrote all.csv, guards.csv, exits.csv.");
    Ok(())
}
