# Key Monitor v1.2.0

Real-time keyboard and mouse click statistics with a visual UI. Backend records input events via Windows hooks or rdev, stores counts in SQLite, and serves a live-updating HTML frontend.

## Quick Start

```bash
# Run with default config (no config.json needed)
cargo run --release
```

Open `http://localhost:5000` in a browser to see the live keyboard/mouse layout.

## CLI Arguments

| Argument | Platform | Description |
|---|---|---|
| `--console`, `-c` | Windows | Show a console window (hidden by default via `#![windows_subsystem = "windows"]`) |

## Configuration

Create a `config.json` in the project root (optional — defaults apply otherwise):

```json
{
  "db_file": "monitor.sqlite",
  "port": 5000,
  "listener": "rawinput"
}
```

### Fields

| Key | Type | Default | Description |
|---|---|---|---|
| `db_file` | string | `"monitor.sqlite"` | SQLite database file path |
| `port` | number | `5000` | HTTP server port |
| `listener` | string | `"rawinput"` (Windows)<br>`"rdev"` (other) | Input event backend |

### `listener` values

| Value | Platform | Keyboard | Mouse | Description |
|---|---|---|---|---|
| `"rawinput"` | Windows only | `WH_KEYBOARD_LL` hook | Raw Input (`WM_INPUT`) via hidden message-only window + `RIDEV_NOLEGACY` | **Default.** Filters out mouse-move events entirely at the driver level — zero CPU overhead for idle movement |
| `"native"` | Windows only | `WH_KEYBOARD_LL` hook | `WH_MOUSE_LL` hook | Legacy approach. Mouse-move events are still delivered but filtered in the callback |
| `"rdev"` | All platforms | rdev `listen()` | rdev `listen()` | Cross-platform fallback via the `rdev` crate |

### `listener` selection notes

- On **Windows**, `"rawinput"` is recommended: mouse movement generates **zero** events, only clicks/wheel give you `WM_INPUT`.
- On **non-Windows**, only `"rdev"` is available and is selected automatically.
- If an unknown value is given, `"rdev"` is used as the fallback.

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/keycounts` | Current in-memory key counts as JSON `{"key_name": count, ...}` |
| `GET` | `/history?start=YYYY-MM-DD&end=YYYY-MM-DD` | Aggregated stats for a date range |
| `GET` | `/events` | SSE stream — pushes updated JSON every time a key/button is pressed |

Static files (`index.html`, `static/`) are served from the project root. The fallback route serves `index.html` for SPA-like navigation.

### Example

```bash
curl http://localhost:5000/keycounts
# → {"a": 42, "enter": 7, "mouse_left": 3, ...}

curl "http://localhost:5000/history?start=2026-06-01&end=2026-06-22"
# → {"2026-06-01": {"a": 100, ...}, "2026-06-22": {"a": 42, ...}}
```

## Build Release

```bash
cargo build --release
```

The single binary is at `target/release/keymouse-monitor.exe` (Windows) or `target/release/keymouse-monitor` (other). No runtime dependencies.

Release builds are also available as GitHub Actions artifacts (triggered on pushes to `main` that modify the `version` file). The release asset includes both the binary and `index.html`.

## Data Persistence

- Counts are saved to SQLite every **60 seconds** automatically.
- Shutdown loses at most 60 seconds of data (graceful shutdown NOT yet implemented — in-memory deltas since last save are dropped).
- Database schema: table `daily_stats` with columns `day` (DATE), `key_name` (TEXT), `count` (INTEGER), unique on `(day, key_name)`.

## Project Structure

```
├── src/
│   ├── main.rs              # Entry point, console attach, timer
│   ├── config.rs            # Config loading from config.json
│   ├── api.rs               # Axum router, SSE, /keycounts, /history
│   ├── data.rs              # In-memory MonitorData + save logic
│   ├── database.rs          # SQLite read/write via rusqlite
│   ├── maps.rs              # Key/Button → display string mapping
│   └── listener/
│       ├── mod.rs           # Dispatch by config listener kind
│       ├── common.rs        # Shared: CallbackData, process_event
│       ├── keyboard.rs      # Shared: VK→Key mapping table
│       ├── native.rs        # WH_KEYBOARD_LL + WH_MOUSE_LL backend
│       ├── rawinput.rs      # WH_KEYBOARD_LL + Raw Input backend
│       └── rdev.rs          # Cross-platform rdev backend
├── tools/
│   └── key_viewer/          # Standalone binary to inspect VK codes
├── index.html               # Live keyboard + mouse visual frontend
├── config.json              # Optional json config
├── version                  # Current version string (used by CI)
└── Cargo.toml
```
