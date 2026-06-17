//! Blocking HTTP client for the public BAM explorer API.

use serde::de::DeserializeOwned;

use crate::error::Result;
use crate::types::{BamNode, BamStake, NetworkSnapshot, Validator};

/// Base URL of the public BAM explorer API.
pub const DEFAULT_BASE_URL: &str = "https://explorer.bam.dev/api/v1";

/// A client for the BAM explorer API.
///
/// The API is public and unauthenticated. All calls are blocking.
///
/// ```no_run
/// use bam_net::BamExplorerClient;
///
/// let client = BamExplorerClient::new();
/// let stake = client.bam_stake()?;
/// println!("BAM secures {:.2}% of stake", stake.bam_stake_percentage);
/// # Ok::<(), bam_net::BamError>(())
/// ```
#[derive(Clone, Debug)]
pub struct BamExplorerClient {
    base_url: String,
    http: reqwest::blocking::Client,
}

impl Default for BamExplorerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl BamExplorerClient {
    /// Create a client pointed at the default public API.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Create a client pointed at a custom base URL (useful for tests or a
    /// self-hosted mirror).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let http = reqwest::blocking::Client::builder()
            .user_agent(concat!("bam-net/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");
        Self {
            base_url: base_url.into(),
            http,
        }
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let body = self.http.get(url).send()?.error_for_status()?.json::<T>()?;
        Ok(body)
    }

    /// Fetch all BAM nodes (`GET /nodes`).
    pub fn nodes(&self) -> Result<Vec<BamNode>> {
        self.get("nodes")
    }

    /// Fetch all BAM validators (`GET /validators`).
    pub fn validators(&self) -> Result<Vec<Validator>> {
        self.get("validators")
    }

    /// Fetch aggregate BAM stake (`GET /bam_stake`).
    pub fn bam_stake(&self) -> Result<BamStake> {
        self.get("bam_stake")
    }

    /// Fetch all three endpoints and assemble a [`NetworkSnapshot`].
    pub fn snapshot(&self) -> Result<NetworkSnapshot> {
        Ok(NetworkSnapshot {
            stake: self.bam_stake()?,
            nodes: self.nodes()?,
            validators: self.validators()?,
        })
    }
}
