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

use rand::seq::IteratorRandom as _;
use serde_json::json;

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
    pub fn new(node_id: String, queue_url: String) -> Self {
        Self {
            node_id,
            queue_url,
            last_seen: 0.0,
            time_to_live: 5,
        }
    }

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
    pub fn add_peer(&mut self, node_id: String, queue_url: String) {
        if self.node_id != node_id && !self.peers.contains_key(&node_id) {
            drop(
                self.peers.insert(
                    node_id.clone(),
                    PeerEntry::new(node_id, queue_url),
                ),
            );
        }
    }

    /// Build a PEER_LIST message payload (list of dicts to send over SQS).
    ///
    /// Returns a list of {"node_id": ..., "queue_url": ...} dicts for all
    /// known non-expired peers. Include yourself so recipients know your URL.
    pub fn get_peer_list_message(&self) -> Vec<serde_json::Value> {
        let mut entries = Vec::new();
        for peer in self.peers.values() {
            entries.push(
                json!({"node_id": peer.node_id, "queue_url": peer.queue_url}),
            );
        }
        entries.push(
            json!({"node_id": self.node_id, "queue_url": self.queue_url}),
        );
        entries
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
        &mut self,
        incoming: Vec<serde_json::Value>,
        sender_id: String,
    ) -> u8 {
        let mut new_count: u8 = 0;

        for entry in incoming {
            let node_id = entry
                .get("node_id")
                .and_then(|v| v.as_str().map(ToOwned::to_owned));

            match node_id {
                Some(node_id) => {
                    if node_id == self.node_id {
                        continue;
                    }

                    if let Some(peer) = self.peers.get_mut(&node_id) {
                        peer.time_to_live = 5
                    } else {
                        let queue_url = entry
                            .get("queue_url")
                            .and_then(|v| v.as_str().map(ToOwned::to_owned));

                        match queue_url {
                            Some(queue_url) => {
                                drop(self.peers.insert(
                                    node_id.clone(),
                                    PeerEntry::new(node_id, queue_url),
                                ));
                                new_count += 1;
                            }
                            None => eprintln!(
                                "[WARNING]: Found no queue_url for {node_id} when receiving peer list from {sender_id}"
                            ),
                        }
                    }
                }
                None => eprintln!(
                    "[WARNING]: Found no node_id when receiving peer list from {sender_id}"
                ),
            }
        }
        new_count
    }

    /// Decrement TTL on all entries; remove expired ones.
    /// Called once per poll round.
    ///
    /// A peer that has not been mentioned in any gossip for `ttl` rounds
    /// should be removed from the table.
    pub fn age_entries(&mut self) {
        let mut expired = Vec::new();
        for (peer_id, entry) in &mut self.peers {
            entry.time_to_live -= 1;

            if entry.is_expired() {
                expired.push(peer_id.clone());
            }
        }

        for peer_id in expired {
            drop(self.peers.remove(&peer_id));
        }
    }

    /// Pick a random peer to send a PEER_LIST to.
    /// Returns None if no peers are known yet.
    pub fn pick_gossip_target(&self) -> Option<String> {
        if self.peers.is_empty() {
            None
        } else {
            self.peers.keys().choose(&mut rand::rng()).cloned()
        }
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

#[expect(clippy::missing_panics_doc, reason = "tests tend to do that")]
#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::algorithms::gossip::GossipNode;

    #[test]
    fn add_peer_increases_count() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        assert_eq!(node.known_peer_count(), 1);
    }

    #[test]
    fn add_same_peer_twice_no_duplicate() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        assert_eq!(node.known_peer_count(), 1);
    }

    #[test]
    fn pick_target_returns_known_peer() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        node.add_peer("node-c".to_owned(), "https://sqs.fake/c".to_owned());
        let target = node.pick_gossip_target();
        match target {
            Some(chosen) => assert!(
                ["node-b".to_owned(), "node-c".to_owned()].contains(&chosen)
            ),
            None => panic!("failed to pick a gossip target"),
        }
        assert_eq!(node.known_peer_count(), 1);
    }

    #[test]
    fn pick_target_none_when_empty() {
        let node = GossipNode::new("node-a".to_owned(), String::new());
        let target = node.pick_gossip_target();
        assert_eq!(target, None);
    }

    #[test]
    fn peer_list_message_format() {
        let mut node = GossipNode::new(
            "node-a".to_owned(),
            "https://sqs.fake/a".to_owned(),
        );
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        let msg = node.get_peer_list_message();
        assert!(!msg.is_empty());
        for entry in msg {
            assert!(entry.get("node_id").is_some());
            assert!(entry.get("queue_url").is_some());
        }
    }

    #[test]
    fn receive_discovers_new_peer() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        let incoming = vec![
            json!({"node_id": "node-c", "queue_url": "https://sqs.fake/c"}),
        ];
        let new = node.receive_peer_list(incoming, "node-b".to_owned());
        assert!(node.known_peer_count() >= 2);
        assert_eq!(new, 1);
    }

    #[test]
    fn receive_no_false_new_for_existing_peer() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());
        let incoming = vec![
            json!({"node_id": "node-b", "queue_url": "https://sqs.fake/b"}),
        ];
        let new = node.receive_peer_list(incoming, "node-b".to_owned());
        assert_eq!(new, 0);
    }

    #[test]
    fn age_entries_expires_peers() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());

        for _ in 0..10 {
            node.age_entries();
        }

        assert_eq!(node.known_peer_count(), 0);
    }

    #[test]
    fn does_not_target_self() {
        let mut node = GossipNode::new("node-a".to_owned(), String::new());
        node.add_peer("node-b".to_owned(), "https://sqs.fake/b".to_owned());

        for _ in 0..20 {
            let target = node.pick_gossip_target();

            if let Some(chosen) = target {
                assert_ne!(chosen, "node-a".to_owned());
            }
        }
    }
}
