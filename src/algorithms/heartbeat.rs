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
#![expect(
    clippy::doc_markdown,
    clippy::doc_paragraphs_missing_punctuation,
    reason = "the docs are as they were meant to be, for now"
)]

use core::fmt;
use core::fmt::Display;

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
    last_pong_round: u8,
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
