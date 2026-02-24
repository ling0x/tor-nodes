# tor-nodes

Fetches the live Tor relay list from the [Onionoo API](https://metrics.torproject.org/onionoo.html) and outputs three CSV files:

| File | Contents |
|------|----------|
| `all.csv` | Every running relay |
| `guards.csv` | Relays with the `Guard` flag |
| `exits.csv` | Relays with the `Exit` flag |

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

The included workflow (`.github/workflows/sync.yml`) runs every hour, executes the parser, and commits updated CSVs if anything changed.
