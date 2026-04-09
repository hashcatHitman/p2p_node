// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module 3: Choking / Unchoking (BitTorrent-style)
//! =================================================
//!
//! Enforces reciprocity by limiting service to peers who contribute.
//!
//! How it works:
//!   1. Track how much each peer has contributed (data, messages, etc.).
//!   2. Periodically rank peers by contribution and unchoke the top N.
//!   3. Choke everyone else (stop serving them).
//!   4. Every `optimistic_interval` rounds, randomly unchoke one choked peer
//!      to give new peers a chance (optimistic unchoke).
//!
//! This is the mechanism BitTorrent uses to prevent free-riding.
//! Peers who only download and never upload will eventually get choked
//! by everyone and stall.
//!
//! Run the Algorithm Labs notebook before implementing this.

use core::fmt;
use core::fmt::Display;
use std::collections::{HashMap, HashSet};

use rand::prelude::IndexedRandom as _;

use crate::node::Id;

/// Tracks a single peer's contribution and choking state.
#[derive(Debug, Clone)]
struct PeerTracker {
    node_id: Id,
    contributed: u32,
    received: u32,
    is_choked: bool,
    is_interested: bool,
    rounds_choked: u32,
}

impl PeerTracker {
    const fn new(node_id: Id) -> Self {
        Self {
            node_id,
            contributed: 0,
            received: 0,
            is_choked: true,
            is_interested: true,
            rounds_choked: 0,
        }
    }

    /// How much they give vs. how much they take. Higher is better.
    fn reciprocity_ratio(&self) -> f64 {
        match self.received {
            0 => f64::from(self.contributed),
            _ => f64::from(self.contributed) / f64::from(self.received),
        }
    }
}

impl Display for PeerTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = if self.is_choked { "CHOKED" } else { "unchoked" };
        write!(
            f,
            "PeerTracker({}, contributed={}, received={}, {state})",
            self.node_id, self.contributed, self.received
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChokingNode {
    node_id: Id,
    max_unchoked: u8,
    optimistic_interval: u32,
    peers: HashMap<Id, PeerTracker>,
    round: u32,
    optimistic_peer: Option<Id>,
    log: Vec<String>,
}

impl ChokingNode {
    pub(crate) fn new(
        node_id: Id,
        max_unchoked: u8,
        optimistic_interval: u32,
    ) -> Self {
        Self {
            node_id,
            max_unchoked,
            optimistic_interval,
            peers: HashMap::new(),
            round: 0,
            optimistic_peer: None,
            log: Vec::new(),
        }
    }

    /// Register a new peer. New peers start choked.
    pub(crate) fn add_peer(&mut self, node_id: Id) {
        if !self.peers.contains_key(&node_id) {
            drop(
                self.peers
                    .insert(node_id.clone(), PeerTracker::new(node_id)),
            );
        }
    }

    /// Record that a peer contributed `units` to us.
    pub(crate) fn record_contribution(&mut self, from_peer: &Id, units: u32) {
        if let Some(peer) = self.peers.get_mut(from_peer) {
            peer.contributed += units;
        }
    }

    /// Record that we served `units` to a peer.
    #[expect(
        clippy::todo,
        reason = "record where? why does the assignment include this API?"
    )]
    fn record_serving(&self, to_peer: &Id, units: u32) {
        todo!()
    }

    /// Recalculate choke/unchoke decisions for this round.
    ///
    /// Algorithm:
    ///   1. Increment round counter.
    ///   2. Sort interested peers by reciprocity_ratio (descending).
    ///   3. Unchoke the top `max_unchoked - 1` peers.
    ///   4. Every `optimistic_interval` rounds, pick one random choked peer
    ///      as the optimistic unchoke (gives new peers a chance).
    ///   5. Choke everyone else.
    ///   6. Send CHOKE / UNCHOKE messages to peers whose state changed.
    ///      Log those changes via self._log.
    ///
    /// Note: Only unchoke peers where is_interested == True.
    pub(crate) fn run_choking_round(&mut self) {
        self.round += 1;

        let interested: Vec<(Id, PeerTracker)> = self
            .peers
            .iter()
            .filter(|&(_node_id, tracker)| tracker.is_interested)
            .map(|(node_id, tracker)| (node_id.clone(), tracker.clone()))
            .collect();

        if interested.is_empty() {
            return;
        }

        let mut ranked = interested.clone();
        #[expect(clippy::pattern_type_mismatch, reason = "todo later")]
        ranked.sort_by_key(|(_node_id, tracker)| tracker.contributed);
        ranked.reverse();

        let optimism = self.round.is_multiple_of(self.optimistic_interval);

        let regular_slots = self.max_unchoked - u8::from(optimism);
        let mut to_unchoke = HashSet::new();

        #[expect(clippy::pattern_type_mismatch, reason = "todo later")]
        for (peer, _tracker) in ranked.iter().take(regular_slots.into()) {
            let _: bool = to_unchoke.insert(peer);
        }

        if optimism {
            #[expect(clippy::pattern_type_mismatch, reason = "todo later")]
            let choked_interested: Vec<&(Id, PeerTracker)> = ranked
                .iter()
                .filter(|(node_id, _tracker)| !to_unchoke.contains(node_id))
                .collect();

            #[expect(clippy::pattern_type_mismatch, reason = "todo later")]
            if let Some((lucky_peer, _tracker)) =
                choked_interested.choose(&mut rand::rng())
            {
                self.optimistic_peer = Some(lucky_peer.clone());
            }
        }

        if let Some(ref lucky_peer) = self.optimistic_peer {
            let _: bool = to_unchoke.insert(lucky_peer);
        }

        for (node_id, _) in interested {
            if let Some(peer) = self.peers.get_mut(&node_id) {
                let old_choked = peer.is_choked;
                peer.is_choked = !to_unchoke.contains(&node_id);

                if peer.is_choked {
                    peer.rounds_choked += 1;
                }

                if old_choked && !peer.is_choked {
                    self.log.push(format!(
                        "  Round {}: UNCHOKED {} (contributed={})",
                        self.round, node_id, peer.contributed
                    ));
                } else if !old_choked && peer.is_choked {
                    self.log.push(format!(
                        "  Round {}: CHOKED {} (contributed={})",
                        self.round, node_id, peer.contributed
                    ));
                }
            }
        }
    }

    /// Return node_ids of all currently unchoked peers.
    fn get_unchoked_peers(&self) -> Vec<Id> {
        self.peers
            .iter()
            .filter(|&(_id, state)| !state.is_choked)
            .map(|(id, _state)| id.clone())
            .collect()
    }

    /// Return node_ids of all currently choked peers.
    fn get_choked_peers(&self) -> Vec<Id> {
        self.peers
            .iter()
            .filter(|&(_id, state)| state.is_choked)
            .map(|(id, _state)| id.clone())
            .collect()
    }

    pub(crate) fn flush_log(&mut self) -> Vec<String> {
        let messages = self.log.clone();
        self.log.clear();
        messages
    }
}

impl Display for ChokingNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unchoked = self.get_unchoked_peers();
        write!(f, "ChokingNode({}, unchoked={:?})", self.node_id, unchoked)
    }
}

#[expect(clippy::missing_panics_doc, reason = "tests tend to do that")]
#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::algorithms::choking::ChokingNode;
    use crate::node::Id;

    #[test]
    fn new_peer_starts_choked() {
        let mut node = ChokingNode::new(Id::new("node-a".to_owned()), 2, 3);
        node.add_peer(Id::new("node-b".to_owned()));
        node.add_peer(Id::new("node-c".to_owned()));
        let unchoked = node.get_unchoked_peers();
        assert!(!unchoked.contains(&Id::new("node-b".to_owned())));
    }

    #[test]
    fn top_contributor_gets_unchoked() {
        let mut node = ChokingNode::new(Id::new("node-a".to_owned()), 1, 999);
        node.add_peer(Id::new("node-b".to_owned()));
        node.add_peer(Id::new("node-c".to_owned()));

        for _ in 0..10 {
            node.record_contribution(&Id::new("node-b".to_owned()), 5);
        }

        node.run_choking_round();

        let unchoked = node.get_unchoked_peers();
        assert!(unchoked.contains(&Id::new("node-b".to_owned())));
        assert!(!unchoked.contains(&Id::new("node-c".to_owned())));
    }

    #[test]
    fn max_unchoked_limit_respected() {
        let mut node = ChokingNode::new(Id::new("node-a".to_owned()), 2, 999);

        for peer in [
            &Id::new("node-b".to_owned()),
            &Id::new("node-c".to_owned()),
            &Id::new("node-d".to_owned()),
            &Id::new("node-e".to_owned()),
        ] {
            node.add_peer(peer.clone());
            node.record_contribution(peer, 1);
        }

        node.run_choking_round();

        let unchoked = node.get_unchoked_peers();
        assert!(unchoked.len() <= 2);
    }

    #[test]
    fn choked_and_unchoked_disjoint() {
        let mut node = ChokingNode::new(Id::new("node-a".to_owned()), 2, 999);

        for peer in [
            &Id::new("node-b".to_owned()),
            &Id::new("node-c".to_owned()),
            &Id::new("node-d".to_owned()),
        ] {
            node.add_peer(peer.clone());
            node.record_contribution(peer, 1);
        }

        node.run_choking_round();

        let unchoked: HashSet<Id> =
            node.get_unchoked_peers().into_iter().collect();
        let choked: HashSet<Id> = node.get_choked_peers().into_iter().collect();

        assert!(unchoked.is_disjoint(&choked));
    }

    #[test]
    fn free_rider_stays_choked() {
        let mut node = ChokingNode::new(Id::new("node-a".to_owned()), 2, 999);

        for peer in [
            Id::new("node-b".to_owned()),
            Id::new("node-c".to_owned()),
            Id::new("node-d".to_owned()),
        ] {
            node.add_peer(peer);
        }

        node.record_contribution(&Id::new("node-b".to_owned()), 10);
        node.record_contribution(&Id::new("node-c".to_owned()), 8);

        for _ in 0..3 {
            node.run_choking_round();
        }

        let choked = node.get_choked_peers();

        assert!(choked.contains(&Id::new("node-d".to_owned())));
    }

    #[test]
    fn optimistic_unchoke_occurs() {
        let mut node = ChokingNode::new(Id::new("node-a".to_owned()), 1, 1);

        for peer in [
            Id::new("node-b".to_owned()),
            Id::new("node-c".to_owned()),
            Id::new("node-d".to_owned()),
        ] {
            node.add_peer(peer);
        }

        node.run_choking_round();

        let unchoked = node.get_unchoked_peers();

        assert!(!unchoked.is_empty());
    }
}
