# Changelog

## [2.1.0]

### Features
- Differential DB saves: `update_mode` config (diff/full) controls whether periodic
  saves send only changed keys (diff) or the full snapshot; SQLite uses `ON CONFLICT
  DO UPDATE`, MongoDB uses `$inc` upsert

## [2.0.1]

### Bug Fixes
- [`72ccc04`](https://github.com/muutot/keymouse_monitor/commit/72ccc04) (rawinput) hardcode X1/X2 button number instead of usButtonData – fixes side buttons on systems where usButtonData is always 0

### Features
- [`72ccc04`](https://github.com/muutot/keymouse_monitor/commit/72ccc04) (key_viewer) add `--rawinput` / `-r` mode for testing raw input events

### Refactoring
- [`53ac0ff`](https://github.com/muutot/keymouse_monitor/commit/53ac0ff) (imports) group imports and remove fully-qualified std paths

### Chores
- [`73d615d`](https://github.com/muutot/keymouse_monitor/commit/73d615d) (changelog) add commit links and remove date from version title

