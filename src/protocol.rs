// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
