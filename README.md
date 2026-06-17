# bam-net

**Typed Rust client + CLI for Jito BAM network data** — the live map of BAM
scheduler nodes, the validators connected to them, and how much of Solana's
stake runs BAM. Built on the public BAM explorer API so other tools don't have
to re-scrape it.

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
![Rust](https://img.shields.io/badge/rust-2021-orange.svg)
![Status](https://img.shields.io/badge/status-v0.1-yellow.svg)

```text
$ bam-net summary
BAM network summary
  BAM stake:   147,653,889.65 SOL (34.69% of network)
  BAM nodes:   15
  Validators:  381
  Busiest:     fra-mainnet-bam-1-tee (89 validators, 25,625,009.41 SOL)
```

---

## What this is

[BAM (Block Assembly Marketplace)](https://bam.dev) is Jito's system for
sequencing Solana transactions through independent, TEE-backed scheduler nodes.
It now secures roughly **a third of all Solana stake** — but the only way to
consume its public network data has been hitting raw JSON endpoints by hand.

`bam-net` is the small, dependency-light layer that fixes that:

- a **library** that turns the BAM explorer API into typed Rust structs plus a
  `NetworkSnapshot` aggregate with ready-made queries, and
- a **CLI** for inspecting the BAM network from your terminal or shell scripts.

It is intended as a building block — the open data layer other BAM tooling
(dashboards, validator-selection tools, decentralization research, alerting)
can stand on.

## Features

- ✅ Typed access to all three public BAM explorer endpoints
- ✅ `NetworkSnapshot` with derived queries (busiest node, validators per node,
  stake totals)
- ✅ Ergonomic CLI: `summary`, `nodes`, `validators`, `stake`, with `--json`
- ✅ Configurable base URL (`--base-url` / `with_base_url`) for tests or mirrors
- ✅ Offline tests against captured response fixtures
- ✅ Small footprint — no async runtime required (blocking client)
- 🔒 A **reserved `attestation` module** for ordering-attestation verification,
  ready to activate when a public source exists (see
  [Roadmap](#roadmap-and-the-attestation-module))

## Install

```bash
# CLI
cargo install bam-net

# library
cargo add bam-net
```

Minimum: a stable Rust toolchain. See [Building](#building) for a Windows note.

## CLI usage

```bash
bam-net summary                        # network-wide overview (default command)
bam-net stake                          # aggregate BAM stake
bam-net nodes                          # all BAM nodes, sorted by stake
bam-net validators --top 20            # top 20 validators by stake
bam-net validators --node fra-mainnet-bam-1-tee   # filter by node
bam-net summary --json | jq            # raw JSON for piping
bam-net --base-url http://localhost:8080/api/v1 nodes   # custom endpoint
```

| Command | Description |
|---|---|
| `summary` | BAM stake %, node count, validator count, busiest node |
| `nodes` | All BAM nodes with connected-validator counts and stake |
| `validators [--node N] [--top K]` | Validators, filterable by node, limitable to top K by stake |
| `stake` | Aggregate BAM stake (SOL and % of network) |

Global flags: `--json` (raw JSON output), `--base-url <URL>` (override the API).

## Library usage

```rust
use bam_net::BamExplorerClient;

fn main() -> bam_net::Result<()> {
    let client = BamExplorerClient::new();

    // One consistent snapshot of the whole network.
    let snap = client.snapshot()?;
    println!(
        "{} validators across {} nodes — {:.2}% of stake",
        snap.validator_count(),
        snap.node_count(),
        snap.stake.bam_stake_percentage,
    );

    // Derived queries.
    if let Some(busiest) = snap.busiest_node() {
        println!("busiest node: {}", busiest.bam_node);
    }
    let fra = snap.validators_for_node("fra-mainnet-bam-1-tee").count();
    println!("validators on fra node: {fra}");

    // Or call individual endpoints directly.
    let stake = client.bam_stake()?;
    println!("BAM stake: {:.0} SOL", stake.bam_stake);

    Ok(())
}
```

### `NetworkSnapshot` queries

`busiest_node()`, `node(name)`, `validators_for_node(name)`,
`total_validator_stake()`, `validator_count()`, `node_count()`.

## Data source / API reference

All data comes from the **public, unauthenticated** BAM explorer API
(`https://explorer.bam.dev/api/v1`):

| Method | Endpoint | Returns |
|---|---|---|
| `client.nodes()` | `GET /nodes` | `Vec<BamNode>` — `{ bam_node, region, connected_validators, node_stake }` |
| `client.validators()` | `GET /validators` | `Vec<Validator>` — `{ validator_pubkey, bam_node_connection, stake, stake_percentage }` |
| `client.bam_stake()` | `GET /bam_stake` | `BamStake` — `{ bam_stake, bam_stake_percentage }` |
| `client.snapshot()` | all three | `NetworkSnapshot` |

> Figures in this README are live samples (≈ June 2026) and will differ when
> you run it.

## Investigation: why network data, not attestations

This project began with a different goal — **indexing and verifying BAM's
transaction-ordering attestations** (the signed proofs that a leader executed
transactions in the order BAM dictated). BAM's documentation describes these as
a *"publicly available audit trail anyone can use."*

A focused data-path spike (preserved in [`spike/`](./spike/)) set out to fetch
and parse one such attestation as an outside party. It could not — and the
reason is structural, not a missing API key:

| Path checked | Result |
|---|---|
| **On-chain** (program logs/accounts via RPC) | The program implied by the marketing copy is a token claim/vesting program, not the sequencer. No attestation program found. |
| **Protocol defs** ([`jito-labs/bam-protos`](https://github.com/jito-labs/bam-protos)) | Defines only the live node ↔ validator scheduler stream (`AtomicTxnBatch`, results, heartbeats). No *published* attestation message; consuming the stream requires running a validator/node. |
| **Explorer API** (`explorer.bam.dev/api/v1`) | Serves only `/nodes`, `/validators`, `/bam_stake`. Every attestation/ordering/inclusion/audit path returns `404`. |
| **Docs / blogs** | State attestations are "publicly available" but never specify an endpoint, on-chain location, or retrieval method. |

**Conclusion:** as of this release, BAM ordering attestations are not
retrievable by an external party from any public source — they appear to flow
only through the private validator scheduler stream. So `bam-net` ships the
BAM data that *is* public and currently untooled (network topology, adoption,
stake), and reserves a clean slot for attestations (below).

> A short, specific question about this gap has been sent to the BAM team; see
> [Note for the BAM / Jito team](#note-for-the-bam--jito-team).

## Roadmap and the `attestation` module

The crate already contains [`src/attestation.rs`](./src/attestation.rs) — a
**reserved interface**, compiled and tested but intentionally non-functional:

- `OrderingAttestation`, `InclusionProof`, `VerificationOutcome`,
  `VerifierConfig` — the provisional data model
- `trait AttestationProvider { fn attestation(&self, slot: u64) -> Result<…> }`
  — the stable seam downstream code can target today
- `PendingPublicSource` — a placeholder provider that returns a typed
  `BamError::AttestationsUnavailable`
- `fn verify(..)` — the verification entry point, signature-stable

This means attestation support can land as a **single additive change** the day
a public source exists — no breaking changes for users who built against the
interface.

Planned, in order:

1. **v0.1** — network/adoption data (this release).
2. **Time-series** — optional local cache (SQLite) to track adoption,
   validator↔node churn, and stake concentration over time.
3. **Attestations** — activate the `attestation` module against whatever public
   source Jito provides (on-chain program, API, or feed), with signature +
   Merkle-inclusion verification.
4. **Thin read API** — optional `axum` service exposing the snapshot + history
   so non-Rust tools can consume it.

## Note for the BAM / Jito team

This crate is a friendly, independent contribution to BAM's tooling ecosystem.
One concrete piece of feedback surfaced while building it:

> BAM's public materials describe ordering attestations as a public audit
> trail, but there is currently **no documented public endpoint** to retrieve
> one as an external party (the explorer API exposes only nodes/validators/
> stake; `bam-protos` only carries the private scheduler stream). If a public
> attestation feed exists or is planned, pointing to it would let open tooling
> like the reserved `attestation` module here verify ordering fairness end to
> end — arguably the strongest demonstration of BAM's transparency thesis.

Happy to adapt `bam-net` to whatever shape that source takes.

## Building

```bash
cargo build
cargo test            # offline; uses captured fixtures
cargo run -- summary  # hits the live API
```

**Windows note:** the networking dependencies require a working native
toolchain. The **MSVC** toolchain (`rustup default stable-msvc` + the *Visual
Studio Build Tools* "Desktop development with C++" / VCTools workload) is the
smoothest path. The GNU toolchain also works but needs a complete MinGW-w64
(the assembler `as.exe` and `dlltool.exe` must both be on `PATH`).

## Project layout

```
src/
  lib.rs          crate root + re-exports
  client.rs       BamExplorerClient (blocking HTTP)
  types.rs        BamNode, Validator, BamStake, NetworkSnapshot (+ tests)
  error.rs        BamError, Result
  attestation.rs  reserved ordering-attestation interface (stub)
  main.rs         the bam-net CLI
spike/            archived phase-0 data-path investigation (TypeScript)
```

## Disclaimer

`bam-net` is an independent open-source project. It is not affiliated with or
endorsed by Jito Labs. "BAM" and related marks belong to their respective
owners. Data is read from the public BAM explorer API and provided as-is.

## License

Licensed under the [Apache License, Version 2.0](./LICENSE).
