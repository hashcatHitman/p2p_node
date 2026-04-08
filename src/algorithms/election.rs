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

    /// Check if the current leader is responsive.
    pub fn check_leader(&mut self) -> bool {
        if self.election_in_progress {
            return false;
        }

        match self.current_leader {
            None => true,
            Some(ref current_leader) => {
                let alive = (self.get_alive_peers)();

                if alive.contains(current_leader) {
                    self.last_leader_contact = time::Instant::now();
                } else {
                    let elapsed = self.last_leader_contact.elapsed();
                    if elapsed > self.leader_timeout {
                        let elapsed = elapsed.as_secs();
                        self.log(&format!("Leader {current_leader} is DEAD (no contact for {elapsed}s). Triggering election."));
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn start_election(&mut self) -> Vec<(Id, u64, f64)> {
        self.term += 1;
        self.state = ElectionStatus::Candidate;
        self.election_in_progress = true;
        self.election_start = time::Instant::now();
        self.got_ok = false;

        let my_rep = if self.is_bot {
            (self.get_reputation)(self.node_id.clone())
        } else {
            0.0
        };
        let alive = (self.get_alive_peers)();

        let mut outgoing = Vec::new();
        for bot_id in &self.bot_ids {
            if *bot_id == self.node_id {
                continue;
            }
            if !alive.contains(bot_id) {
                continue;
            }

            let bot_rep = (self.get_reputation)(bot_id.clone());
            let error_margin = 0.000_000_1;
            if (bot_rep > my_rep)
                || ((bot_rep - my_rep).abs() < error_margin
                    && *bot_id > self.node_id)
            {
                outgoing.push((bot_id.clone(), self.term, my_rep));
            }
        }

        if outgoing.is_empty() && self.is_bot {
            self.log(&format!(
                "No higher-rep bots alive. Declaring self as leader (term={})",
                self.term
            ));
            self.state = ElectionStatus::Leader;
            self.current_leader = Some(self.node_id.clone());
            self.election_in_progress = false;
            self.last_leader_contact = time::Instant::now();
        }

        self.log(&format!(
            "Election started (term={}, rep={my_rep:.3}). Challenging {} higher-rep bots.",
            self.term,
            outgoing.len()
        ));

        outgoing
    }

    pub fn receive_election(
        &mut self,
        sender: Id,
        term: u64,
        sender_reputation: f64,
    ) -> Option<(Id, u64, f64)> {
        if term > self.term {
            self.term = term;
        }

        let my_rep = if self.is_bot {
            (self.get_reputation)(self.node_id.clone())
        } else {
            0.0
        };

        let error_margin = 0.000_000_1;
        if self.is_bot
            && ((my_rep > sender_reputation)
                || ((sender_reputation - my_rep).abs() < error_margin
                    && self.node_id > sender))
        {
            self.log(&format!(
                "Received ELECTION from {sender} (rep={sender_reputation:.3}). I outrank (rep={my_rep:.3}). Sending OK."
            ));

            if !self.election_in_progress {
                self.election_in_progress = true;
                self.election_start = time::Instant::now();
                self.got_ok = false;
                self.state = ElectionStatus::Candidate;
            }

            return Some((sender, self.term, my_rep));
        }
        None
    }

    pub fn receive_election_ok(
        &mut self,
        sender: &Id,
        term: u64, // this is unused... but part of the API, for some reason
        sender_reputation: f64,
    ) {
        self.got_ok = true;
        self.log(&format!(
                "Received ELECTION_OK from {sender} (rep={sender_reputation:.3}). Backing off."
        ));
    }

    pub fn receive_coordinator(&mut self, sender: Id, term: u64) -> bool {
        if term < self.term {
            self.log(&format!(
                "Ignoring stale COORDINATOR from {sender} (term={term} < {})",
                self.term
            ));
            false
        } else {
            let log_message =
                format!("New leader: {sender} (term={term}). I am FOLLOWER.");
            self.term = term;
            self.current_leader = Some(sender);
            self.state = ElectionStatus::Follower;
            self.election_in_progress = false;
            self.last_leader_contact = time::Instant::now();
            self.log(&log_message);
            true
        }
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
