// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The content module is for data types relating to view events and audits.

use crate::node::Id;

#[derive(Debug, Clone)]
pub(crate) struct ViewEventRecord {
    peer_id: Id,
    event_id: String,
    content_id: String,
    count: u64,
    ad_id: Option<String>,
}

impl ViewEventRecord {
    pub(crate) const fn new(
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

    const fn peer_id(&self) -> &Id {
        &self.peer_id
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn content_id(&self) -> &str {
        &self.content_id
    }

    pub(crate) const fn count(&self) -> u64 {
        self.count
    }

    fn ad_id(&self) -> Option<&str> {
        self.ad_id.as_deref()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PaymentRecord {
    from: Id,
    content_id: String,
    amount: f64,
    agreed_count: u64,
}

impl PaymentRecord {
    pub(crate) const fn new(
        from: Id,
        content_id: String,
        amount: f64,
        agreed_count: u64,
    ) -> Self {
        Self {
            from,
            content_id,
            amount,
            agreed_count,
        }
    }

    const fn from(&self) -> &Id {
        &self.from
    }

    fn content_id(&self) -> &str {
        &self.content_id
    }

    const fn amount(&self) -> f64 {
        self.amount
    }

    const fn agreed_count(&self) -> u64 {
        self.agreed_count
    }
}
