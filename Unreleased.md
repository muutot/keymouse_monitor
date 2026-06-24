- :recycle: [core]: extract `keymouse-common` and `keymouse-rawinput` library
  crates — tools and main binary now depend on shared crates instead of
  duplicating code; raw input logic moved to reusable library
- :sparkles: [db]: add `update_mode` config (`diff`/`full`) for periodic saves
  — `diff` sends only changed keys via incremental merge, `full` sends snapshot
- :sparkles: [db]: add MongoDB backend with SQLite fallback — primary writes go
  to MongoDB; on failure, retry on local SQLite; on reconnect, sync data back
- :wrench: [workflow]: replace githook-based auto-changelog with skill-driven
  Unreleased.md workflow — commits no longer auto-modify CHANGELOG.md
- :art: [changelog]: reformat all historical CHANGELOG entries to
  description-first (`hash last`) format; remove `[Unreleased]` section heading
