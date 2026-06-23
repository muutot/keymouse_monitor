# Key Monitor v1.3.1

Real-time keyboard and mouse click statistics with a visual UI. Backend records input events via Windows hooks or rdev, stores counts in SQLite or MongoDB, and serves a live-updating HTML frontend.

## Quick Start

```bash
cargo run --release
```

Open `http://localhost:5000` in a browser.

## CLI Arguments

| Argument | Platform | Description |
|---|---|---|
| `--console`, `-c` | Windows | Attach a console window (hidden by default) |

## Configuration

Create a `config.json` in the project root (optional — all fields have defaults):

```json
{
  "database": {
    "backend": "sqlite",
    "sqlite": {
      "path": "monitor.sqlite",
      "table": "daily_stats"
    },
    "mongodb": {
      "database": "keymouse_monitor",
      "hosts": ["localhost:27017"],
      "ssl": true,
      "collection": "daily_stats"
    },
    "use_server_aggregation": true
  },
  "port": 5000,
  "listener": "rawinput",
  "save_interval_secs": 60
}
```

### `config.database`

| Key | Type | Default | Description |
|---|---|---|---|
| `backend` | string | `"sqlite"` | `"sqlite"` or `"mongodb"` |
| `sqlite` | object | `{path: "monitor.sqlite", table: "daily_stats"}` | SQLite settings |
| `mongodb` | object | (see below) | MongoDB connection settings |
| `use_server_aggregation` | bool | `true` | Use SQL/aggregation pipeline for range queries vs. client-side sum |

#### `database.sqlite` fields

| Key | Type | Default | Description |
|---|---|---|---|
| `path` | string | `"monitor.sqlite"` | Database file path |
| `table` | string | `"daily_stats"` | Table name |

#### `database.mongodb` fields

| Key | Type | Default | Description |
|---|---|---|---|
| `protocol` | string | `"mongodb"` | URI protocol (`mongodb` / `mongodb+srv`) |
| `database` | string | `"keymouse_monitor"` | Database name |
| `username` | string (nullable) | `null` | Auth username |
| `password` | string (nullable) | `null` | Auth password |
| `auth_source` | string | `"admin"` | Auth source database |
| `ssl` | bool | `true` | Use TLS |
| `replica_set` | string (nullable) | `null` | Replica set name |
| `app_name` | string (nullable) | `null` | Application name |
| `hosts` | string array (nullable) | `null` | Host list, e.g. `["host:27017"]` |
| `connect_timeout_ms` | number | `15000` | Connection timeout |
| `server_selection_timeout_ms` | number | `30000` | Server selection timeout |
| `collection` | string | `"daily_stats"` | Collection name |

### Other top-level fields

| Key | Type | Default | Description |
|---|---|---|---|
| `port` | number | `5000` | HTTP server port |
| `listener` | string | `"rawinput"` (Windows)<br>`"rdev"` (other) | Input event backend |
| `save_interval_secs` | number | `60` | Periodic DB save interval |

### `listener` values

| Value | Platform | Keyboard | Mouse |
|---|---|---|---|
| `"rawinput"` | Windows | `WH_KEYBOARD_LL` hook | Raw Input (`WM_INPUT`) via hidden message-only window + `RIDEV_NOLEGACY` |
| `"native"` | Windows | `WH_KEYBOARD_LL` hook | `WH_MOUSE_LL` hook |
| `"rdev"` | All | rdev `listen()` | rdev `listen()` |

On non-Windows only `"rdev"` is available and selected automatically. Unknown values fall back to `"rdev"`.

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/keycounts` | Current in-memory counts as JSON `{"key": count, ...}` |
| `GET` | `/history?start=YYYY-MM-DD&end=YYYY-MM-DD` | Aggregated stats for a date range |
| `GET` | `/events` | SSE stream — pushes full count JSON on each key/button press |
| `GET` | `/api/export` | Full database export as JSON |
| `POST` | `/api/import?mode=overwrite\|merge` | Import JSON data from export format |
| `GET` | `/api/version` | `{"version": "1.3.1", "name": "keymouse-monitor"}` |

### SSE format (`/events`)

```
data: {"a": 42, "enter": 7, "mouse_left": 3, ...}\n\n
```

Fires on every key/button/wheel event (only when at least one SSE client is connected).

### Export / Import JSON format

```json
{
  "backend": "sqlite",
  "exported_at": "2026-06-23T12:34:56",
  "records": {
    "2026-06-22": {"a": 100, "enter": 7},
    "2026-06-23": {"a": 42, "mouse_left": 3}
  }
}
```

Import modes: `"overwrite"` (replace existing records) or `"merge"` (add counts).

### Examples

```bash
curl http://localhost:5000/keycounts
curl "http://localhost:5000/history?start=2026-06-01&end=2026-06-22"
curl http://localhost:5000/api/export > backup.json
curl -X POST "http://localhost:5000/api/import?mode=merge" -d @backup.json -H "Content-Type: application/json"
```

## Data Persistence

- Counts saved to SQLite/MongoDB every `save_interval_secs` (default 60).
- Graceful shutdown via `Ctrl+C`: saves remaining in-memory data before exit.
- DB schema (SQLite): table `daily_stats` with columns `date` (TEXT PRIMARY KEY), `data` (TEXT — JSON blob).
- MongoDB: collection `daily_stats` with documents `{date, data}`.

## Build Release

```bash
cargo build --release
```

Single binary at `target/release/keymouse-monitor` (`.exe` on Windows). No runtime dependencies.  
Statically linked C runtime (via `.cargo/config.toml`).

CI (GitHub Actions) builds on push to `main` that modifies the `version` file; assets include binary + `index.html`.

## Project Structure

```
├── src/
│   ├── main.rs               # Entry point, timer, graceful shutdown
│   ├── config.rs             # Config loading + all serde structs
│   ├── api.rs                # Axum router, SSE, all endpoints
│   ├── data.rs               # In-memory MonitorData, save logic
│   ├── database/
│   │   ├── mod.rs            # Database enum, DatabaseBackend trait, ImportMode
│   │   ├── sqlite.rs         # SQLite backend (rusqlite, json_each)
│   │   └── mongodb.rs       # MongoDB backend (dedicated tokio runtime)
│   ├── maps.rs               # Key/Button → display string mapping
│   └── listener/
│       ├── mod.rs            # Dispatch by kind (native/rawinput/rdev)
│       ├── common.rs         # CallbackData, process_event
│       ├── keyboard.rs       # VK → rdev::Key mapping
│       ├── native.rs         # WH_KEYBOARD_LL + WH_MOUSE_LL
│       ├── rawinput.rs       # WH_KEYBOARD_LL + Raw Input (stack buffer)
│       └── rdev.rs           # Cross-platform rdev backend
├── tools/
│   ├── key_viewer/           # Live VK code inspector
│   ├── mouse_bench/          # CPU benchmark of 3 mouse backends
│   └── db_check/             # Database connectivity checker
├── index.html                # 1270-line SPA frontend (dark theme, grid layout)
├── static/                   # Static assets
├── config.json               # Optional config file
├── version                   # Version string for CI (1.3.1)
└── Cargo.toml
```
