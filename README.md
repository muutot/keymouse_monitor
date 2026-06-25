# Key Monitor v2.1.1

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
| `--help`, `-h` | all | Print usage and exit (all binaries support this) |

## Configuration

Create a `config.json` next to the executable (optional — all fields have defaults):

```json
{
  "database": {
    "backend": "sqlite",
    "sqlite": {
      "path": "monitor.sqlite",
      "table": "daily_stats"
    },
    "mongodb": {
      "protocol": "mongodb",
      "database": "keymouse_monitor",
      "hosts": ["localhost:27017"],
      "ssl": true,
      "replicaSet": "atlas-abc-shard-0",
      "appName": "MyApp",
      "username": "user",
      "password": "pass",
      "collection": "daily_stats"
    }
  },
  "port": 5000,
  "listener": "rawinput",
  "save_interval_secs": 60,
  "log": {
    "level": "info",
    "file": "logs/monitor.log",
    "rotation": "daily",
    "console": true
  }
}
```

### `config.database`

| Key | Type | Default | Description |
|---|---|---|---|
| `backend` | string | `"sqlite"` | `"sqlite"` or `"mongodb"` |
| `sqlite` | object | `{path: "monitor.sqlite", table: "daily_stats"}` | SQLite settings |
| `mongodb` | object | (see below) | MongoDB connection settings |

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
| `authSource` | string | `"admin"` | Auth source database |
| `ssl` | bool | `true` | Use TLS |
| `replicaSet` | string (nullable) | `null` | Replica set name |
| `appName` | string (nullable) | `null` | Application name |
| `hosts` | string array (nullable) | `null` | Host list, e.g. `["host:27017"]` |
| `connectTimeoutMs` | number | `15000` | Connection timeout |
| `serverSelectionTimeoutMs` | number | `30000` | Server selection timeout |
| `collection` | string | `"daily_stats"` | Collection name |

### Other top-level fields

| Key | Type | Default | Description |
|---|---|---|---|
| `port` | number | `5000` | HTTP server port |
| `listener` | string | `"rawinput"` (Windows)<br>`"rdev"` (other) | Input event backend |
| `update_mode` | string | `"diff"` | DB save mode: `"diff"` (only changed keys) or `"full"` (snapshot) |
| `save_interval_secs` | number | `60` | Periodic DB save interval |
| `log` | object | (see below) | Logging configuration |

### `log` fields

| Key | Type | Default | Description |
|---|---|---|---|
| `level` | string | `"info"` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `file` | string | `"logs/monitor.log"` | Log file path (relative to exe) |
| `rotation` | string | `"daily"` | Log rotation: `"daily"`, `"hourly"`, `"never"` |
| `console` | bool | `true` | Whether to also log to terminal when `--console` is passed |

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
| `GET` | `/api/version` | `{"version": "2.1.1", "name": "keymouse-monitor"}` |

### SSE format (`/events`)

```
data: {"a": 42, "enter": 7, "mouse_left": 3, ...}\n\n
```

Fires on every key/button/wheel event (only when at least one SSE client is connected).

### Export / Import JSON format

Two output formats available via the UI export modal:

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

Flat format (same structure, `records` is an array of `{date, key, count}` objects):

```json
{
  "backend": "sqlite",
  "exported_at": "2026-06-23T12:34:56",
  "records": [
    {"date": "2026-06-22", "key": "a", "count": 100},
    {"date": "2026-06-22", "key": "enter", "count": 7}
  ]
}
```

Import accepts both formats. Import modes: `"overwrite"` (replace existing records) or `"merge"` (add counts).

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
- DB schema (SQLite): table `daily_stats` with columns `date` (TEXT), `key` (TEXT), `count` (INTEGER), primary key `(date, key)`.
- MongoDB: collection `daily_stats` with documents `{date, key, count}`.

## Build Release

```bash
cargo build --release
```

Single binary at `target/release/keymouse-monitor` (`.exe` on Windows) with embedded icon. No runtime dependencies.  
Statically linked C runtime (via `.cargo/config.toml`).

CI (GitHub Actions) builds on push to `main` that modifies the `version` file; assets include binary + `index.html`.

## Project Structure

```
├── src/
│   ├── main.rs               # Entry point, timer, graceful shutdown
│   ├── config.rs             # Config loading + all serde structs
│   ├── api.rs                # Axum router, SSE, all endpoints
│   ├── data.rs               # In-memory MonitorData, save logic
│   ├── log.rs                # Tracing-based logging (tinfo/terror/twarn/...)
│   ├── database/
│   │   ├── mod.rs            # Database enum, DatabaseBackend trait, ImportMode
│   │   ├── sqlite.rs         # SQLite backend (rusqlite, flat schema)
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
│   ├── key_viewer/           # Live VK code inspector (--rawinput, -h)
│   ├── mouse_bench/          # CPU benchmark of 3 mouse backends (--auto, -j, -h)
│   └── db_check/             # Database connectivity checker [CONFIG_PATH] (-h)
├── index.html                # SPA frontend (dark theme, grid layout)
├── static/
│   ├── svg/                  # SVG assets (logo)
│   └── icon/                 # Auto-generated icon (app.ico, app.rc)
├── build.rs                  # Auto-generates app.ico from SVG
├── CHANGELOG.md              # Version history
├── config.json               # Optional config file
├── version                   # Version string for CI (2.1.1)
└── Cargo.toml
```
