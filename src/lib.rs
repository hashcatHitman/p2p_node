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
    clippy::needless_pass_by_value,
    clippy::use_debug,
    clippy::iter_over_hash_type,
    clippy::min_ident_chars,
    clippy::arithmetic_side_effects,
    clippy::else_if_without_else,
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    reason = "to worry about later"
)]

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
