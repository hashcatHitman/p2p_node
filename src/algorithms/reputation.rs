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
use std::collections::HashMap;

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
    pub fn recalculate_trust(&mut self) {
        let raw = 0.1_f64.mul_add(
            self.reciprocity(),
            0.6_f64.mul_add(self.accuracy(), 0.3_f64 * self.uptime()),
        );

        let decay_factor = (f64::from(self.reports_total)
            + f64::from(self.heartbeats_total) / 20.0_f64)
            .min(1.0_f64);

        self.trust_score =
            decay_factor.mul_add(raw, (1.0_f64 - decay_factor) * 0.5_f64);
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

/// Tracks reputation for all known peers and computes weighted votes.
#[derive(Debug, Clone)]
pub struct ReputationNode {
    node_id: String,
    peers: HashMap<String, ReputationRecord>,
    log: Vec<String>,
}

impl ReputationNode {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            peers: HashMap::new(),
            log: Vec::new(),
        }
    }

    /// Register a new peer with a neutral trust score (0.5).
    pub fn add_peer(&self, node_id: String) {
        todo!()
    }

    /// Record whether a peer's VIEW_EVENT report matched the audit majority.
    pub fn record_report(&self, peer_id: String, was_accurate: bool) {
        todo!()
    }

    /// Record a heartbeat event (whether the peer responded to a PING).
    pub fn record_heartbeat(&self, peer_id: String, responded: bool) {
        todo!()
    }

    /// Record that a peer contributed `units` of data/messages to us.
    pub fn record_contribution(&self, peer_id: String, units: u32) {
        todo!()
    }

    /// Record that a peer consumed `units` from us.
    pub fn record_consumption(&self, peer_id: String, units: u32) {
        todo!()
    }

    /// Recalculate trust scores for all peers.
    pub fn update_all_scores(&self) {
        todo!()
    }

    /// Compute a reputation-weighted majority vote over reported counts.
    ///
    /// Each vote is weighted by that peer's trust score. Group similar
    /// counts together (votes within 5% are considered the same). The
    /// group with the highest total weight wins.
    ///
    /// Args:
    ///     votes:   {peer_id: their_reported_count}
    ///     verbose: Print vote breakdown if True
    ///
    /// Returns:
    ///     (winning_count, confidence) where confidence is the winning
    ///     group's share of total weighted votes (0.0 to 1.0)
    pub fn weighted_majority_vote(
        &self,
        votes: HashMap<String, u32>,
        verbose: bool,
    ) -> (u32, f64) {
        todo!()
    }

    /// Return all peers sorted by trust score, highest first.
    pub fn get_ranked_peers(&self) -> Vec<ReputationRecord> {
        todo!()
    }

    pub fn flush_log(&mut self) -> Vec<String> {
        let messages = self.log.clone();
        self.log.clear();
        messages
    }
}

#[expect(clippy::missing_panics_doc, reason = "tests tend to do that")]
#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::algorithms::reputation::ReputationNode;

    #[test]
    fn new_peer_neutral_trust() {
        let node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());

        let ranked = node.get_ranked_peers();
        assert_eq!(ranked.len(), 1);
        let score = ranked[0].trust_score;
        assert!(0.3 <= score);
        assert!(score <= 0.7);
    }

    #[test]
    fn accurate_peer_gains_trust() {
        let node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());

        for _ in 0..10 {
            node.record_report("node-b".to_owned(), true);
        }

        node.update_all_scores();

        let ranked = node.get_ranked_peers();
        let score = ranked[0].trust_score;
        assert!(score > 0.5);
    }

    #[test]
    fn inaccurate_peer_loses_trust() {
        let node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());

        for _ in 0..10 {
            node.record_report("node-b".to_owned(), false);
        }

        node.update_all_scores();

        let ranked = node.get_ranked_peers();
        let score = ranked[0].trust_score;
        assert!(score < 0.5);
    }

    #[test]
    fn ranking_order() {
        let node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());
        node.add_peer("node-c".to_owned());

        for _ in 0..10 {
            node.record_report("node-b".to_owned(), true);
            node.record_report("node-c".to_owned(), false);
        }

        node.update_all_scores();

        let ranked = node.get_ranked_peers();
        assert_eq!(ranked[0].node_id, "node-b");
        assert_eq!(ranked[ranked.len()].node_id, "node-c");
    }

    #[test]
    fn weighted_vote_honest_beats_liar() {
        let node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());
        node.add_peer("node-c".to_owned());

        for _ in 0..10 {
            node.record_report("node-b".to_owned(), true);
            node.record_report("node-c".to_owned(), false);
        }

        node.update_all_scores();

        let mut votes = HashMap::new();
        let _: Option<u32> = votes.insert("node-b".to_owned(), 100);
        let _: Option<u32> = votes.insert("node-c".to_owned(), 9999);

        let (result, confidence) = node.weighted_majority_vote(votes, false);

        assert_eq!(result, 100);
        assert!(0.0 < confidence);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn confidence_higher_when_unanimous() {
        let node = ReputationNode::new("node-a".to_owned());

        for peer in ["node-b", "node-c", "node-d"] {
            node.add_peer(peer.to_owned());
            node.record_report(peer.to_owned(), true);
        }

        node.update_all_scores();

        let mut unaninmous_votes = HashMap::new();
        let _: Option<u32> = unaninmous_votes.insert("node-b".to_owned(), 100);
        let _: Option<u32> = unaninmous_votes.insert("node-c".to_owned(), 100);
        let _: Option<u32> = unaninmous_votes.insert("node-d".to_owned(), 100);

        let mut split_votes = HashMap::new();
        let _: Option<u32> = split_votes.insert("node-b".to_owned(), 100);
        let _: Option<u32> = split_votes.insert("node-c".to_owned(), 100);
        let _: Option<u32> = split_votes.insert("node-d".to_owned(), 999);

        let (_, unanimous_confidence) =
            node.weighted_majority_vote(unaninmous_votes, false);

        let (_, split_confidence) =
            node.weighted_majority_vote(split_votes, false);

        assert!(unanimous_confidence > split_confidence);
    }

    #[test]
    fn heartbeat_uptime_affects_trust() {
        let node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-reliable".to_owned());
        node.add_peer("node-offline".to_owned());

        for _ in 0..10 {
            node.record_heartbeat("node-reliable".to_owned(), true);
            node.record_heartbeat("node-offline".to_owned(), false);
        }

        node.update_all_scores();

        let ids: Vec<String> = node
            .get_ranked_peers()
            .into_iter()
            .map(|record| record.node_id)
            .collect();

        let best_id = ids.first().unwrap();

        assert_eq!(best_id, "node-reliable");
    }
}
