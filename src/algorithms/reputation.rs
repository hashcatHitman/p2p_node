// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module 4: Reputation Scoring
//! ==============================
//!
//! Tracks per-peer trust and computes reputation-weighted majority votes.
//!
//! How it works:
//!   1. Each peer has a trust score (0.0 = untrusted, 1.0 = fully trusted).
//!   2. Scores are built from three signals:
//!        - Accuracy:     Did their reported counts match the majority?
//!        - Uptime:       Do they respond to heartbeats reliably?
//!        - Reciprocity:  Do they contribute as much as they consume?
//!   3. For each audit, a weighted majority vote is computed: each peer's
//!      reported count is weighted by their trust score. The count with the
//!      most total weight wins.
//!   4. Scores are recalculated periodically and decay slightly toward 0.5
//!      to prevent scores from getting permanently locked.
//!
//! Run the Algorithm Labs notebook to see weighted voting in action
//! before implementing it here.

use core::fmt;
use core::fmt::Display;

/// Tracks reputation metrics for a single peer.
#[derive(Debug, Clone)]
pub struct ReputationRecord {
    node_id: String,
    reports_total: u32,
    reports_accurate: u32,
    heartbeats_total: u32,
    heartbeats_responded: u32,
    contributions: u32,
    consumptions: u32,
    decay_factor: f64,
    trust_score: f64,
}

impl ReputationRecord {
    pub const fn new(node_id: String) -> Self {
        Self {
            node_id,
            reports_total: 0,
            reports_accurate: 0,
            heartbeats_total: 0,
            heartbeats_responded: 0,
            contributions: 0,
            consumptions: 0,
            decay_factor: 0.95,
            trust_score: 0.5,
        }
    }

    pub fn accuracy(&self) -> f64 {
        match self.reports_total {
            0 => 0.5,
            _ => {
                f64::from(self.reports_accurate) / f64::from(self.reports_total)
            }
        }
    }

    pub fn uptime(&self) -> f64 {
        match self.heartbeats_total {
            0 => 0.5,
            _ => {
                f64::from(self.heartbeats_responded)
                    / f64::from(self.heartbeats_total)
            }
        }
    }

    pub fn reciprocity(&self) -> f64 {
        let total = self.contributions + self.consumptions;
        match total {
            0 => 0.5,
            _ => f64::from(self.contributions) / f64::from(total),
        }
    }

    pub const fn trust_score(&self) -> f64 {
        self.trust_score
    }

    /// Recalculate trust score from metrics, with decay toward neutral.
    ///
    /// Suggested formula:
    ///     raw = 0.6 * accuracy() + 0.3 * uptime() + 0.1 * reciprocity()
    ///     _trust_score = decay_factor * raw + (1 - decay_factor) * 0.5
    ///
    /// The decay_factor pulls scores back toward 0.5 each round,
    /// preventing permanent entrenchment (good or bad).
    pub fn recalculate_trust(&self) {
        todo!()
    }
}

impl Display for ReputationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ReputationRecord({}, trust={:.3}, accuracy={:.0}%)",
            self.node_id,
            self.trust_score,
            self.accuracy()
        )
    }
}
