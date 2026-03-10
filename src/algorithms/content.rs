// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The content module is for data types relating to view events and audits.

use crate::node::Id;

#[derive(Debug, Clone)]
pub struct ViewEvent {
    peer_id: Id,
    event_id: String,
    content_id: String,
    count: u64,
    ad_id: Option<String>,
}

impl ViewEvent {
    pub const fn new(
        peer_id: Id,
        event_id: String,
        content_id: String,
        count: u64,
        ad_id: Option<String>,
    ) -> Self {
        Self {
            peer_id,
            event_id,
            content_id,
            count,
            ad_id,
        }
    }

    pub const fn peer_id(&self) -> &Id {
        &self.peer_id
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn content_id(&self) -> &str {
        &self.content_id
    }

    pub const fn count(&self) -> u64 {
        self.count
    }

    pub fn ad_id(&self) -> Option<&str> {
        self.ad_id.as_deref()
    }
}
