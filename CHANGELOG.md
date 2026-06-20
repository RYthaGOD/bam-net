# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project aims
to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0 — 2026-06-20

### Added
- Per-request HTTP timeout (default 30s) so a stalled connection can no longer
  hang the CLI or a cron `snapshot`; `BamExplorerClient::with_timeout()` and the
  exported `DEFAULT_TIMEOUT` make it configurable.
- Automatic retry (up to 3 attempts, exponential backoff) on transient request
  failures — timeouts, connection errors, 5xx, and 429.
- `--csv` output for `nodes`, `validators`, and `history`.
- `SnapshotStore::load_tail(n)` — read only the most recent `n` records; `churn`
  now uses it so it no longer parses the whole log to compare the last two.

### Changed
- The CLI now prints a short, actionable message (instead of reqwest's internal
  error) for timeouts, connection failures, and 404s, and exits non-zero.

## 0.2.0 — 2026-06-17

### Added
- Local append-only history log (JSONL, no database) via the new `cache`
  module: `SnapshotStore` (`append` / `load`) plus `history()` and `churn()`
  time-series helpers.
- CLI commands `snapshot`, `history [--limit N]`, and `churn`, plus a global
  `--cache <PATH>` flag (also honours `$BAM_NET_CACHE`, else the OS data dir).
- `BamError` variants `Io`, `Serde`, and `Time` for the history path.
- `NetworkSnapshot` now derives `PartialEq`.

### Changed
- README documents the history workflow; roadmap marks v0.1 and v0.2 as shipped.
- Published package excludes the TypeScript spike and CI config (Rust-only tarball).

## 0.1.0 — 2026-06-17

### Added
- Typed client and CLI for the public BAM explorer API (`/nodes`,
  `/validators`, `/bam_stake`): `BamExplorerClient` and `NetworkSnapshot` with
  derived queries (busiest node, validators per node, stake totals).
- CLI commands `summary`, `nodes`, `validators`, and `stake` with coloured
  output (yellow + purple), `--json`, `--no-color`, and `--base-url`.
- Reserved `attestation` module — compiled and tested but intentionally
  non-functional — as the stable seam for future ordering-attestation
  verification.
- Apache-2.0 license; offline tests against captured API fixtures.
