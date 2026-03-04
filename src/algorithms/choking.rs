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
use std::collections::HashMap;

/// Tracks a single peer's contribution and choking state.
#[derive(Debug, Clone)]
pub struct PeerTracker {
    node_id: String,
    contributed: u32,
    received: u32,
    is_choked: bool,
    is_interested: bool,
    rounds_choked: u32,
}

impl PeerTracker {
    pub const fn new(node_id: String) -> Self {
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
    pub fn reciprocity_ratio(&self) -> f64 {
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
pub struct ChokingNode {
    node_id: String,
    max_unchoked: u8,
    optimistic_interval: u32,
    peers: HashMap<String, PeerTracker>,
    round: u32,
    optimistic_peer: Option<String>,
    log: Vec<String>,
}

impl ChokingNode {
    pub fn new(
        node_id: String,
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
    pub fn add_peer(&self, node_id: String, interested: bool) {
        todo!()
    }

    /// Record that a peer contributed `units` to us.
    pub fn record_contribution(&self, from_peer: String, units: u32) {
        todo!()
    }

    /// Record that we served `units` to a peer.
    pub fn record_serving(&self, to_peer: String, units: u32) {
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
    pub fn run_choking_round(&self) {
        todo!()
    }

    /// Return node_ids of all currently unchoked peers.
    pub fn get_unchoked_peers(&self) -> Vec<String> {
        todo!()
    }

    /// Return node_ids of all currently choked peers.
    pub fn get_choked_peers(&self) -> Vec<String> {
        todo!()
    }

    pub fn flush_log(&mut self) -> Vec<String> {
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
