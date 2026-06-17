use thiserror::Error;

/// Errors returned by the BAM network client.
#[derive(Debug, Error)]
pub enum BamError {
    /// The HTTP request failed, or the API returned a non-success status,
    /// or the response body could not be deserialized.
    #[error("BAM explorer request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Returned by the (reserved) [`crate::attestation`] module: BAM ordering
    /// attestations are not yet retrievable from any public source. See the
    /// project README, section "Investigation", for details.
    #[error("BAM ordering attestations have no public source yet (see the bam-net README roadmap)")]
    AttestationsUnavailable,
}

/// Convenience alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, BamError>;
