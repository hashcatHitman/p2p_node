// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! HashCash: Proof-of-Work Message Stamps
//! ========================================
//!
//! Sender-side defense: every message must carry a PoW stamp proving
//! that the sender burned real CPU cycles to produce it.
//!
//! mine_stamp(msg_body, difficulty) -> {"nonce": int, "difficulty": int, "hash": str}
//! verify_stamp(msg_body, pow_data) -> bool
//!
//! Integration:
//!   - Sender calls mine_stamp() before sending via SQS.
//!   - Receiver calls verify_stamp() immediately on receipt; drops invalid messages.
//!   - Difficulty is set per message type (see DIFFICULTY_MAP).
//!
//! Difficulty guide:
//!   - 2: trivial (~16 attempts, instant) — PING/PONG/PEER_LIST
//!   - 3: light  (~4k attempts, <0.1s)    — VIEW_EVENT/HELLO/GOSSIP
//!   - 4: moderate (~65k attempts, ~0.5s)  — ELECTION/COORDINATOR
//!   - 5: serious (~1M attempts, ~2-4s)    — AUDIT_RESULT (triggers payments)

use base16ct::lower;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct ProofOfWork {
    nonce: u64,
    difficulty: u8,
    // The actual type sha2 returns is not print/serde friendly, so...
    hash: String,
}
