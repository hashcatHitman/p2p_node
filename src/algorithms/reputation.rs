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

use core::cmp::Ordering;
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
    pub fn add_peer(&mut self, node_id: String) {
        if !self.peers.contains_key(&node_id) {
            drop(
                self.peers
                    .insert(node_id.clone(), ReputationRecord::new(node_id)),
            );
        }
    }

    /// Record whether a peer's VIEW_EVENT report matched the audit majority.
    pub fn record_report(&mut self, peer_id: &str, was_accurate: bool) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.reports_total += 1;

            if was_accurate {
                peer.reports_accurate += 1;
            }
        }
    }

    /// Record a heartbeat event (whether the peer responded to a PING).
    pub fn record_heartbeat(&mut self, peer_id: &str, responded: bool) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.heartbeats_total += 1;

            if responded {
                peer.heartbeats_responded += 1;
            }
        }
    }

    /// Record that a peer contributed `units` of data/messages to us.
    pub fn record_contribution(&mut self, peer_id: &str, units: u32) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.contributions += 1;
        }
    }

    /// Record that a peer consumed `units` from us.
    pub fn record_consumption(&mut self, peer_id: &str, units: u32) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.consumptions += 1;
        }
    }

    /// Recalculate trust scores for all peers.
    pub fn update_all_scores(&mut self) {
        for record in self.peers.values_mut() {
            record.recalculate_trust();
        }
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
        votes: &HashMap<String, u64>,
        verbose: bool,
    ) -> (Option<u64>, f64) {
        if votes.is_empty() {
            return (None, 0.0);
        }

        let mut weighted = HashMap::new();
        let mut total_weight = 0.0;

        for (peer_id, value) in votes {
            let weight = self
                .peers
                .get(peer_id)
                .map_or(0.5_f64, |peer| peer.trust_score);

            match weighted.get_mut(&value) {
                Some(stored) => *stored += weight,
                None => {
                    let _: Option<f64> = weighted.insert(value, weight);
                }
            }

            total_weight += weight;
        }

        let mut weighted: Vec<(u64, TotalCmpF64)> = weighted
            .into_iter()
            .map(|(&value, weight)| (value, TotalCmpF64(weight)))
            .collect();

        if let Some(&(winner, win_weight)) =
            weighted.iter().max_by_key(|&&(value, weight)| weight)
        {
            let confidence = if total_weight > 0.0 {
                win_weight.0 / total_weight
            } else {
                0.0
            };

            if verbose {
                println!("  Votes: {votes:?}");
                print!("  Weights: ");
                for (peer_id, votes) in votes {
                    let weight = self
                        .peers
                        .get(peer_id)
                        .map_or(0.5, |peer| peer.trust_score);

                    print!("{peer_id}={weight:.2}  ");
                }
                println!();
                println!("  Winner: {winner} (confidence: {confidence:.2})");
            }

            return (Some(winner), confidence);
        }
        (None, 0.0)
    }

    /// Return all peers sorted by trust score, highest first.
    pub fn get_ranked_peers(&self) -> Vec<ReputationRecord> {
        let mut peer_ratings: Vec<ReputationRecord> =
            self.peers.values().cloned().collect();
        peer_ratings.sort_by_key(|record| TotalCmpF64(record.trust_score()));
        peer_ratings.reverse();
        peer_ratings
    }

    pub fn flush_log(&mut self) -> Vec<String> {
        let messages = self.log.clone();
        self.log.clear();
        messages
    }
}

#[derive(Debug, Clone, Copy)]
struct TotalCmpF64(f64);

impl PartialEq for TotalCmpF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
    }
}

impl Eq for TotalCmpF64 {}

impl PartialOrd for TotalCmpF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TotalCmpF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[expect(clippy::missing_panics_doc, reason = "tests tend to do that")]
#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::algorithms::reputation::ReputationNode;

    #[test]
    fn new_peer_neutral_trust() {
        let mut node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());

        let ranked = node.get_ranked_peers();
        assert_eq!(ranked.len(), 1);
        let score = ranked[0].trust_score;
        assert!(0.3 <= score);
        assert!(score <= 0.7);
    }

    #[test]
    fn accurate_peer_gains_trust() {
        let mut node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());

        for _ in 0..10 {
            node.record_report("node-b", true);
        }

        node.update_all_scores();

        let ranked = node.get_ranked_peers();
        let score = ranked[0].trust_score;
        assert!(score > 0.5);
    }

    #[test]
    fn inaccurate_peer_loses_trust() {
        let mut node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());

        for _ in 0..10 {
            node.record_report("node-b", false);
        }

        node.update_all_scores();

        let ranked = node.get_ranked_peers();
        let score = ranked[0].trust_score;
        assert!(score < 0.5);
    }

    #[test]
    fn ranking_order() {
        let mut node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());
        node.add_peer("node-c".to_owned());

        for _ in 0..10 {
            node.record_report("node-b", true);
            node.record_report("node-c", false);
        }

        node.update_all_scores();

        let ranked = node.get_ranked_peers();
        assert_eq!(ranked[0].node_id, "node-b");
        assert_eq!(ranked[ranked.len() - 1].node_id, "node-c");
    }

    #[test]
    fn weighted_vote_honest_beats_liar() {
        let mut node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-b".to_owned());
        node.add_peer("node-c".to_owned());

        for _ in 0..10 {
            node.record_report("node-b", true);
            node.record_report("node-c", false);
        }

        node.update_all_scores();

        let mut votes = HashMap::new();
        let _: Option<u64> = votes.insert("node-b".to_owned(), 100);
        let _: Option<u64> = votes.insert("node-c".to_owned(), 9999);

        let (result, confidence) = node.weighted_majority_vote(&votes, false);

        assert_eq!(result, Some(100));
        assert!(0.0 < confidence);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn confidence_higher_when_unanimous() {
        let mut node = ReputationNode::new("node-a".to_owned());

        for peer in ["node-b", "node-c", "node-d"] {
            node.add_peer(peer.to_owned());
            node.record_report(peer, true);
        }

        node.update_all_scores();

        let mut unaninmous_votes = HashMap::new();
        let _: Option<u64> = unaninmous_votes.insert("node-b".to_owned(), 100);
        let _: Option<u64> = unaninmous_votes.insert("node-c".to_owned(), 100);
        let _: Option<u64> = unaninmous_votes.insert("node-d".to_owned(), 100);

        let mut split_votes = HashMap::new();
        let _: Option<u64> = split_votes.insert("node-b".to_owned(), 100);
        let _: Option<u64> = split_votes.insert("node-c".to_owned(), 100);
        let _: Option<u64> = split_votes.insert("node-d".to_owned(), 999);

        let (_, unanimous_confidence) =
            node.weighted_majority_vote(&unaninmous_votes, false);

        let (_, split_confidence) =
            node.weighted_majority_vote(&split_votes, false);

        assert!(unanimous_confidence > split_confidence);
    }

    #[test]
    fn heartbeat_uptime_affects_trust() {
        let mut node = ReputationNode::new("node-a".to_owned());
        node.add_peer("node-reliable".to_owned());
        node.add_peer("node-offline".to_owned());

        for _ in 0..10 {
            node.record_heartbeat("node-reliable", true);
            node.record_heartbeat("node-offline", false);
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
