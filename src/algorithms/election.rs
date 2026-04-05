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
