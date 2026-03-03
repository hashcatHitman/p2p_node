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
