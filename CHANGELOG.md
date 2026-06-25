# Changelog

## [2.1.1]

### Features
- (cli) add `-h`/`--help` to all 4 binaries — [`d0de40a`](https://github.com/muutot/keymouse_monitor/commit/d0de40a)

### Bug Fixes
- (frontend) fix SSE status stuck on "正在重连" — add `onopen` to reset on reconnect — [`7ccb9eb`](https://github.com/muutot/keymouse_monitor/commit/7ccb9eb)
- (frontend) fix export/import/version using relative URLs — use `${API_URL}` prefix so
  they work from external static hosting — [`cc849b8`](https://github.com/muutot/keymouse_monitor/commit/cc849b8)

### Chores
- (changelog) rewrap all entries at 88 display chars (hash links count as 0) — [`498ad19`](https://github.com/muutot/keymouse_monitor/commit/498ad19)
- (build) consolidate build scripts into `scripts/` directory; add `exe/` to
  `.gitignore` — [`8d1c295`](https://github.com/muutot/keymouse_monitor/commit/8d1c295)

## [2.1.0]

### Features
- (db) add `update_mode` config (`diff`/`full`) for periodic saves
  — `diff` sends only changed keys, `full` sends snapshot — [`2156081`](https://github.com/muutot/keymouse_monitor/commit/2156081)
- (db) add MongoDB backend with SQLite fallback — auto-retry on
  local SQLite on failure, sync data back on reconnect — [`748a544`](https://github.com/muutot/keymouse_monitor/commit/748a544)

### Refactoring
- (core) extract `keymouse-common` and `keymouse-rawinput` library crates — tools
  and main binary now depend on shared crates; raw input logic (window creation,
  device registration, raw data reading) moved to reusable library — ([`e22ad9f`](https://github.com/muutot/keymouse_monitor/commit/e22ad9f), [`31aec76`](https://github.com/muutot/keymouse_monitor/commit/31aec76), [`47d22ed`](https://github.com/muutot/keymouse_monitor/commit/47d22ed))

### Chores
- (ci) update release workflow and gitignore — [`1bc9b53`](https://github.com/muutot/keymouse_monitor/commit/1bc9b53)
- (workflow) replace githook-based auto-changelog with `[Unreleased]`-section
  workflow — commits write macro summaries into CHANGELOG.md — ([`1325675`](https://github.com/muutot/keymouse_monitor/commit/1325675), [`ca2b3b7`](https://github.com/muutot/keymouse_monitor/commit/ca2b3b7), [`924fa2d`](https://github.com/muutot/keymouse_monitor/commit/924fa2d), [`72d12cd`](https://github.com/muutot/keymouse_monitor/commit/72d12cd))
- (changelog) reformat all historical entries to description-first format — [`eeed664`](https://github.com/muutot/keymouse_monitor/commit/eeed664)

## [2.0.1]

### Bug Fixes
- (rawinput) hardcode X1/X2 button number instead of usButtonData –
  fixes side buttons on systems where usButtonData is always 0 — [`867b043`](https://github.com/muutot/keymouse_monitor/commit/867b043)

### Features
- (key_viewer) add `--rawinput` / `-r` mode for testing raw input events — [`867b043`](https://github.com/muutot/keymouse_monitor/commit/867b043)

### Refactoring
- (imports) group imports and remove fully-qualified std paths — [`ec050a2`](https://github.com/muutot/keymouse_monitor/commit/ec050a2)

### Chores
- (changelog) add commit links and remove date from version title — [`dc16dc4`](https://github.com/muutot/keymouse_monitor/commit/dc16dc4)

## [2.0.0]

### Features
- upgrade dependencies to latest major versions (axum 0.7→0.8,
  mongodb 2→3, rusqlite 0.31→0.40, tower-http 0.5→0.7, windows-sys
  0.52→0.61) and fix Windows null pointer safety — [`d4e85eb`](https://github.com/muutot/keymouse_monitor/commit/d4e85eb)
- (log) replace `println`/`eprintln` with tracing-based logging system —
  configurable level, file output with daily rotation, optional console — [`474760b`](https://github.com/muutot/keymouse_monitor/commit/474760b)
- (build) auto-generate and embed app icon from SVG via `resvg`, plus UI fixes — [`cf3f507`](https://github.com/muutot/keymouse_monitor/commit/cf3f507)
- (core) add graceful shutdown via `tokio::signal::ctrl_c()`,
  configurable DB collections (`SQLite` table / `MongoDB`
  collection), and safe raw input read via `read_unaligned` — [`a599ec0`](https://github.com/muutot/keymouse_monitor/commit/a599ec0)
- (ui) add export format selector (nested/flat) — [`c471178`](https://github.com/muutot/keymouse_monitor/commit/c471178)
- (ui) add loading overlay with progress bar for history query — [`873a194`](https://github.com/muutot/keymouse_monitor/commit/873a194)
- (ui) add export format modal with polished form controls — [`ad973c4`](https://github.com/muutot/keymouse_monitor/commit/ad973c4)
- (ui) add export data button — [`e0ef5d2`](https://github.com/muutot/keymouse_monitor/commit/e0ef5d2)
- (api) include import duration in response — [`8e2c69d`](https://github.com/muutot/keymouse_monitor/commit/8e2c69d)

### Bug Fixes
- (windows) resolve config/log paths relative to exe directory instead
  of CWD; fix console shutdown via `SetConsoleCtrlHandler` — [`4fd7d61`](https://github.com/muutot/keymouse_monitor/commit/4fd7d61)
- (shutdown) resolve hangs, panics and SSE drain during graceful
  shutdown — `spawn_blocking` for MongoDB `rt.block_on`,
  `process::exit(0)` to avoid `MongoBackend.rt` drop panic — [`fa4b71e`](https://github.com/muutot/keymouse_monitor/commit/fa4b71e)
- (shutdown) replace `timer_task.abort()` with cooperative watch-channel
  shutdown to prevent deadlock on `data.write()` lock during final save — [`551b30e`](https://github.com/muutot/keymouse_monitor/commit/551b30e)
- (ui) prevent top-N flicker on page refresh — add placeholder elements,
  `serde(rename)` for MongoConfig camelCase, dynamic `VERSIONINFO` version — [`44717d0`](https://github.com/muutot/keymouse_monitor/commit/44717d0)
- (ui) reset `lastLiveData` cache on live refresh
  to avoid stale display after `clearUI()` — [`ebeb1d1`](https://github.com/muutot/keymouse_monitor/commit/ebeb1d1)
- (timer) prevent burst of rapid saves with `MissedTickBehavior::Skip` —
  missed ticks are dropped instead of firing all at once after delay — [`7778e63`](https://github.com/muutot/keymouse_monitor/commit/7778e63)
- (data) save data to the day it belongs to (`self.today`)
  instead of current date (`today_str`) — fixes cross-day
  rollover saving yesterday's data under today's date — [`099a66c`](https://github.com/muutot/keymouse_monitor/commit/099a66c)
- (api) return JSON error responses instead of plain text — [`2f992d6`](https://github.com/muutot/keymouse_monitor/commit/2f992d6)

### Refactoring
- (db) flat storage model `(date, key, count)` for MongoDB and
  SQLite — auto-migration from old nested format on startup, import
  accepts both formats, export supports `?format=nested|flat` — [`abd3d3d`](https://github.com/muutot/keymouse_monitor/commit/abd3d3d)

### Performance
- minimize `RwLock` hold time during save — `prepare_save()` does in-memory work
  under lock (microseconds), then releases before `upsert_day_stats()` — [`b332834`](https://github.com/muutot/keymouse_monitor/commit/b332834)
- (db) batch import operations to reduce network round-trips — MongoDB: 3 round trips
  instead of 2N; SQLite: single transaction with cached prepared statement — [`b4fbed1`](https://github.com/muutot/keymouse_monitor/commit/b4fbed1)
- avoid `base_counts` clone on save — use `drain()` for in-place
  merge; skip aggregation for history query (fetch docs +
  client-side sum is faster for typical <365 documents) — [`f22359c`](https://github.com/muutot/keymouse_monitor/commit/f22359c)
- (mongodb) restore server-side aggregation for range queries — better than
  client-side sum for large date ranges due to smaller network payload — [`8261821`](https://github.com/muutot/keymouse_monitor/commit/8261821)

### Chores
- (ui) skip refresh if already in live mode —
  prevents unnecessary SSE connection restart — [`9429e1a`](https://github.com/muutot/keymouse_monitor/commit/9429e1a)
- (api) log import duration to terminal — [`24198f0`](https://github.com/muutot/keymouse_monitor/commit/24198f0)
- (mongodb) add timing logs to `get_stats_for_range` — [`8c7d155`](https://github.com/muutot/keymouse_monitor/commit/8c7d155)
- (mongodb) split iterate into network and process time — [`1587cc5`](https://github.com/muutot/keymouse_monitor/commit/1587cc5)
- bump version to 1.3.1 — [`b520d58`](https://github.com/muutot/keymouse_monitor/commit/b520d58)

## [1.3.0]

### Features
- database export/import functionality and MongoDB backend support — [`ca19d53`](https://github.com/muutot/keymouse_monitor/commit/ca19d53)
- import mode — `ImportMode` enum supporting "overwrite"
  and "merge" modes with modal UI for file selection — [`8e7f716`](https://github.com/muutot/keymouse_monitor/commit/8e7f716)
- `use_server_aggregation` flag for server-side
  aggregation in SQLite and MongoDB backends — [`6fb6e5d`](https://github.com/muutot/keymouse_monitor/commit/6fb6e5d)
- version endpoint (`/api/version`) and update project version to 1.3.0
  (Cargo.toml, version file, version badge in `index.html`) — [`2a02d68`](https://github.com/muutot/keymouse_monitor/commit/2a02d68)
- parallel benchmarking, auto-stimulation, and
  JSON output — CLI options for `mouse_bench` — [`0bd6326`](https://github.com/muutot/keymouse_monitor/commit/0bd6326)
- (listener) Raw Input — stack buffer allocation, direct message
  loop, no `DispatchMessage` — bypasses `TranslateMessage`
  + `DispatchMessage` for every mouse event — [`79e8b08`](https://github.com/muutot/keymouse_monitor/commit/79e8b08)

### Refactoring
- MongoDB configuration and URI building — `MongoConfig` fields for protocol, SSL, auth
  source, replica set, app name, hosts, connect timeout, server selection timeout — [`821396b`](https://github.com/muutot/keymouse_monitor/commit/821396b)

## [1.2.0]

### Features
- (listener) Raw Input mouse backend — replaces `WH_MOUSE_LL` with Raw Input (hidden
  window + `WM_INPUT`) using `RIDEV_NOLEGACY`; extract `common.rs` (`CallbackData`,
  `process_event`), `keyboard.rs` (VK→Key mapping); default listener changed to
  "rawinput" on Windows; rewrite README.md with full config/api/CLI docs — [`a91fe4c`](https://github.com/muutot/keymouse_monitor/commit/a91fe4c)

## [1.1.0]

### Features
- (listener) replace `rdev` with native Windows hooks (`SetWindowsHookEx` —
  `WH_KEYBOARD_LL` + `WH_MOUSE_LL`); add scan code detection for numpad Enter
  via `LLKHF_EXTENDED`; always record stats regardless of frontend state — [`a05d05f`](https://github.com/muutot/keymouse_monitor/commit/a05d05f)
- (tools) add `key_viewer` binary for key code inspection;
  skip `MouseMove` events early in listener callback — [`9de7ce7`](https://github.com/muutot/keymouse_monitor/commit/9de7ce7)

### Refactoring
- (listener) split into module folder with `native`/`rdev` backends;
  add listener config field for runtime backend selection — [`5fd74a8`](https://github.com/muutot/keymouse_monitor/commit/5fd74a8)

## [1.0.0]

### Bug Fixes
- (ui) fix nested CSS bug (flatten sub-rules for plain CSS); optimize JS DOM
  queries with key element cache map at init; optimize Top N — cache 25 elements,
  `textContent` instead of full DOM rebuild; merge duplicate CSS selector blocks — [`152f9fb`](https://github.com/muutot/keymouse_monitor/commit/152f9fb)

### Performance
- (db) add `PRAGMA synchronous=NORMAL` for better write performance;
  switch `prepare()` to `prepare_cached()` to reuse compiled query plans;
  remove intermediate `Vec<String>` allocation in range query — [`455de54`](https://github.com/muutot/keymouse_monitor/commit/455de54)

### Refactoring
- optimize maps `Cow<'static, str>` to avoid `String` heap allocation per
  keypress; reduce tokio features (faster compile); simplify timer to fixed 60s
  interval; replace `unwrap` with `expect` for better error messages — [`37ebc51`](https://github.com/muutot/keymouse_monitor/commit/37ebc51)

### Chores
- (release) generate grouped changelog with commit
  links, include `index.html` in release assets — [`2a3a5eb`](https://github.com/muutot/keymouse_monitor/commit/2a3a5eb)
- add Python artifacts to `.gitignore`, clean working tree — [`c94597d`](https://github.com/muutot/keymouse_monitor/commit/c94597d)

## [0.3.0]

### Features
- rewrite entire project from Python to Rust — axum HTTP server, SQLite via
  `rusqlite`, `rdev` for global keyboard/mouse event capture, key/mouse
  count tracking, live SSE updates, configuration management — [`5e3ffb7`](https://github.com/muutot/keymouse_monitor/commit/5e3ffb7)
- (maps) return owned `String` from key/button mapping functions;
  add support for `Key::Letter` and `Key::Num` variants — [`ddb9502`](https://github.com/muutot/keymouse_monitor/commit/ddb9502)
- (api) replace polling with SSE for real-time push updates — `/events`
  SSE endpoint, `tokio::sync::watch` channel, EventSource frontend — [`2f693e9`](https://github.com/muutot/keymouse_monitor/commit/2f693e9)
- (core) replace `Mutex` with `RwLock` for better read concurrency; remove
  `save_threshold` / auto-save-on-threshold; add comprehensive unit tests
  for `MonitorData`, `Database`, and key/mouse mapping functions — [`108658a`](https://github.com/muutot/keymouse_monitor/commit/108658a)

### Bug Fixes
- (ci) update release trigger from `rust-rewrite` to `main` — [`b466420`](https://github.com/muutot/keymouse_monitor/commit/b466420)

### Performance
- (sse) replace 200ms polling in SSE handler with push-based event
  streaming via `tokio::sync::watch` channel; add `SseConnectionGuard`
  to skip processing when no clients connected — [`2a8e680`](https://github.com/muutot/keymouse_monitor/commit/2a8e680)

## [0.2.0]

### Features
- (frontend) migrate live updates from polling (`fetchLiveUpdate`) to SSE
  (`EventSource`) — add `/events` endpoint with async event-driven push; add
  `on_increase` callback; add `--console` / `-c` CLI flag; improve mouse
  button mapping; fix scroll direction key names; add missing key mappings;
  add `vk_` fallback for unmapped keys; clean up `.gitignore` — [`cab79c2`](https://github.com/muutot/keymouse_monitor/commit/cab79c2)

## [0.1.0]

### Features
- (monitor) add mouse scroll event support and redesign mouse UI — [`d6df2f8`](https://github.com/muutot/keymouse_monitor/commit/d6df2f8)
- (ui) add SVG logo, favicon, and update `.gitignore` — [`3d921e6`](https://github.com/muutot/keymouse_monitor/commit/3d921e6)
- isHistory flag and optimize live and history data updates — [`906b8ca`](https://github.com/muutot/keymouse_monitor/commit/906b8ca)
- timer and scheduled tasks for periodic data saving — [`5b1f9de`](https://github.com/muutot/keymouse_monitor/commit/5b1f9de)
- Top N module to display most frequent keys — [`a789d3f`](https://github.com/muutot/keymouse_monitor/commit/a789d3f)

### Refactoring
- (ui) use external favicon, expand mouse visual, and add scroll direction zones — [`796187b`](https://github.com/muutot/keymouse_monitor/commit/796187b)
- switch to `on_release` for keyboard listener and refactor key name extraction — [`05e0e73`](https://github.com/muutot/keymouse_monitor/commit/05e0e73)
- refactor settings to use `Config` class and load from JSON file — [`3dd90db`](https://github.com/muutot/keymouse_monitor/commit/3dd90db)
- (monitor) rename the timed task method name — [`e668b4a`](https://github.com/muutot/keymouse_monitor/commit/e668b4a)
- update key styles and add padding for better readability — [`e490ae4`](https://github.com/muutot/keymouse_monitor/commit/e490ae4)
- update port and dependencies, refactor monitor logic — [`d513a5c`](https://github.com/muutot/keymouse_monitor/commit/d513a5c)

### Chores
- (project) initialize key-monitor — [`59351f3`](https://github.com/muutot/keymouse_monitor/commit/59351f3)
- change `run_timer` to use non-repeating timer and update `Timer` class — [`6f1d41b`](https://github.com/muutot/keymouse_monitor/commit/6f1d41b)
- (ci) add GitHub Actions release workflow and version file — [`4d6a3c4`](https://github.com/muutot/keymouse_monitor/commit/4d6a3c4)





