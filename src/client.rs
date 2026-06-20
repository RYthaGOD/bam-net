//! Blocking HTTP client for the public BAM explorer API.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::Result;
use crate::types::{BamNode, BamStake, NetworkSnapshot, Validator};

/// Base URL of the public BAM explorer API.
pub const DEFAULT_BASE_URL: &str = "https://explorer.bam.dev/api/v1";

/// Default per-request timeout. Without one a stalled connection would hang
/// the caller (and any cron `snapshot`) indefinitely.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How many times to attempt a request before giving up on a *transient*
/// failure (timeout, connection error, 5xx, or 429).
const MAX_ATTEMPTS: usize = 3;

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
        Self::build(base_url.into(), DEFAULT_TIMEOUT)
    }

    /// Return a client with a custom per-request timeout.
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use bam_net::BamExplorerClient;
    ///
    /// let client = BamExplorerClient::new().with_timeout(Duration::from_secs(5));
    /// # let _ = client;
    /// ```
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self::build(self.base_url, timeout)
    }

    fn build(base_url: String, timeout: Duration) -> Self {
        let http = reqwest::blocking::Client::builder()
            .user_agent(concat!("bam-net/", env!("CARGO_PKG_VERSION")))
            .timeout(timeout)
            .build()
            .expect("failed to build HTTP client");
        Self { base_url, http }
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        // Retry transient failures with a short exponential backoff; GETs here
        // are idempotent, so a retried request can't double-apply anything.
        let mut attempt = 1;
        loop {
            match self.fetch(&url) {
                Ok(value) => return Ok(value),
                Err(e) if attempt < MAX_ATTEMPTS && is_transient(&e) => {
                    std::thread::sleep(backoff(attempt));
                    attempt += 1;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn fetch<T: DeserializeOwned>(&self, url: &str) -> reqwest::Result<T> {
        self.http.get(url).send()?.error_for_status()?.json::<T>()
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

/// Backoff before the next attempt: 250ms, 500ms, … (exponential).
fn backoff(attempt: usize) -> Duration {
    Duration::from_millis(250 * (1 << (attempt - 1)))
}

/// Whether a failed request is worth retrying: network-level timeouts and
/// connection errors, plus server-side 5xx and 429. A 4xx (other than 429) or
/// a malformed body is treated as permanent.
fn is_transient(e: &reqwest::Error) -> bool {
    if e.is_timeout() || e.is_connect() {
        return true;
    }
    matches!(e.status().map(|s| s.as_u16()), Some(429) | Some(500..=599))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially() {
        assert_eq!(backoff(1), Duration::from_millis(250));
        assert_eq!(backoff(2), Duration::from_millis(500));
    }

    #[test]
    fn with_timeout_preserves_base_url() {
        let client = BamExplorerClient::with_base_url("http://example.test/api")
            .with_timeout(Duration::from_secs(1));
        assert_eq!(client.base_url, "http://example.test/api");
    }
}
