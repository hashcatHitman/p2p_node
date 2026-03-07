// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module 2: Heartbeat / Liveness Detection
//! =========================================
//!
//! Detects when peers go offline using a PING/PONG protocol.
//!
//! How it works:
//!   1. Periodically send PING to every known peer.
//!   2. Peers that respond with PONG before the next round are ALIVE.
//!   3. Peers that miss `grace_period` consecutive rounds become SUSPECT.
//!   4. Peers that miss `miss_threshold` consecutive rounds become DEAD.
//!
//! State machine:
//!     ALIVE -> SUSPECT (after grace_period misses)
//!     SUSPECT -> DEAD  (after miss_threshold misses)
//!     SUSPECT -> ALIVE (if PONG received while suspect)
//!
//! Run the Algorithm Labs notebook to see the state machine in action
//! before implementing it here.

use core::fmt;
use core::fmt::Display;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PeerStatus {
    Alive,
    Suspect,
    Dead,
}

impl Display for PeerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match *self {
            Self::Alive => "ALIVE",
            Self::Suspect => "SUSPECT",
            Self::Dead => "DEAD",
        };
        write!(f, "{string}")
    }
}

/// Tracked state for a single monitored peer.
#[derive(Debug, Clone)]
pub struct PeerState {
    node_id: String,
    status: PeerStatus,
    consecutive_misses: u8,
    last_pong_round: u32,
    total_pings_sent: u8,
    total_pongs_received: u8,
}

impl PeerState {
    pub const fn new(node_id: String) -> Self {
        Self {
            node_id,
            status: PeerStatus::Alive,
            consecutive_misses: 0,
            last_pong_round: 0,
            total_pings_sent: 0,
            total_pongs_received: 0,
        }
    }

    pub fn response_rate(&self) -> f64 {
        match self.total_pings_sent {
            0 => 1.0,
            _ => {
                f64::from(self.total_pongs_received)
                    / f64::from(self.total_pings_sent)
            }
        }
    }
}

impl Display for PeerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PeerState({}, {}, misses={}, rate={:.0}%)",
            self.node_id,
            self.status,
            self.consecutive_misses,
            self.response_rate()
        )
    }
}

#[derive(Debug, Clone)]
pub struct HeartbeatNode {
    node_id: String,
    miss_threshold: u8,
    grace_period: u8,
    peers: HashMap<String, PeerState>,
    log: Vec<String>,
}

impl HeartbeatNode {
    pub fn new(node_id: String, miss_threshold: u8, grace_period: u8) -> Self {
        Self {
            node_id,
            miss_threshold,
            grace_period,
            peers: HashMap::new(),
            log: Vec::new(),
        }
    }

    /// Register a new peer to monitor (start in ALIVE state).
    pub fn add_peer(&mut self, node_id: String) {
        if !self.peers.contains_key(&node_id) {
            drop(self.peers.insert(node_id.clone(), PeerState::new(node_id)));
        }
    }

    /// Record that we are sending a PING to every ALIVE and SUSPECT peer.
    ///
    /// Increments total_pings_sent for each peer pinged.
    /// Dead peers are skipped.
    ///
    /// Returns:
    ///     List of peer_ids that should receive a PING this round.
    pub fn send_pings(&mut self, current_round: u32) -> Vec<String> {
        let mut pinged = Vec::new();

        for (peer_id, state) in &mut self.peers {
            if state.status != PeerStatus::Dead {
                state.total_pings_sent += 1;
                pinged.push(peer_id.clone());
            }
        }
        pinged
    }

    /// Process a PONG response from a peer.
    ///
    /// Update the peer's state:
    ///   - Reset consecutive_misses to 0
    ///   - Set status back to ALIVE
    ///   - Update last_pong_round and total_pongs_received
    ///
    /// Args:
    ///     from_node:     node_id of the peer who replied
    ///     current_round: current poll round number
    pub fn receive_pong(&mut self, from_node: &str, current_round: u32) {
        if let Some(peer) = self.peers.get_mut(from_node) {
            peer.total_pongs_received += 1;
            peer.last_pong_round = current_round;
            let old = peer.status;
            peer.consecutive_misses = 0;
            peer.status = PeerStatus::Alive;

            if old != PeerStatus::Alive {
                self.log.push(format!("  Round {current_round}: {from_node} recovered ({old} -> ALIVE)"));
            }
        }
    }

    /// Record that a PING got no PONG from this peer this round.
    ///
    /// Update the peer's state machine:
    ///   - Increment consecutive_misses
    ///   - If misses >= miss_threshold -> DEAD
    ///   - If misses >= grace_period   -> SUSPECT
    ///   - Log any status transition
    ///
    /// Args:
    ///     peer_id:       node_id of the non-responding peer
    ///     current_round: current poll round number
    pub fn record_miss(&mut self, peer_id: String, current_round: u32) {
        if let Some(peer) = self.peers.get_mut(&peer_id) {
            if peer.status == PeerStatus::Dead {
                return;
            }

            peer.consecutive_misses += 1;

            let old = peer.status;

            if peer.consecutive_misses >= self.miss_threshold {
                peer.status = PeerStatus::Dead;
            } else if peer.consecutive_misses >= self.grace_period {
                peer.status = PeerStatus::Suspect;
            }

            if peer.status != old {
                self.log.push(format!("  Round {current_round}: {peer_id} {old} -> {} (misses={})",peer.status, peer.consecutive_misses));
            }
        }
    }

    /// Return node_ids of all ALIVE peers.
    pub fn get_alive_peers(&self) -> Vec<String> {
        self.peers
            .iter()
            .filter(|&(_id, state)| state.status == PeerStatus::Alive)
            .map(|(id, _state)| id.clone())
            .collect()
    }

    /// Return node_ids of all SUSPECT peers.
    pub fn get_suspect_peers(&self) -> Vec<String> {
        self.peers
            .iter()
            .filter(|&(_id, state)| state.status == PeerStatus::Suspect)
            .map(|(id, _state)| id.clone())
            .collect()
    }

    /// Return node_ids of all DEAD peers.
    pub fn get_dead_peers(&self) -> Vec<String> {
        self.peers
            .iter()
            .filter(|&(_id, state)| state.status == PeerStatus::Dead)
            .map(|(id, _state)| id.clone())
            .collect()
    }

    /// Remove DEAD peers from the tracking table.
    pub fn prune_dead(&mut self) {
        let dead = self.get_dead_peers();

        for peer in dead {
            drop(self.peers.remove(&peer));
        }
    }

    pub fn flush_log(&mut self) -> Vec<String> {
        let messages = self.log.clone();
        self.log.clear();
        messages
    }
}

impl Display for HeartbeatNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HeartbeatNode({}, alive={:?}, dead={:?})",
            self.node_id,
            self.get_alive_peers(),
            self.get_dead_peers(),
        )
    }
}

#[expect(clippy::missing_panics_doc, reason = "tests tend to do that")]
#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::algorithms::heartbeat::HeartbeatNode;

    #[test]
    fn new_peer_is_alive() {
        let mut node = HeartbeatNode::new("node-a".to_owned(), 3, 2);
        node.add_peer("node-b".to_owned());
        let living = node.get_alive_peers();
        assert!(living.contains(&"node-b".to_owned()));
    }

    #[test]
    fn send_pings_returns_alive_peers() {
        let mut node = HeartbeatNode::new("node-a".to_owned(), 3, 2);
        node.add_peer("node-b".to_owned());
        node.add_peer("node-c".to_owned());
        let pinged = node.send_pings(1);
        assert!(pinged.contains(&"node-b".to_owned()));
        assert!(pinged.contains(&"node-c".to_owned()));
    }

    #[test]
    fn pong_keeps_peer_alive() {
        let mut node = HeartbeatNode::new("node-a".to_owned(), 3, 2);
        node.add_peer("node-b".to_owned());
        drop(node.send_pings(1));
        node.receive_pong("node-b", 1);
        let living = node.get_alive_peers();
        assert!(living.contains(&"node-b".to_owned()));
    }

    #[test]
    fn grace_period_misses_produces_suspect() {
        const GRACE: u8 = 2;
        let mut node = HeartbeatNode::new("node-a".to_owned(), 5, GRACE);
        node.add_peer("node-b".to_owned());

        for round in 1..=GRACE {
            drop(node.send_pings(round.into()));
            node.record_miss("node-b".to_owned(), round.into());
        }

        let suspicious = node.get_suspect_peers();
        assert!(suspicious.contains(&"node-b".to_owned()));

        let dead = node.get_dead_peers();
        assert!(!dead.contains(&"node-b".to_owned()));
    }

    #[test]
    fn threshold_misses_produces_dead() {
        const THRESHOLD: u8 = 3;
        let mut node = HeartbeatNode::new("node-a".to_owned(), THRESHOLD, 2);
        node.add_peer("node-b".to_owned());

        for round in 1..=THRESHOLD {
            drop(node.send_pings(round.into()));
            node.record_miss("node-b".to_owned(), round.into());
        }

        let dead = node.get_dead_peers();
        assert!(dead.contains(&"node-b".to_owned()));
    }

    #[test]
    fn pong_after_suspect_restores_alive() {
        let mut node = HeartbeatNode::new("node-a".to_owned(), 5, 2);
        node.add_peer("node-b".to_owned());

        for round in 1..=2 {
            drop(node.send_pings(round));
            node.record_miss("node-b".to_owned(), round);
        }

        let suspicious = node.get_suspect_peers();
        assert!(suspicious.contains(&"node-b".to_owned()));

        node.receive_pong("node-b", 3);

        let living = node.get_alive_peers();
        assert!(living.contains(&"node-b".to_owned()));

        let suspicious = node.get_suspect_peers();
        assert!(!suspicious.contains(&"node-b".to_owned()));
    }

    #[test]
    fn dead_peers_not_pinged() {
        let mut node = HeartbeatNode::new("node-a".to_owned(), 3, 2);
        node.add_peer("node-b".to_owned());

        for round in 1..4 {
            drop(node.send_pings(round));
            node.record_miss("node-b".to_owned(), round);
        }

        let dead = node.get_dead_peers();
        assert!(dead.contains(&"node-b".to_owned()));

        let pinged = node.send_pings(4);
        assert!(!pinged.contains(&"node-b".to_owned()));
    }

    #[test]
    fn lists_are_disjoint() {
        let mut node = HeartbeatNode::new("node-a".to_owned(), 3, 2);
        node.add_peer("node-b".to_owned());
        node.add_peer("node-c".to_owned());
        node.add_peer("node-d".to_owned());

        for round in 1..3 {
            node.record_miss("node-b".to_owned(), round);
        }

        for round in 1..4 {
            node.record_miss("node-c".to_owned(), round);
        }

        let living: HashSet<String> =
            node.get_alive_peers().into_iter().collect();
        let suspicious: HashSet<String> =
            node.get_suspect_peers().into_iter().collect();
        let dead: HashSet<String> = node.get_dead_peers().into_iter().collect();

        assert!(living.is_disjoint(&suspicious));
        assert!(living.is_disjoint(&dead));
        assert!(suspicious.is_disjoint(&dead));
    }
}
