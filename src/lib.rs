//! `bam-net` — typed, queryable access to Jito BAM network data.
//!
//! Jito's [Block Assembly Marketplace (BAM)](https://bam.dev) publishes a
//! small, public, unauthenticated API describing the live network: which
//! scheduler nodes exist, which validators connect to them, and how much
//! stake runs BAM. This crate wraps that API in typed Rust structs and a
//! [`NetworkSnapshot`] aggregate so other tools don't have to.
//!
//! ```no_run
//! use bam_net::BamExplorerClient;
//!
//! let client = BamExplorerClient::new();
//! let snap = client.snapshot()?;
//! println!(
//!     "{} validators across {} nodes, {:.2}% of stake",
//!     snap.validator_count(),
//!     snap.node_count(),
//!     snap.stake.bam_stake_percentage,
//! );
//! # Ok::<(), bam_net::BamError>(())
//! ```
//!
//! Note: this exposes BAM **network/adoption** data. Per-transaction
//! *ordering attestations* are not currently published through any public
//! endpoint; the [`attestation`] module is the reserved interface for that
//! capability — see the project README for the investigation behind it.

pub mod attestation;
pub mod cache;
pub mod client;
pub mod error;
pub mod types;

pub use cache::{Churn, HistoryPoint, SnapshotStore, TimestampedSnapshot, ValidatorMove};
pub use client::{BamExplorerClient, DEFAULT_BASE_URL, DEFAULT_TIMEOUT};
pub use error::{BamError, Result};
pub use types::{BamNode, BamStake, NetworkSnapshot, Validator};

// The `attestation` module is intentionally not re-exported at the crate root:
// it is a reserved, not-yet-functional surface. Reach it via
// `bam_net::attestation::*`.
