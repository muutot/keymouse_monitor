# Changelog

## [2.0.0] - 2026-06-23

### Features
- `f35dd2a` (ui) add export data button
- `00bd3e1` (ui) add export format selector (nested/flat)
- `72d3c32` (ui) add loading overlay with progress bar for history query
- `fda0146` (ui) add export format modal with polished form controls
- `258a552` (api) include import duration in response
- `11d4ff9` upgrade dependencies to latest major versions and fix Windows null pointer safety
- `f6aba77` (log) replace println/eprintln with tracing-based logging system
- `200e358` (build) auto-generate and embed app icon from SVG, plus UI fixes

### Bug Fixes
- `f60f1bd` (api) return JSON error responses instead of plain text
- `f95528c` (timer) prevent burst of rapid saves with MissedTickBehavior::Skip
- `3282fb3` (ui) reset lastLiveData cache on live refresh to avoid stale display
- `0c22c7a` (data) save data to the day it belongs to, not current date
- `71bcf7a` (shutdown) replace timer_task.abort() with cooperative watch-channel shutdown
- `b8d6f01` (shutdown) resolve hangs, panics and SSE drain during graceful shutdown
- `5dcabb1` (windows) resolve config/log paths relative to exe, fix console shutdown

### Performance
- `91d3cac` (db) batch import operations to reduce network round-trips
- `a0d433e` avoid base_counts clone on save and skip aggregation for history query
- `1db1f39` (mongodb) restore server-side aggregation for range queries
- `2ad6582` minimize RwLock hold time during save, split data extraction and db write

### Refactoring
- `55f519a` (ui) skip refresh if already in live mode
- `aa76320` (db) flat storage model (date, key, count) for MongoDB and SQLite

### Chores
- `f0fabef` (api) log import duration to terminal
