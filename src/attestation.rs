//! Ordering-attestation interface — **reserved**.
//!
//! BAM nodes (and the leader validator) produce signed proofs that a block's
//! transactions were sequenced and executed in the order BAM dictated. BAM
//! markets these as a *"publicly available audit trail anyone can use."* As of
//! this release, however, **no public endpoint serves them** — see the project
//! README, section *Investigation*.
//!
//! This module is the deliberately-reserved slot for that capability. The data
//! model and the [`AttestationProvider`] trait are defined now so that:
//!
//! 1. downstream code can be written against a stable interface today, and
//! 2. real support can land as a single, additive change the moment Jito
//!    exposes a feed — without breaking anyone.
//!
//! Until then, every provided implementation returns
//! [`BamError::AttestationsUnavailable`]. Field layouts are provisional and
//! will be finalized against Jito's published wire format.
//!
//! ```
//! use bam_net::attestation::{AttestationProvider, PendingPublicSource};
//!
//! let provider = PendingPublicSource;
//! // Today this is an explicit, typed "not available yet" — not a surprise.
//! assert!(provider.attestation(123_456_789).is_err());
//! ```

use serde::{Deserialize, Serialize};

use crate::error::{BamError, Result};

/// A signed proof from a BAM node that a block's transactions were sequenced
/// in a specific order, anchored to a TEE measurement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OrderingAttestation {
    /// Solana slot the attestation covers.
    pub slot: u64,
    /// Ed25519 public key bound to the BAM node's TEE enclave.
    pub enclave_pubkey: Vec<u8>,
    /// Merkle root over the ordered, serialized transactions in the block.
    pub merkle_root: [u8; 32],
    /// Ed25519 signature by the enclave over the canonical payload.
    pub signature: Vec<u8>,
    /// TEE measurement (SGX MRENCLAVE / TDX MRTD) of the signing node.
    pub tee_measurement: String,
    /// Unix seconds when the attestation was produced.
    pub timestamp: u64,
}

/// Proof that a specific transaction occupies a specific position within an
/// attested block's ordering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InclusionProof {
    /// SHA-256 of the serialized transaction.
    pub tx_hash: [u8; 32],
    /// Zero-based position in the attested ordering.
    pub position: u32,
    /// Total transactions in the attested block.
    pub total_txs: u32,
    /// Bottom-up Merkle sibling hashes.
    pub siblings: Vec<[u8; 32]>,
}

/// Outcome of verifying an attestation (and, optionally, an inclusion proof).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VerificationOutcome {
    /// Did the enclave signature over the attestation payload check out?
    pub signature_valid: bool,
    /// Does the TEE measurement match the expected value?
    pub tee_measurement_valid: bool,
    /// Is the attestation within the configured freshness window?
    pub fresh: bool,
    /// Does the Merkle proof re-derive the attested root?
    pub merkle_valid: bool,
    /// Was the transaction's claimed position confirmed?
    pub position_verified: bool,
    /// Slot the verification pertains to.
    pub slot: u64,
}

impl VerificationOutcome {
    /// All checks passed — the ordering integrity is cryptographically sound.
    pub fn is_valid(&self) -> bool {
        self.signature_valid
            && self.tee_measurement_valid
            && self.fresh
            && self.merkle_valid
            && self.position_verified
    }
}

/// Tunables for [`verify`].
#[derive(Clone, Debug)]
pub struct VerifierConfig {
    /// Expected TEE measurement (hex). `None` skips the measurement check.
    pub expected_tee_measurement: Option<String>,
    /// Maximum allowed attestation age, in seconds.
    pub max_age_secs: u64,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            expected_tee_measurement: None,
            max_age_secs: 120,
        }
    }
}

/// A source of BAM ordering attestations — an on-chain program, a public API,
/// a Geyser feed, etc. Implementors fetch attestations by slot.
///
/// Defining this trait now lets callers depend on a stable seam; the concrete
/// provider can be swapped in later with no churn at call sites.
pub trait AttestationProvider {
    /// Fetch the ordering attestation for `slot`.
    fn attestation(&self, slot: u64) -> Result<OrderingAttestation>;
}

/// The placeholder provider used until a public attestation source exists.
///
/// Every call returns [`BamError::AttestationsUnavailable`]. Replace this with
/// a real provider once Jito ships a feed — downstream code is unaffected.
#[derive(Clone, Copy, Debug, Default)]
pub struct PendingPublicSource;

impl AttestationProvider for PendingPublicSource {
    fn attestation(&self, _slot: u64) -> Result<OrderingAttestation> {
        Err(BamError::AttestationsUnavailable)
    }
}

/// Verify an attestation's signature and freshness, and — when an
/// [`InclusionProof`] is supplied — a transaction's position within the
/// attested ordering.
///
/// **Reserved:** returns [`BamError::AttestationsUnavailable`] until the wire
/// format and enclave-key distribution are finalized. The signature is stable
/// so integrations can be written against it now.
pub fn verify(
    _attestation: &OrderingAttestation,
    _proof: Option<&InclusionProof>,
    _config: &VerifierConfig,
) -> Result<VerificationOutcome> {
    Err(BamError::AttestationsUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_source_is_explicit_about_unavailability() {
        let provider = PendingPublicSource;
        assert!(matches!(
            provider.attestation(1),
            Err(BamError::AttestationsUnavailable)
        ));
    }

    #[test]
    fn outcome_validity() {
        let mut o = VerificationOutcome {
            signature_valid: true,
            tee_measurement_valid: true,
            fresh: true,
            merkle_valid: true,
            position_verified: true,
            slot: 1,
        };
        assert!(o.is_valid());
        o.fresh = false;
        assert!(!o.is_valid());
    }
}
