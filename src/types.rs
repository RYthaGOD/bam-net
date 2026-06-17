//! Typed representations of the BAM explorer API responses, plus a
//! [`NetworkSnapshot`] aggregate with convenience queries.

use serde::{Deserialize, Serialize};

/// A single BAM scheduler node (a TEE running in a region), as reported by
/// the `/nodes` endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BamNode {
    /// Node identifier, e.g. `"fra-mainnet-bam-1-tee"`.
    pub bam_node: String,
    /// Region label (currently mirrors the node id).
    pub region: String,
    /// Number of validators currently connected to this node.
    pub connected_validators: u32,
    /// Total active stake (in SOL) of validators connected to this node.
    pub node_stake: f64,
}

/// A validator participating in BAM, as reported by the `/validators` endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Validator {
    /// The validator's identity public key (base58).
    pub validator_pubkey: String,
    /// The BAM node this validator is connected to, if any.
    pub bam_node_connection: Option<String>,
    /// The validator's active stake, in SOL.
    pub stake: f64,
    /// The validator's stake as a percentage of total network stake.
    pub stake_percentage: f64,
}

/// Aggregate BAM stake, as reported by the `/bam_stake` endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BamStake {
    /// Total stake (in SOL) running BAM.
    pub bam_stake: f64,
    /// That stake as a percentage of total network stake.
    pub bam_stake_percentage: f64,
}

/// A consistent point-in-time view of the BAM network: the three endpoints
/// fetched together, plus derived queries other tools can build on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    /// Aggregate BAM stake.
    pub stake: BamStake,
    /// All BAM nodes.
    pub nodes: Vec<BamNode>,
    /// All BAM validators.
    pub validators: Vec<Validator>,
}

impl NetworkSnapshot {
    /// Number of validators in the snapshot.
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Number of BAM nodes in the snapshot.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Sum of validator stake across the snapshot (in SOL).
    pub fn total_validator_stake(&self) -> f64 {
        self.validators.iter().map(|v| v.stake).sum()
    }

    /// Look up a node by its identifier.
    pub fn node(&self, name: &str) -> Option<&BamNode> {
        self.nodes.iter().find(|n| n.bam_node == name)
    }

    /// The node with the most connected validators, if any.
    pub fn busiest_node(&self) -> Option<&BamNode> {
        self.nodes.iter().max_by_key(|n| n.connected_validators)
    }

    /// Iterate the validators connected to a given node.
    pub fn validators_for_node<'a>(
        &'a self,
        node: &'a str,
    ) -> impl Iterator<Item = &'a Validator> + 'a {
        self.validators
            .iter()
            .filter(move |v| v.bam_node_connection.as_deref() == Some(node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real response shapes captured from explorer.bam.dev/api/v1.
    const NODES_JSON: &str = r#"[
        {"bam_node":"fra-mainnet-bam-1-tee","region":"fra-mainnet-bam-1-tee","connected_validators":89,"node_stake":25584102.76},
        {"bam_node":"ewr-mainnet-bam-1-tee","region":"ewr-mainnet-bam-1-tee","connected_validators":31,"node_stake":20541389.41}
    ]"#;

    const VALIDATORS_JSON: &str = r#"[
        {"validator_pubkey":"DRpbCBMxVnDK7maPM5tGv6MvB3v1sRMC86PZ8okm21hy","bam_node_connection":"ewr-mainnet-bam-1-tee","stake":12992991.17,"stake_percentage":3.0528},
        {"validator_pubkey":"5EhGYUyQNrxgUbuYF4vbL2SZDT6RMfhq3yjeyevvULeC","bam_node_connection":"tyo-mainnet-bam-1-tee","stake":3598006.07,"stake_percentage":0.8454}
    ]"#;

    const STAKE_JSON: &str = r#"{"bam_stake":138367883.6,"bam_stake_percentage":32.511}"#;

    #[test]
    fn deserializes_endpoints() {
        let nodes: Vec<BamNode> = serde_json::from_str(NODES_JSON).unwrap();
        let validators: Vec<Validator> = serde_json::from_str(VALIDATORS_JSON).unwrap();
        let stake: BamStake = serde_json::from_str(STAKE_JSON).unwrap();

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].bam_node, "fra-mainnet-bam-1-tee");
        assert_eq!(nodes[0].connected_validators, 89);
        assert_eq!(
            validators[0].bam_node_connection.as_deref(),
            Some("ewr-mainnet-bam-1-tee")
        );
        assert!((stake.bam_stake_percentage - 32.511).abs() < 1e-9);
    }

    #[test]
    fn snapshot_queries() {
        let snap = NetworkSnapshot {
            stake: serde_json::from_str(STAKE_JSON).unwrap(),
            nodes: serde_json::from_str(NODES_JSON).unwrap(),
            validators: serde_json::from_str(VALIDATORS_JSON).unwrap(),
        };

        assert_eq!(snap.node_count(), 2);
        assert_eq!(snap.validator_count(), 2);
        assert_eq!(
            snap.busiest_node().unwrap().bam_node,
            "fra-mainnet-bam-1-tee"
        );
        assert_eq!(snap.validators_for_node("ewr-mainnet-bam-1-tee").count(), 1);
        assert!((snap.total_validator_stake() - (12992991.17 + 3598006.07)).abs() < 1e-6);
    }
}
