//! Local append-only history log for the BAM network.
//!
//! Each [`SnapshotStore::append`] writes one JSON object per line (JSONL) to a
//! plain text file: a [`NetworkSnapshot`] tagged with its UTC capture time.
//! There is no database and no C dependency — the log is human-readable,
//! `git`-diffable, and streamed line by line, which is plenty for the handful
//! of nodes and few hundred validators BAM reports.
//!
//! Capture is manual: run `bam-net snapshot` on whatever schedule you like
//! (cron, Windows Task Scheduler, …) and the [`history`] / [`churn`] helpers
//! turn the accumulated log into time series.
//!
//! ```no_run
//! use bam_net::{BamExplorerClient, SnapshotStore};
//!
//! let client = BamExplorerClient::new();
//! let store = SnapshotStore::new(SnapshotStore::default_path());
//! let captured = store.append(&client.snapshot()?)?;
//! println!("captured at {}", captured.ts);
//! # Ok::<(), bam_net::BamError>(())
//! ```

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::error::Result;
use crate::types::NetworkSnapshot;

/// A [`NetworkSnapshot`] tagged with the time it was captured.
///
/// Serialized as a single JSONL line, e.g.
/// `{"ts":"2026-06-17T12:00:00Z","stake":{…},"nodes":[…],"validators":[…]}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimestampedSnapshot {
    /// RFC 3339 / ISO 8601 UTC capture time.
    pub ts: String,
    /// The network snapshot captured at `ts`.
    #[serde(flatten)]
    pub snapshot: NetworkSnapshot,
}

/// An append-only JSONL log of [`TimestampedSnapshot`] records on disk.
#[derive(Clone, Debug)]
pub struct SnapshotStore {
    path: PathBuf,
}

impl SnapshotStore {
    /// Create a store backed by `path` (the file is created on first append).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The default history file location.
    ///
    /// Resolution order: the `BAM_NET_CACHE` environment variable, else the
    /// per-user OS data directory (`%APPDATA%` on Windows, `$XDG_DATA_HOME`
    /// or `~/.local/share` elsewhere), with `bam-net/history.jsonl` appended.
    pub fn default_path() -> PathBuf {
        if let Some(p) = std::env::var_os("BAM_NET_CACHE") {
            return PathBuf::from(p);
        }
        let base = if cfg!(windows) {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        };
        base.unwrap_or_else(|| PathBuf::from("."))
            .join("bam-net")
            .join("history.jsonl")
    }

    /// The path this store reads from and writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append `snapshot`, stamped with the current UTC time, to the log.
    ///
    /// Creates the parent directory and the file if needed. Returns the record
    /// as written.
    pub fn append(&self, snapshot: &NetworkSnapshot) -> Result<TimestampedSnapshot> {
        // Whole-second precision keeps timestamps a tidy fixed width; nothing
        // here is captured often enough for sub-second resolution to matter.
        let now = OffsetDateTime::now_utc();
        let now = now.replace_nanosecond(0).unwrap_or(now);
        let record = TimestampedSnapshot {
            ts: now.format(&Rfc3339)?,
            snapshot: snapshot.clone(),
        };
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{}", serde_json::to_string(&record)?)?;
        Ok(record)
    }

    /// Load every record in capture order (oldest first).
    ///
    /// Returns an empty vec if the log does not exist yet.
    pub fn load(&self) -> Result<Vec<TimestampedSnapshot>> {
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut records = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            records.push(serde_json::from_str(&line)?);
        }
        Ok(records)
    }
}

/// One point in the adoption time series, derived from a [`TimestampedSnapshot`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    /// Capture time (RFC 3339 UTC).
    pub ts: String,
    /// Total stake running BAM, in SOL.
    pub bam_stake: f64,
    /// BAM stake as a percentage of total network stake.
    pub bam_stake_percentage: f64,
    /// Number of BAM nodes.
    pub node_count: usize,
    /// Number of BAM validators.
    pub validator_count: usize,
    /// Largest node's share of total node stake, as a percentage — a simple
    /// concentration gauge (100% = one node holds all BAM stake).
    pub top_node_share: f64,
}

/// Turn a run of captures into the adoption time series (same order in).
pub fn history(records: &[TimestampedSnapshot]) -> Vec<HistoryPoint> {
    records
        .iter()
        .map(|r| {
            let total_node_stake: f64 = r.snapshot.nodes.iter().map(|n| n.node_stake).sum();
            let top_node_stake = r
                .snapshot
                .nodes
                .iter()
                .map(|n| n.node_stake)
                .fold(0.0_f64, f64::max);
            let top_node_share = if total_node_stake > 0.0 {
                top_node_stake / total_node_stake * 100.0
            } else {
                0.0
            };
            HistoryPoint {
                ts: r.ts.clone(),
                bam_stake: r.snapshot.stake.bam_stake,
                bam_stake_percentage: r.snapshot.stake.bam_stake_percentage,
                node_count: r.snapshot.node_count(),
                validator_count: r.snapshot.validator_count(),
                top_node_share,
            }
        })
        .collect()
}

/// A validator whose BAM node connection changed between two captures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidatorMove {
    /// The validator's identity public key.
    pub validator_pubkey: String,
    /// Node it was connected to in the earlier capture (`None` = unconnected).
    pub from: Option<String>,
    /// Node it is connected to in the later capture (`None` = unconnected).
    pub to: Option<String>,
}

/// Validator-to-node changes between two captures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Churn {
    /// Capture time of the earlier snapshot.
    pub from_ts: String,
    /// Capture time of the later snapshot.
    pub to_ts: String,
    /// Validators present in both captures whose node connection changed.
    pub moved: Vec<ValidatorMove>,
    /// Public keys of validators present only in the later capture.
    pub joined: Vec<String>,
    /// Public keys of validators present only in the earlier capture.
    pub left: Vec<String>,
}

impl Churn {
    /// `true` when nothing changed between the two captures.
    pub fn is_empty(&self) -> bool {
        self.moved.is_empty() && self.joined.is_empty() && self.left.is_empty()
    }
}

/// Compute validator-to-node [`Churn`] from the `from` capture to the `to` one.
///
/// Output lists are sorted (by pubkey) so the result is deterministic.
pub fn churn(from: &TimestampedSnapshot, to: &TimestampedSnapshot) -> Churn {
    let from_conn: HashMap<&str, Option<&str>> = from
        .snapshot
        .validators
        .iter()
        .map(|v| {
            (
                v.validator_pubkey.as_str(),
                v.bam_node_connection.as_deref(),
            )
        })
        .collect();
    let to_conn: HashMap<&str, Option<&str>> = to
        .snapshot
        .validators
        .iter()
        .map(|v| {
            (
                v.validator_pubkey.as_str(),
                v.bam_node_connection.as_deref(),
            )
        })
        .collect();

    let mut moved = Vec::new();
    let mut joined = Vec::new();
    for (pubkey, to_node) in &to_conn {
        match from_conn.get(pubkey) {
            None => joined.push((*pubkey).to_string()),
            Some(from_node) if from_node != to_node => moved.push(ValidatorMove {
                validator_pubkey: (*pubkey).to_string(),
                from: from_node.map(str::to_string),
                to: to_node.map(str::to_string),
            }),
            Some(_) => {}
        }
    }

    let mut left: Vec<String> = from_conn
        .keys()
        .filter(|pubkey| !to_conn.contains_key(*pubkey))
        .map(|pubkey| (*pubkey).to_string())
        .collect();

    moved.sort_by(|a, b| a.validator_pubkey.cmp(&b.validator_pubkey));
    joined.sort();
    left.sort();

    Churn {
        from_ts: from.ts.clone(),
        to_ts: to.ts.clone(),
        moved,
        joined,
        left,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BamNode, BamStake, Validator};

    fn node(id: &str, stake: f64) -> BamNode {
        BamNode {
            bam_node: id.to_string(),
            region: id.to_string(),
            connected_validators: 0,
            node_stake: stake,
        }
    }

    fn validator(pubkey: &str, node: Option<&str>) -> Validator {
        Validator {
            validator_pubkey: pubkey.to_string(),
            bam_node_connection: node.map(str::to_string),
            stake: 1.0,
            stake_percentage: 0.1,
        }
    }

    fn record(ts: &str, nodes: Vec<BamNode>, validators: Vec<Validator>) -> TimestampedSnapshot {
        TimestampedSnapshot {
            ts: ts.to_string(),
            snapshot: NetworkSnapshot {
                stake: BamStake {
                    bam_stake: 100.0,
                    bam_stake_percentage: 30.0,
                },
                nodes,
                validators,
            },
        }
    }

    #[test]
    fn append_then_load_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "bam-net-test-{}-{}.jsonl",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let store = SnapshotStore::new(&path);

        let snap = record("", vec![node("a", 10.0)], vec![validator("v1", Some("a"))]).snapshot;
        store.append(&snap).unwrap();
        store.append(&snap).unwrap();

        let loaded = store.load().unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].snapshot, snap);
        assert!(!loaded[0].ts.is_empty(), "append should stamp a timestamp");
    }

    #[test]
    fn load_missing_file_is_empty() {
        let store = SnapshotStore::new("does-not-exist-xyz.jsonl");
        assert!(store.load().unwrap().is_empty());
    }

    #[test]
    fn history_tracks_adoption_and_concentration() {
        let recs = vec![
            record("t1", vec![node("a", 75.0), node("b", 25.0)], vec![]),
            record("t2", vec![node("a", 50.0), node("b", 50.0)], vec![]),
        ];
        let points = history(&recs);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].node_count, 2);
        assert!((points[0].top_node_share - 75.0).abs() < 1e-9);
        assert!((points[1].top_node_share - 50.0).abs() < 1e-9);
    }

    #[test]
    fn churn_detects_moves_joins_and_leaves() {
        let before = record(
            "t1",
            vec![],
            vec![
                validator("stayer", Some("a")),
                validator("mover", Some("a")),
                validator("leaver", Some("b")),
            ],
        );
        let after = record(
            "t2",
            vec![],
            vec![
                validator("stayer", Some("a")),
                validator("mover", Some("b")),
                validator("joiner", Some("c")),
            ],
        );

        let c = churn(&before, &after);
        assert!(!c.is_empty());
        assert_eq!(c.moved.len(), 1);
        assert_eq!(c.moved[0].validator_pubkey, "mover");
        assert_eq!(c.moved[0].from.as_deref(), Some("a"));
        assert_eq!(c.moved[0].to.as_deref(), Some("b"));
        assert_eq!(c.joined, vec!["joiner".to_string()]);
        assert_eq!(c.left, vec!["leaver".to_string()]);
    }
}
