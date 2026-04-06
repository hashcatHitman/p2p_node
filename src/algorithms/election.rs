// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Leader Election: Reputation-Weighted Bully Algorithm
//! ======================================================
//!
//! Elects a single Payment Server from the bot pool.
//!
//! Only bots can WIN the election, but any node can TRIGGER one
//! when heartbeat detects the current Payment Server is dead.
//!
//! Algorithm:
//!   1.  Any node detects leader is DEAD via heartbeat.
//!   2.  Node sends ELECTION(term, reputation) to all bots with higher reputation.
//!   3.  If a higher-reputation bot responds ELECTION_OK, the sender backs off.
//!   4.  If nobody responds within `election_timeout`, the sender declares itself
//!       COORDINATOR (if it is a bot) or the highest-reputation bot it knows about.
//!   5.  COORDINATOR is broadcast to all peers.
//!
//! Term numbers prevent stale elections from overriding newer ones.

use core::fmt;
use core::fmt::Display;
use std::time;

use crate::node::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElectionStatus {
    Follower,
    Candidate,
    Leader,
}

impl Display for ElectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match *self {
            Self::Follower => "FOLLOWER",
            Self::Candidate => "CANDIDATE",
            Self::Leader => "LEADER",
        };
        write!(f, "{string}")
    }
}

#[derive(Debug, Clone)]
pub struct ElectionNode {
    node_id: Id,
    bot_ids: Vec<Id>,
    is_bot: bool,
    state: ElectionStatus,
    current_leader: Option<Id>,
    term: u64,
    election_in_progress: bool,
    election_start: time::Instant,
    got_ok: bool,
    election_timeout: time::Duration,
    last_leader_contact: time::Instant,
    leader_timeout: time::Duration,
    get_reputation: fn(Id) -> f64,
    get_alive_peers: fn() -> Vec<Id>,
    log: Vec<String>,
}

impl ElectionNode {
    pub fn new(
        node_id: Id,
        bot_ids: Vec<Id>,
        get_reputation: fn(Id) -> f64,
        get_alive_peers: fn() -> Vec<Id>,
    ) -> Self {
        let is_bot = bot_ids.contains(&node_id);
        Self {
            node_id,
            bot_ids,
            is_bot,
            state: ElectionStatus::Follower,
            current_leader: None,
            term: 0,
            election_in_progress: false,
            election_start: time::Instant::now(),
            got_ok: false,
            election_timeout: time::Duration::from_secs(8),
            last_leader_contact: time::Instant::now(),
            leader_timeout: time::Duration::from_secs(15),
            get_reputation,
            get_alive_peers,
            log: Vec::new(),
        }
    }

    pub fn check_leader(&self) -> bool {
        todo!()
    }

    pub fn start_election(&self) -> Vec<(Id, u64, f64)> {
        todo!()
    }

    pub fn receive_election(
        &self,
        sender: Id,
        term: u64,
        sender_reputation: f64,
    ) -> Option<(Id, u64, f64)> {
        todo!()
    }

    pub fn receive_election_ok(
        &self,
        sender: Id,
        term: u64,
        sender_reputation: f64,
    ) {
        todo!()
    }

    pub fn receive_coordinator(&self, sender: Id, term: u64) -> bool {
        todo!()
    }

    pub fn check_election_timeout(&self) -> Option<bool> {
        todo!()
    }

    pub fn get_coordinator_targets(&self) -> Vec<Id> {
        todo!()
    }

    pub fn is_active_payment_server(&self) -> bool {
        todo!()
    }

    pub fn get_status(&self) {
        todo!()
    }

    pub fn log(&self, message: &str) {
        todo!()
    }

    pub fn flush_log(&self) -> Vec<String> {
        todo!()
    }
}
