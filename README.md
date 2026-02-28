# tor-nodes

![Tor Relay World Map](https://raw.githubusercontent.com/ling0x/tor-nodes/feat/world-map/latest.map.svg)

Fetches the live Tor relay list from the [Onionoo API](https://metrics.torproject.org/onionoo.html) and outputs three CSV files:

| File | Contents |
|------|----------|
| [`latest.all.csv`](latest.all.csv) | Every running relay |
| [`latest.guards.csv`](latest.guards.csv) | Relays with the `Guard` flag |
| [`latest.exits.csv`](latest.exits.csv) | Relays with the `Exit` flag |

Each CSV row has the format:
```
fingerprint,ipaddr,port
```

## How a Tor Circuit Works

Tor routes traffic through a fixed chain of three relays. Each hop only knows its immediate neighbours — no single node can see both the origin and the destination.

**Without a bridge (standard circuit)**
```
Client → Guard → Middle Relay → Exit → Destination
```
The client connects directly to a publicly listed Guard. Its IP is visible to the Guard, but nothing else.

**With a bridge (censored networks)**
```
Client → Bridge → Middle Relay → Exit → Destination
```
The Guard is replaced by a Bridge — an unlisted entry node not published in the public directory. This makes the entry point hard for censors to block, since they cannot enumerate and block what they cannot find.

## Relay Types

**Guard** — The entry hop. Clients connect to it directly (or via a bridge). Guards are stable, high-bandwidth nodes that a client keeps for months to reduce long-term traffic-analysis risk.

**Bridge** — A secret Guard. Functionally identical to a Guard relay but not listed in the public consensus, making it resistant to enumeration-based blocking. Used in place of a Guard on censored networks.

**Middle Relay** — The intermediate hop. Passes encrypted traffic between Guard and Exit without knowing the origin or destination. Any relay that is not a Guard or Exit acts as a middle.

**Exit** — The final hop. Strips the last layer of onion encryption and makes the actual TCP connection to the destination on the user's behalf. The only node that sees the destination — but not the user's identity.

## Usage

```bash
cargo run --release
```

Outputs `all.csv`, `guards.csv`, and `exits.csv` in the current directory.

## World Map

The `world-map` binary fetches live relay positions and renders a self-contained SVG map.
It is rebuilt and committed hourly by CI. To generate it locally:

```bash
# First time only — downloads GeoLite2-City.mmdb into assets/
MAXMIND_LICENSE_KEY=your_key cargo build --release

# Every subsequent run (no key needed once the mmdb is cached)
./target/release/world-map
```

Dot colours: 🟣 purple = Guard · 🔴 red = Exit · 🟡 yellow = Middle

## GitHub Actions

The included workflow (`.github/workflows/sync.yml`) runs every hour via `schedule: cron: '0 * * * *'`, builds and runs the parser, then commits the updated CSVs to the repo if anything changed. You can also trigger it manually from the **Actions** tab using `workflow_dispatch`.

The `.github/workflows/map.yml` workflow also runs hourly, regenerates `latest.map.svg`, and commits it back to the branch.

> **Note:** GitHub may delay scheduled workflows by up to ~15–30 minutes during high runner demand, and will automatically disable the schedule if the repo has no activity for 60 days.
>
> **Secret required:** Add `MAXMIND_LICENSE_KEY` as a repository secret under **Settings → Secrets and variables → Actions** for the map CI to download the GeoLite2-City database on its first run.

## Other Useful Onionoo Endpoints

The same Onionoo API exposes several other endpoints for running relays worth tracking on an hourly basis:

| Endpoint | What it provides |
|----------|------------------|
| [`/bandwidth`](https://onionoo.torproject.org/bandwidth?search=type:relay%20running:true) | Per-relay read/write history over 1 month, 6 months, 1 year, 5 years (bytes/sec) |
| [`/weights`](https://onionoo.torproject.org/weights?search=type:relay%20running:true) | Consensus weight fractions — probability each relay is selected for guard, middle, or exit position |
| [`/uptime`](https://onionoo.torproject.org/uptime?search=type:relay%20running:true) | Historical uptime fractions per relay across the same time windows |
| [`/details`](https://onionoo.torproject.org/details?search=type:relay%20running:true) *(current)* | Full relay metadata: flags, GeoIP country/AS, Tor version, exit policy, family, bandwidth caps |

## References

- [Hiding Routing Information](https://www.onion-router.net/Publications.html#IH-1996) — D. Goldschlag, M. Reed, P. Syverson (1996). The original onion routing paper describing layered encryption and anonymous communication through a network of routing nodes.
- [Tor Protocol Specification](https://spec.torproject.org/tor-spec/) — The official Tor Project specification covering the Tor protocol in detail, including circuit construction, cell formats, and relay cryptography.
