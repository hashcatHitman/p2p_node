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

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::node::Id;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
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
            "HELLO" => Self::Hello,
            "PEER_LIST" => Self::PeerList,
            "PING" => Self::Ping,
            "PONG" => Self::Pong,
            "VIEW_EVENT" => Self::ViewEvent,
            "AUDIT_RESULT" => Self::AuditResult,
            "CHOKE" => Self::Choke,
            "UNCHOKE" => Self::Unchoke,
            _ => return Err(()),
        };
        Ok(ok)
    }
}

pub fn base(kind: MessageKind, sender: &Id) -> Map<String, Value> {
    let mut message = Map::new();
    let timestamp = jiff::Timestamp::now()
        .in_tz("UTC")
        .unwrap()
        .timestamp()
        .to_string();
    let message_id = uuid::Uuid::new_v4().to_string();
    drop(message.insert("type".to_owned(), Value::String(kind.to_string())));
    drop(
        message.insert("sender".to_owned(), Value::String(sender.to_string())),
    );
    drop(message.insert("timestamp".to_owned(), Value::String(timestamp)));
    drop(message.insert("msg_id".to_owned(), Value::String(message_id)));

    message
}

pub fn hello(sender: &Id, queue_url: String) -> Map<String, Value> {
    let mut message = base(MessageKind::Hello, sender);
    drop(message.insert("queue_url".to_owned(), Value::String(queue_url)));
    message
}

pub fn peer_list(sender: &Id, peers: &[Value]) -> Map<String, Value> {
    let mut message = base(MessageKind::PeerList, sender);

    drop(message.insert("peers".to_owned(), json!(peers)));
    message
}

pub fn ping(sender: &Id, sequence: u16) -> Map<String, Value> {
    let mut message = base(MessageKind::Ping, sender);

    drop(message.insert("seq".to_owned(), json!(sequence)));
    message
}

pub fn pong(sender: &Id, sequence: u16) -> Map<String, Value> {
    let mut message = base(MessageKind::Pong, sender);

    drop(message.insert("seq".to_owned(), json!(sequence)));
    message
}

pub fn view_event(
    sender: &Id,
    event_id: String,
    content_id: String,
    count: u64,
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
    sender: &Id,
    content_id: String,
    agreed_count: u64,
    confidence: f64,
    voters: Option<Vec<Id>>,
) -> Map<String, Value> {
    let mut message = base(MessageKind::AuditResult, sender);
    drop(message.insert("content_id".to_owned(), Value::String(content_id)));
    drop(message.insert("agreed_count".to_owned(), json!(agreed_count)));
    drop(message.insert("confidence".to_owned(), json!(confidence)));
    let voters: Vec<Id> = voters.unwrap_or_default();
    let voters: Vec<String> = voters.iter().map(ToString::to_string).collect();
    drop(message.insert("voters".to_owned(), json!(voters)));
    message
}

pub fn choke(sender: &Id) -> Map<String, Value> {
    base(MessageKind::Choke, sender)
}

pub fn unchoke(sender: &Id) -> Map<String, Value> {
    base(MessageKind::Unchoke, sender)
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Peer {
    pub node_id: Id,
    pub queue_url: String,
}

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
pub struct Hello {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
    queue_url: String,
}

impl Hello {
    pub fn new(sender: Id, queue_url: String) -> Self {
        Self {
            sender,
            timestamp: jiff::Timestamp::now().in_tz("UTC").unwrap().timestamp(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            queue_url,
        }
    }

    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }

    pub fn queue_url(&self) -> &str {
        &self.queue_url
    }
}

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
pub struct PeerList {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
    peers: Vec<Peer>,
}

impl PeerList {
    pub fn new(sender: Id, peers: Vec<Peer>) -> Self {
        Self {
            sender,
            timestamp: jiff::Timestamp::now().in_tz("UTC").unwrap().timestamp(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            peers,
        }
    }

    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }

    pub fn peers(&self) -> &[Peer] {
        &self.peers
    }
}

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
pub struct Ping {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
    seq: u16,
}

impl Ping {
    pub fn new(sender: Id, seq: u16) -> Self {
        Self {
            sender,
            timestamp: jiff::Timestamp::now().in_tz("UTC").unwrap().timestamp(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            seq,
        }
    }

    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }

    pub const fn seq(&self) -> u16 {
        self.seq
    }
}

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
pub struct Pong {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
    seq: u16,
}

impl Pong {
    pub fn new(sender: Id, seq: u16) -> Self {
        Self {
            sender,
            timestamp: jiff::Timestamp::now().in_tz("UTC").unwrap().timestamp(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            seq,
        }
    }

    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }

    pub const fn seq(&self) -> u16 {
        self.seq
    }
}

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
pub struct ViewEvent {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
    event_id: String,
    content_id: String,
    count: u64,
    ad_id: Option<String>,
}

impl ViewEvent {
    pub fn new(
        sender: Id,
        event_id: String,
        content_id: String,
        count: u64,
        ad_id: Option<String>,
    ) -> Self {
        Self {
            sender,
            timestamp: jiff::Timestamp::now().in_tz("UTC").unwrap().timestamp(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            event_id,
            content_id,
            count,
            ad_id,
        }
    }

    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
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

#[derive(
    Debug, Clone, PartialEq, PartialOrd, Default, Serialize, Deserialize,
)]
pub struct AuditResult {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
    content_id: String,
    agreed_count: u64,
    confidence: f64,
    voters: Option<Vec<Id>>,
}

impl AuditResult {
    pub fn new(
        sender: Id,
        content_id: String,
        agreed_count: u64,
        confidence: f64,
        voters: Option<Vec<Id>>,
    ) -> Self {
        Self {
            sender,
            timestamp: jiff::Timestamp::now().in_tz("UTC").unwrap().timestamp(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            content_id,
            agreed_count,
            confidence,
            voters,
        }
    }

    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }

    pub fn content_id(&self) -> &str {
        &self.content_id
    }

    pub const fn agreed_count(&self) -> u64 {
        self.agreed_count
    }

    pub const fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn voters(&self) -> Option<&[Id]> {
        self.voters.as_deref()
    }
}

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
pub struct Choke {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
}

impl Choke {
    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }
}

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
pub struct Unchoke {
    sender: Id,
    timestamp: jiff::Timestamp,
    msg_id: String,
}

impl Unchoke {
    pub const fn sender(&self) -> &Id {
        &self.sender
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        self.timestamp
    }

    pub fn msg_id(&self) -> &str {
        &self.msg_id
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Message {
    Hello {
        #[serde(flatten)]
        message: Hello,
    },
    PeerList {
        #[serde(flatten)]
        message: PeerList,
    },
    Ping {
        #[serde(flatten)]
        message: Ping,
    },
    Pong {
        #[serde(flatten)]
        message: Pong,
    },
    ViewEvent {
        #[serde(flatten)]
        message: ViewEvent,
    },
    AuditResult {
        #[serde(flatten)]
        message: AuditResult,
    },
    Choke {
        #[serde(flatten)]
        message: Choke,
    },
    Unchoke {
        #[serde(flatten)]
        message: Unchoke,
    },
}

impl Message {
    pub const fn sender(&self) -> &Id {
        match *self {
            Self::Hello { ref message } => message.sender(),
            Self::PeerList { ref message } => message.sender(),
            Self::Ping { ref message } => message.sender(),
            Self::Pong { ref message } => message.sender(),
            Self::ViewEvent { ref message } => message.sender(),
            Self::AuditResult { ref message } => message.sender(),
            Self::Choke { ref message } => message.sender(),
            Self::Unchoke { ref message } => message.sender(),
        }
    }

    pub const fn timestamp(&self) -> jiff::Timestamp {
        match *self {
            Self::Hello { ref message } => message.timestamp(),
            Self::PeerList { ref message } => message.timestamp(),
            Self::Ping { ref message } => message.timestamp(),
            Self::Pong { ref message } => message.timestamp(),
            Self::ViewEvent { ref message } => message.timestamp(),
            Self::AuditResult { ref message } => message.timestamp(),
            Self::Choke { ref message } => message.timestamp(),
            Self::Unchoke { ref message } => message.timestamp(),
        }
    }

    pub fn msg_id(&self) -> &str {
        match *self {
            Self::Hello { ref message } => message.msg_id(),
            Self::PeerList { ref message } => message.msg_id(),
            Self::Ping { ref message } => message.msg_id(),
            Self::Pong { ref message } => message.msg_id(),
            Self::ViewEvent { ref message } => message.msg_id(),
            Self::AuditResult { ref message } => message.msg_id(),
            Self::Choke { ref message } => message.msg_id(),
            Self::Unchoke { ref message } => message.msg_id(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::node::Id;
    use crate::protocol::{
        AuditResult, Choke, Hello, Message, Peer, PeerList, Ping, Pong,
        Unchoke, ViewEvent,
    };

    const NINETEEN_EIGHTY_FOUR: jiff::Timestamp =
        jiff::Timestamp::constant(441_763_200, 0);

    #[test]
    fn deserialize_hello() {
        let json: &str = r#"
        {
            "type": "HELLO",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "queue_url": "https://sqs.us-east-1.amazonaws.com/194722398367/ds2032-node-alice-p2p"
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::Hello {
            message:Hello{
            sender: Id::new("alice".to_owned()),
            timestamp: NINETEEN_EIGHTY_FOUR,
            msg_id: "a1b2c3d4".to_owned(),
            queue_url:"https://sqs.us-east-1.amazonaws.com/194722398367/ds2032-node-alice-p2p".to_owned()}
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_peer_list() {
        let json = r#"
        {
            "type": "PEER_LIST",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "peers": [
                {"node_id": "bob", "queue_url": "https://sqs.../ds2032-node-bob-p2p"},
                {"node_id": "charlie", "queue_url": "https://sqs.../ds2032-node-charlie-p2p"}
            ]
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::PeerList {
            message: PeerList {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                peers: vec![
                    Peer {
                        node_id: Id::new("bob".to_owned()),
                        queue_url: "https://sqs.../ds2032-node-bob-p2p"
                            .to_owned(),
                    },
                    Peer {
                        node_id: Id::new("charlie".to_owned()),
                        queue_url: "https://sqs.../ds2032-node-charlie-p2p"
                            .to_owned(),
                    },
                ],
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_ping() {
        let json = r#"
        {
            "type": "PING",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "seq": 31337
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::Ping {
            message: Ping {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                seq: 31337,
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_pong() {
        let json = r#"
        {
            "type": "PONG",
            "sender": "bob",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "seq": 42
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::Pong {
            message: Pong {
                sender: Id::new("bob".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                seq: 42,
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_view_event() {
        let json = r#"
        {
            "type": "VIEW_EVENT",
            "sender": "bob",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "event_id": "evt-a1b2c3d4",
            "content_id": "show:midnight-run",
            "count": 150,
            "ad_id": "ad-7"
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::ViewEvent {
            message: ViewEvent {
                sender: Id::new("bob".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                event_id: "evt-a1b2c3d4".to_owned(),
                content_id: "show:midnight-run".to_owned(),
                count: 150,
                ad_id: Some("ad-7".to_owned()),
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_view_event_null() {
        let json = r#"
        {
            "type": "VIEW_EVENT",
            "sender": "bob",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "event_id": "evt-a1b2c3d4",
            "content_id": "show:midnight-run",
            "count": 150,
            "ad_id": null
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::ViewEvent {
            message: ViewEvent {
                sender: Id::new("bob".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                event_id: "evt-a1b2c3d4".to_owned(),
                content_id: "show:midnight-run".to_owned(),
                count: 150,
                ad_id: None,
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_view_event_field_not_present() {
        let json = r#"
        {
            "type": "VIEW_EVENT",
            "sender": "bob",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "event_id": "evt-a1b2c3d4",
            "content_id": "show:midnight-run",
            "count": 150
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::ViewEvent {
            message: ViewEvent {
                sender: Id::new("bob".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                event_id: "evt-a1b2c3d4".to_owned(),
                content_id: "show:midnight-run".to_owned(),
                count: 150,
                ad_id: None,
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_audit_result() {
        let json = r#"
        {
            "type": "AUDIT_RESULT",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "content_id": "show:midnight-run",
            "agreed_count": 150,
            "confidence": 0.92,
            "voters": ["bob", "charlie", "don", "edward"]
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::AuditResult {
            message: AuditResult {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                content_id: "show:midnight-run".to_owned(),
                agreed_count: 150,
                confidence: 0.92,
                voters: Some(vec![
                    Id::new("bob".to_owned()),
                    Id::new("charlie".to_owned()),
                    Id::new("don".to_owned()),
                    Id::new("edward".to_owned()),
                ]),
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_audit_result_null() {
        let json = r#"
        {
            "type": "AUDIT_RESULT",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "content_id": "show:midnight-run",
            "agreed_count": 150,
            "confidence": 0.92,
            "voters": null
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::AuditResult {
            message: AuditResult {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                content_id: "show:midnight-run".to_owned(),
                agreed_count: 150,
                confidence: 0.92,
                voters: None,
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_audit_result_field_not_present() {
        let json = r#"
        {
            "type": "AUDIT_RESULT",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4",
            "content_id": "show:midnight-run",
            "agreed_count": 150,
            "confidence": 0.92
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::AuditResult {
            message: AuditResult {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
                content_id: "show:midnight-run".to_owned(),
                agreed_count: 150,
                confidence: 0.92,
                voters: None,
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_choke() {
        let json = r#"
        {
            "type": "CHOKE",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4"
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::Choke {
            message: Choke {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
            },
        };

        assert_eq!(constructed, deserialized);
    }

    #[test]
    fn deserialize_unchoke() {
        let json = r#"
        {
            "type": "UNCHOKE",
            "sender": "alice",
            "timestamp": "1984-01-01T00:00:00Z",
            "msg_id": "a1b2c3d4"
        }"#;
        let deserialized: Message = serde_json::from_str(json).unwrap();

        let constructed: Message = Message::Unchoke {
            message: Unchoke {
                sender: Id::new("alice".to_owned()),
                timestamp: NINETEEN_EIGHTY_FOUR,
                msg_id: "a1b2c3d4".to_owned(),
            },
        };

        assert_eq!(constructed, deserialized);
    }
}
