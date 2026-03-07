// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This isn't a real project.

#![expect(
    unused_crate_dependencies,
    clippy::result_large_err,
    reason = "to worry about later"
)]
#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    reason = "to worry about later"
)]

use aws_sdk_sqs as sqs;
use p2p_node::node::P2PNode;

/// This isn't a real project.
///
/// ```rust
/// println!("hello")
/// ```
#[::tokio::main]
async fn main() -> Result<(), sqs::Error> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_sqs::Client::new(&config);

    let mut node = P2PNode::new("sam".to_owned(), true, client).await;

    node.bootstrap(None).await;

    node.run().await;

    Ok(())
}
