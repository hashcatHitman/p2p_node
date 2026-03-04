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
#![expect(
    clippy::doc_markdown,
    reason = "the docs are as they were meant to be, for now"
)]

use core::fmt;
use core::fmt::Display;

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
