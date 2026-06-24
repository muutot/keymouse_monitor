## [Unreleased]

- :recycle: [core]: extract `keymouse-common` and `keymouse-rawinput` library
  crates — tools and main binary now depend on shared crates instead of
  duplicating code; raw input logic (window creation, device registration, data
  reading) moved to reusable library
- :sparkles: [db]: add `update_mode` config (`diff`/`full`) for periodic saves
  — `diff` sends only changed keys via incremental merge, `full` sends snapshot
- :sparkles: [db]: add MongoDB backend with SQLite fallback — primary writes go
  to MongoDB; on failure, automatically retry on local SQLite; on reconnect,
  sync missing data back to MongoDB
- :recycle: [code]: replace manual `impl Default` with `#[derive(Default)]`;
  simplify closure and string splitting syntax
- :wrench: [workflow]: replace githook-based auto-changelog with skill-driven
  Unreleased.md workflow — after each commit, review code diff at architecture
  level and write a macro summary to Unreleased.md, committed together with
  code changes
