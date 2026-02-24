# tor-nodes

Fetches the live Tor relay list from the [Onionoo API](https://metrics.torproject.org/onionoo.html) and outputs three CSV files:

| File | Contents |
|------|----------|
| [`latest.all.csv`](latest.all.csv) | Every running relay |
| [`latest.guards.csv`](latest.guards.csv) | Relays with the `Guard` flag |
| [`latest.exits.csv`](latest.exits.csv) | Relays with the `Exit` flag |

Each CSV row has the format:
```
fingerprint, ipaddr, port
```

## Usage

```bash
cargo run --release
```

Outputs `all.csv`, `guards.csv`, and `exits.csv` in the current directory.

## GitHub Actions

The included workflow (`.github/workflows/sync.yml`) runs every hour via `schedule: cron: '0 * * * *'`, builds and runs the parser, then commits the updated CSVs to the repo if anything changed. You can also trigger it manually from the **Actions** tab using `workflow_dispatch`.

> **Note:** GitHub may delay scheduled workflows by up to ~15–30 minutes during high runner demand, and will automatically disable the schedule if the repo has no activity for 60 days.
