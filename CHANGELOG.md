# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project aims
to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
