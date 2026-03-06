// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::fmt;
use core::fmt::Display;
use core::str::FromStr;

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
