# Keyboard & Mouse Click Monitor

A real-time keyboard and mouse click statistics tool with a visual UI. Backend listens for input events via `pynput`, stores counts in SQLite, and serves data via FastAPI. Frontend renders a realistic keyboard and mouse layout that updates live.

## Features

- **Real-time monitoring** — tracks every key press, mouse click, and scroll event
- **Visual keyboard layout** — see click counts on each key in real time
- **Visual mouse shape** — clickable zones for left/right/middle buttons, scroll wheel, X1/X2 side buttons, and scroll directions
- **Live data updates** — polls the backend every 500ms with animated key highlights
- **Historical queries** — select a date range to view aggregated stats
- **Top N display** — shows the most frequently pressed keys
- **Auto-save** — data is persisted to SQLite on every N clicks (configurable) and at each minute boundary

## Quick Start

### 1. Install dependencies

```bash
pip install fastapi uvicorn pynput
```

### 2. Start the backend

```bash
python main.py
```

The server starts at `http://0.0.0.0:5000`.

### 3. Open the frontend

Open `index.html` in your browser.

## Configuration

Create a `config.json` file in the project root (optional, defaults are used otherwise):

```json
{
  "db_file": "monitor.sqlite",
  "save_threshold": 20,
  "port": 5000
}
```

| Key | Default | Description |
|---|---|---|
| `db_file` | `monitor.sqlite` | SQLite database file path |
| `save_threshold` | `20` | Number of clicks before auto-saving to disk |
| `port` | `5000` | Backend HTTP server port |

## Build Executable

```bash
pyinstaller --onefile --window --name monitor main.py
```

## Project Structure

```
├── main.py              # FastAPI server entry point
├── index.html           # Frontend UI (keyboard + mouse visual)
├── config.json          # Optional configuration
├── monitor.sqlite       # Daily click statistics database
├── src/
│   ├── database.py      # SQLite read/write operations
│   ├── setting.py       # Configuration loader
│   ├── timer.py         # Scheduled task utility
│   ├── tools.py         # Date/time helper functions
│   ├── type_model.py    # Type annotations
│   └── monitor/
│       ├── __init__.py  # Monitor orchestrator
│       ├── listen.py    # Keyboard/mouse event listener
│       ├── monitor_data.py  # In-memory data aggregation
│       └── maps.py      # Virtual key code to name mapping
├── test/
│   └── test_schedule.py # Timer tests
└── static/
    └── svg/             # SVG assets
```

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/keycounts` | Get current real-time key counts |
| `GET` | `/history?start=YYYY-MM-DD&end=YYYY-MM-DD` | Get aggregated stats for a date range |
