//! Module 1: Gossip Protocol
//! =========================
//!
//! Simulates peer-list gossip among a network of nodes.
//!
//! How gossip works:
//!   1. Each node maintains a list of peers it knows about.
//!   2. Periodically, a node picks a random peer and shares its peer list.
//!   3. The receiving node merges the incoming list with its own.
//!   4. Over time, all nodes converge to a complete view of the network.
//!
//! Real-world systems that use gossip:
//!   - Apache Cassandra (cluster membership)
//!   - Amazon DynamoDB (ring topology)
//!   - Consul (membership and health)
//!
//! Run the Algorithm Labs notebook to see gossip convergence in action
//! before implementing it here.
#![expect(
    clippy::doc_markdown,
    reason = "the docs are as they were meant to be, for now"
)]
#![allow(
    clippy::missing_docs_in_private_items,
    reason = "the docs are as they were meant to be, for now"
)]

use core::fmt;
use core::fmt::Display;
use std::collections::HashMap;

/// A single known peer in the gossip table.
#[derive(Debug, Clone)]
pub struct PeerEntry {
    node_id: String,
    // TODO: there is probably a URL newtype somewhere I could be using.
    queue_url: String,
    // I feel like this should be a date or time. But for now, the reference
    // implementation uses a float, so this is what I've got.
    last_seen: f64,
    /// Rounds until expiry (refreshed on re-gossip).
    // I feel like this should be a u8. But for now, an i8 should provide good
    // compatibility.
    time_to_live: i8,
}

impl PeerEntry {
    const fn is_expired(&self) -> bool {
        self.time_to_live <= 0
    }
}

impl Display for PeerEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PeerEntry({}, ttl={})", self.node_id, self.time_to_live)
    }
}

/// A node that participates in gossip-based peer discovery.
#[derive(Debug, Clone)]
pub struct GossipNode {
    /// This node's identifier.
    node_id: String,
    // TODO: there is probably a URL newtype somewhere I could be using.
    queue_url: String,
    /// Dict of node_id -> PeerEntry for all known peers.
    peers: HashMap<String, PeerEntry>,
    log: Vec<String>,
}

impl GossipNode {
    pub fn new(node_id: String, queue_url: String) -> Self {
        Self {
            node_id,
            queue_url,
            peers: HashMap::new(),
            log: Vec::new(),
        }
    }

    /// Manually add or refresh a peer (bootstrap / initial knowledge).
    ///
    /// Called when:
    ///  - We receive a HELLO from a new node
    ///  - We are told about a peer via PEER_LIST
    ///
    /// Args:
    ///    node_id:   The peer's identifier.
    ///    queue_url: The peer's SQS queue URL.
    pub fn add_peer(&self, node_id: String, queue_url: String) {
        todo!()
    }

    /// Build a PEER_LIST message payload (list of dicts to send over SQS).
    ///
    /// Returns a list of {"node_id": ..., "queue_url": ...} dicts for all
    /// known non-expired peers. Include yourself so recipients know your URL.
    pub fn get_peer_list_message(&self) -> Vec<HashMap<String, String>> {
        todo!()
    }

    /// Merge an incoming peer list with our own.
    ///
    /// For each peer in the incoming list:
    ///  - If we don't know them, add them (TTL = 5)
    ///  - If we already know them, refresh their TTL
    ///
    /// Args:
    ///    incoming:  List of {"node_id": str, "queue_url": str} dicts.
    ///    sender_id: node_id of the peer who sent this list.
    ///
    /// Returns:
    ///    Number of new peers discovered (not previously in our table).
    pub fn receive_peer_list(
        &self,
        incoming: Vec<HashMap<String, String>>,
        sender_id: String,
    ) -> u8 {
        todo!()
    }

    /// Decrement TTL on all entries; remove expired ones.
    /// Called once per poll round.
    ///
    /// A peer that has not been mentioned in any gossip for `ttl` rounds
    /// should be removed from the table.
    pub fn age_entries(&self) {
        todo!()
    }

    /// Pick a random peer to send a PEER_LIST to.
    /// Returns None if no peers are known yet.
    pub fn pick_gossip_target(&self) -> Option<String> {
        todo!()
    }

    pub fn known_peer_count(&self) -> usize {
        self.peers.len()
    }
}

impl Display for GossipNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GossipNode({}, peers={:?})",
            self.node_id,
            self.peers.keys()
        )
    }
}
