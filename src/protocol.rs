// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! P2P Message Protocol
//! ====================
//!
//! Builders and parsers for the 8 P2P message types.
//! All messages are JSON dicts sent as SQS message bodies.
//!
//! Every message has:
//!   - type:      one of MESSAGE_TYPES
//!   - sender:    node_id of the sender
//!   - timestamp: ISO 8601 UTC timestamp
//!
//! Type-specific fields are documented on each builder function.

use core::fmt;
use core::fmt::Display;
use core::str::FromStr;

use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageKind {
    Hello,
    PeerList,
    Ping,
    Pong,
    ViewEvent,
    AuditResult,
    Choke,
    Unchoke,
}

impl Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match *self {
            Self::Hello => "HELLO",
            Self::PeerList => "PEER_LIST",
            Self::Ping => "PING",
            Self::Pong => "PONG",
            Self::ViewEvent => "VIEW_EVENT",
            Self::AuditResult => "AUDIT_RESULT",
            Self::Choke => "CHOKE",
            Self::Unchoke => "UNCHOKE",
        };
        write!(f, "{string}")
    }
}

impl FromStr for MessageKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ok = match s.trim() {
            "\"HELLO\"" => Self::Hello,
            "\"PEER_LIST\"" => Self::PeerList,
            "\"PING\"" => Self::Ping,
            "\"PONG\"" => Self::Pong,
            "\"VIEW_EVENT\"" => Self::ViewEvent,
            "\"AUDIT_RESULT\"" => Self::AuditResult,
            "\"CHOKE\"" => Self::Choke,
            "\"UNCHOKE\"" => Self::Unchoke,
            _ => return Err(()),
        };
        Ok(ok)
    }
}

pub fn base(kind: MessageKind, sender: String) -> Map<String, Value> {
    let mut message = Map::new();
    let timestamp = jiff::Timestamp::now()
        .in_tz("UTC")
        .unwrap()
        .timestamp()
        .to_string();
    let message_id = uuid::Uuid::new_v4().to_string();
    drop(message.insert("type".to_owned(), Value::String(kind.to_string())));
    drop(message.insert("sender".to_owned(), Value::String(sender)));
    drop(message.insert("timestamp".to_owned(), Value::String(timestamp)));
    drop(message.insert("msg_id".to_owned(), Value::String(message_id)));

    message
}

pub fn hello(sender: String, queue_url: String) -> Map<String, Value> {
    let mut message = base(MessageKind::Hello, sender);
    drop(message.insert("queue_url".to_owned(), Value::String(queue_url)));
    message
}

pub fn peer_list(sender: String, peers: Vec<Value>) -> Map<String, Value> {
    let mut message = base(MessageKind::PeerList, sender);

    drop(message.insert("peers".to_owned(), json!(peers)));
    message
}

pub fn ping(sender: String, sequence: u16) -> Map<String, Value> {
    let mut message = base(MessageKind::Ping, sender);

    drop(message.insert("seq".to_owned(), json!(sequence)));
    message
}

pub fn pong(sender: String, sequence: u16) -> Map<String, Value> {
    let mut message = base(MessageKind::Pong, sender);

    drop(message.insert("seq".to_owned(), json!(sequence)));
    message
}

pub fn view_event(
    sender: String,
    event_id: String,
    content_id: String,
    count: u32,
    ad_id: String,
) -> Map<String, Value> {
    let mut message = base(MessageKind::ViewEvent, sender);
    drop(message.insert("event_id".to_owned(), Value::String(event_id)));
    drop(message.insert("content_id".to_owned(), Value::String(content_id)));
    drop(message.insert("count".to_owned(), json!(count)));
    drop(message.insert("ad_id".to_owned(), Value::String(ad_id)));
    message
}

pub fn audit_result(
    sender: String,
    content_id: String,
    agreed_count: u32,
    confidence: f64,
    voters: Option<Vec<String>>,
) -> Map<String, Value> {
    let mut message = base(MessageKind::AuditResult, sender);
    drop(message.insert("content_id".to_owned(), Value::String(content_id)));
    drop(message.insert("agreed_count".to_owned(), json!(agreed_count)));
    drop(message.insert("confidence".to_owned(), json!(confidence)));
    let voters = voters.unwrap_or_default();
    drop(message.insert("voters".to_owned(), json!(voters)));
    message
}

pub fn choke(sender: String) -> Map<String, Value> {
    base(MessageKind::Choke, sender)
}

pub fn unchoke(sender: String) -> Map<String, Value> {
    base(MessageKind::Unchoke, sender)
}
