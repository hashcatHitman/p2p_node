// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! reading makes you smart.

#![expect(
    clippy::doc_markdown,
    clippy::doc_paragraphs_missing_punctuation,
    missing_docs,
    reason = "the docs are as they were meant to be, for now"
)]
#![allow(
    clippy::missing_docs_in_private_items,
    reason = "the docs are as they were meant to be, for now"
)]
#![expect(
    clippy::module_name_repetitions,
    unused,
    clippy::use_debug,
    clippy::iter_over_hash_type,
    clippy::arithmetic_side_effects,
    clippy::else_if_without_else,
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    reason = "to worry about later"
)]

use aws_config as _;
use jiff::tz::TimeZone;
use owo_colors::OwoColorize as _;
use tokio as _;

use crate::node::Id;

pub mod algorithms;
pub mod node;
pub mod protocol;

/// This isn't a real project.
///
/// ```rust
/// println!("{}", p2p_node::read_a_book())
/// ```
pub const fn read_a_book() -> &'static str {
    "you are so smart!!"
}

pub fn log(node_id: &Id, message: &str) {
    let timestamp = jiff::Timestamp::now()
        .to_zoned(TimeZone::UTC)
        .strftime("%FT%T%.8fZ");
    let timestamp = format!("[{timestamp}]");
    let node_id = format!("[{node_id}]");
    println!(
        "{} {} {}",
        timestamp.cyan(),
        node_id.purple(),
        message.white()
    );
}

pub fn warn(node_id: &Id, message: &str) {
    let timestamp = jiff::Timestamp::now()
        .to_zoned(TimeZone::UTC)
        .strftime("%FT%T%.8fZ");
    let timestamp = format!("[{timestamp}]");
    let node_id = format!("[{node_id}]");
    println!(
        "{} {} {}",
        timestamp.cyan(),
        node_id.purple(),
        message.yellow()
    );
}

pub fn error(node_id: &Id, message: &str) {
    let timestamp = jiff::Timestamp::now()
        .to_zoned(TimeZone::UTC)
        .strftime("%FT%T%.8fZ");
    let timestamp = format!("[{timestamp}]");
    let node_id = format!("[{node_id}]");
    println!(
        "{} {} {}",
        timestamp.cyan(),
        node_id.purple(),
        message.red()
    );
}
