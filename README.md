# tor-nodes

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

## Relay Types

**All relays** — Any Tor node currently marked as running in the consensus. Acts as a middle hop in circuits by default, passing encrypted traffic between other relays without knowing the origin or destination.

**Guards** — Entry-point relays that a Tor client connects to directly. They are stable, high-bandwidth nodes vetted by the directory authorities. A client picks a small set of guards and sticks with them for months to limit traffic-analysis exposure.

**Exits** — The final hop in a circuit. Exit relays decrypt the last layer of onion encryption and make the actual connection to the destination server on the user's behalf. They are the only nodes that see the destination hostname/IP (but not who the user is).

## Usage

```bash
cargo run --release
```

Outputs `all.csv`, `guards.csv`, and `exits.csv` in the current directory.

## GitHub Actions

The included workflow (`.github/workflows/sync.yml`) runs every hour via `schedule: cron: '0 * * * *'`, builds and runs the parser, then commits the updated CSVs to the repo if anything changed. You can also trigger it manually from the **Actions** tab using `workflow_dispatch`.

> **Note:** GitHub may delay scheduled workflows by up to ~15–30 minutes during high runner demand, and will automatically disable the schedule if the repo has no activity for 60 days.

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
