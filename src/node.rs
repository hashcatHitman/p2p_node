// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! P2P Node
//! ========
//!
//! Architecture:
//!   run() polls SQS every few seconds, calls handle_message() for each
//!   received message, then calls run_periodic_tasks() for timers.

use core::fmt::{self, Display};
use std::collections::HashMap;
use std::fs::File;
use std::{io, time};

use aws_sdk_sqs::Client;
use rand::seq::IteratorRandom as _;
use serde::{Deserialize, Serialize};

use crate::algorithms::choking::ChokingNode;
use crate::algorithms::content::ViewEventRecord;
use crate::algorithms::gossip::GossipNode;
use crate::algorithms::hashcash::stamp_message;
use crate::algorithms::heartbeat::HeartbeatNode;
use crate::algorithms::reputation::ReputationNode;
use crate::protocol::{
    AuditResult, Choke, Hello, Message, PeerList, Ping, Pong, Unchoke,
    ViewEvent,
};

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
#[serde(transparent)]
pub struct Id {
    node_id: String,
}

impl Id {
    pub const fn new(node_id: String) -> Self {
        Self { node_id }
    }

    pub fn as_str(&self) -> &str {
        &self.node_id
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node_id)
    }
}

#[derive(Debug, Clone)]
pub struct SqsTransport {
    sqs: Client,
    queue_url_cache: HashMap<Id, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Resources {
    queues: HashMap<Id, String>,
}

impl SqsTransport {
    pub fn new(sqs: Client) -> Self {
        Self {
            sqs,
            queue_url_cache: HashMap::new(),
        }
    }

    pub fn load_resources(&mut self, disk_cache: File) -> bool {
        let disk_cache = io::BufReader::new(disk_cache);
        let cache: Result<Resources, serde_json::Error> =
            serde_json::from_reader(disk_cache);

        match cache {
            Ok(cache) => {
                self.queue_url_cache = cache.queues;
                true
            }
            Err(_error) => false,
        }
    }

    pub async fn get_queue_url(&mut self, node_id: Id) -> Option<String> {
        if let Some(url) = self.queue_url_cache.get(&node_id) {
            return Some(url.clone());
        }

        let request = self.sqs.get_queue_url().queue_name(node_id.as_str());

        request
            .send()
            .await
            .map(|result| result.queue_url)
            .ok()
            .flatten()
            .inspect(|url| {
                drop(self.queue_url_cache.insert(node_id, url.clone()));
            })
    }

    pub async fn send(&mut self, target_node_id: Id, message: Message) -> bool {
        if let Some(url) = self.get_queue_url(target_node_id.clone()).await {
            match serde_json::to_string(&message) {
                Ok(serialized) => {
                    match self
                        .sqs
                        .send_message()
                        .queue_url(url)
                        .message_body(serialized)
                        .send()
                        .await
                    {
                        Ok(_) => return true,
                        Err(err) => {
                            eprintln!("[ERR] Send to {target_node_id}: {err}");
                            return false;
                        }
                    }
                }
                Err(err) => {
                    eprintln!(
                        "[ERR] Could not serialize message for {target_node_id}: {err}"
                    );
                    return false;
                }
            }
        }
        false
    }

    pub async fn receive(
        &mut self,
        node_id: Id,
        max_messages: i32,
        wait_seconds: i32,
    ) -> Vec<(Message, String)> {
        let mut messages = Vec::new();
        if let Some(url) = self.get_queue_url(node_id.clone()).await {
            match self
                .sqs
                .receive_message()
                .queue_url(url)
                .max_number_of_messages(max_messages.min(10))
                .wait_time_seconds(wait_seconds)
                .send()
                .await
            {
                Ok(response) => {
                    if let Some(msg_vec) = response.messages {
                        for sqs_msg in msg_vec {
                            if let Some(body) = sqs_msg.body()
                                && let Ok(deserialized) =
                                    serde_json::from_str::<Message>(body)
                                && let Some(receipt_handle) =
                                    sqs_msg.receipt_handle
                            {
                                messages.push((deserialized, receipt_handle));
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("[ERR] Receive for {node_id}: {err}");
                }
            }
        }
        messages
    }

    pub async fn delete(&mut self, node_id: Id, receipt_handle: String) {
        if let Some(url) = self.get_queue_url(node_id.clone()).await {
            match self
                .sqs
                .delete_message()
                .queue_url(url)
                .receipt_handle(&receipt_handle)
                .send()
                .await
            {
                Ok(_deleted) => (),
                Err(err) => {
                    crate::warn(
                        &node_id,
                        &format!("Failed to delete {receipt_handle}: {err}"),
                    );
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct P2PNode {
    node_id: Id,
    verbose: bool,
    running: bool,
    transport: SqsTransport,
    queue_url: Option<String>,
    gossip: GossipNode,
    heartbeat: HeartbeatNode,
    choking: ChokingNode,
    reputation: ReputationNode,
    view_events: HashMap<String, HashMap<Id, ViewEventRecord>>,
    view_counts: HashMap<&'static str, u64>,
    gossip_interval: u16,
    heartbeat_interval: u16,
    choking_interval: u16,
    reputation_interval: u16,
    publish_interval: u16,
    audit_interval: u16,
    last_gossip: time::Instant,
    last_heartbeat: time::Instant,
    last_choking: time::Instant,
    last_reputation: time::Instant,
    last_publish: time::Instant,
    last_audit: time::Instant,
    ping_seq: u16,
    messages_received: u32,
    messages_sent: u32,
    rounds: u32,
}

impl P2PNode {
    pub async fn new(node_id: Id, verbose: bool, sqs: Client) -> Self {
        let mut transport = SqsTransport::new(sqs);

        if let Ok(disk_cache) = File::open("resources.json") {
            if !transport.load_resources(disk_cache) {
                crate::warn(&node_id, "Failed to load resources from disk");
            }
        } else {
            crate::warn(&node_id, "Failed to open resources file on disk");
        }

        let queue_url = transport.get_queue_url(node_id.clone()).await;

        let this = Self {
            node_id: node_id.clone(),
            verbose,
            running: false,
            transport,
            queue_url: queue_url.clone(),
            gossip: GossipNode::new(
                node_id.clone(),
                queue_url.unwrap_or(String::new()),
            ),
            heartbeat: HeartbeatNode::new(node_id.clone(), 3, 2),
            choking: ChokingNode::new(node_id.clone(), 4, 3),
            reputation: ReputationNode::new(node_id),
            view_events: HashMap::new(),
            view_counts: HashMap::from_iter([
                ("show:midnight-run", 0),
                ("show:neon-drift", 0),
                ("show:binary-sunset", 0),
            ]),
            gossip_interval: 15,
            heartbeat_interval: 10,
            choking_interval: 30,
            reputation_interval: 30,
            publish_interval: 15,
            audit_interval: 45,
            last_gossip: time::Instant::now(),
            last_heartbeat: time::Instant::now(),
            last_choking: time::Instant::now(),
            last_reputation: time::Instant::now(),
            last_publish: time::Instant::now(),
            last_audit: time::Instant::now(),
            ping_seq: 0,
            messages_received: 0,
            messages_sent: 0,
            rounds: 0,
        };
        crate::log(
            &this.node_id,
            &format!(
                "Initialized. Queue: {}",
                this.queue_url.clone().unwrap_or_default()
            ),
        );
        this
    }

    pub async fn bootstrap(&mut self, bootstrap_nodes: Option<Vec<Id>>) {
        let bootstrap_nodes = bootstrap_nodes.unwrap_or_else(|| {
            vec![
                Id::new("bot-alpha".to_owned()),
                Id::new("bot-bravo".to_owned()),
                Id::new("bot-charlie".to_owned()),
            ]
        });
        crate::log(
            &self.node_id,
            &format!("Bootstrapping via {bootstrap_nodes:?}..."),
        );

        let message = Hello::new(
            self.node_id.clone(),
            self.queue_url.clone().unwrap_or_default(),
        );

        for node in bootstrap_nodes {
            let _: bool = self
                .transport
                .send(
                    node,
                    Message::Hello {
                        message: message.clone(),
                    },
                )
                .await;
        }
    }

    pub async fn stamp_and_send(
        &mut self,
        target_node_id: Id,
        message: Message,
    ) -> bool {
        let mut message = message;
        stamp_message(&mut message, None);
        if self.transport.send(target_node_id, message).await {
            self.messages_sent += 1;
            true
        } else {
            false
        }
    }

    pub async fn handle_message(&mut self, message: &Message) {
        let node_id = message.sender();

        if *node_id == self.node_id {
            return;
        }
        self.messages_received += 1;

        if let Some(sender) = self.gossip.peers_mut().get_mut(node_id) {
            *sender.time_to_live_mut() = 5;
        }

        match *message {
            Message::Hello { ref message } => self.handle_hello(message).await,
            Message::PeerList { ref message } => self.handle_peer_list(message),
            Message::Ping { ref message } => self.handle_ping(message).await,
            Message::Pong { ref message } => self.handle_pong(message),
            Message::ViewEvent { ref message } => {
                self.handle_view_event(message);
            }
            Message::AuditResult { ref message } => {
                self.handle_audit_result(message);
            }
            Message::Choke { ref message } => self.handle_choke(message),
            Message::Unchoke { ref message } => self.handle_unchoke(message),
            Message::Election { ref message } => todo!("{:?}", message),
            Message::ElectionOk { ref message } => todo!("{:?}", message),
            Message::Coordinator { ref message } => todo!("{:?}", message),
            Message::Payment { ref message } => todo!("{:?}", message),
        }
    }

    pub async fn handle_hello(&mut self, message: &Hello) {
        let node_id = message.sender();

        let queue_url = message.queue_url();

        crate::log(&self.node_id, &format!("Got a hello from: {node_id}"));
        self.gossip.add_peer(node_id.clone(), queue_url.to_owned());
        self.heartbeat.add_peer(node_id.clone());
        self.choking.add_peer(node_id.clone());
        self.reputation.add_peer(node_id.clone());

        drop(
            self.transport
                .queue_url_cache
                .insert(node_id.clone(), queue_url.to_owned()),
        );

        let plist = PeerList::new(
            self.node_id.clone(),
            self.gossip.get_peer_list_message(),
        );

        if !self
            .transport
            .send(node_id.clone(), Message::PeerList { message: plist })
            .await
        {
            crate::warn(
                &self.node_id,
                "Failed to send peer list when handling hello from peer",
            );
        }
    }

    pub fn handle_peer_list(&mut self, message: &PeerList) {
        let sender_id = message.sender();

        let incoming = message.peers();
        crate::log(
            &self.node_id,
            &format!("Got a peer list from: {sender_id}"),
        );
        let _: u8 = self.gossip.receive_peer_list(incoming, sender_id);

        for (peer, record) in self.gossip.peers() {
            self.heartbeat.add_peer(peer.clone());
            self.choking.add_peer(peer.clone());
            self.reputation.add_peer(peer.clone());
            drop(
                self.transport
                    .queue_url_cache
                    .insert(peer.clone(), record.queue_url().to_owned()),
            );
        }
    }

    pub async fn handle_ping(&mut self, message: &Ping) {
        let node_id = message.sender();
        let seq = message.seq();
        crate::log(
            &self.node_id,
            &format!("Got a ping from: {node_id} (#{seq})"),
        );

        let response = Pong::new(self.node_id.clone(), seq);
        if !self
            .transport
            .send(node_id.clone(), Message::Pong { message: response })
            .await
        {
            crate::warn(
                &self.node_id,
                &format!("Failed to send a pong to: {node_id} (#{seq})"),
            );
        }
        self.choking.record_contribution(node_id, 1);
        self.reputation.record_contribution(node_id, 1);
    }

    pub fn handle_pong(&mut self, message: &Pong) {
        let node_id = message.sender();
        let seq: u16 = message.seq();
        crate::log(
            &self.node_id,
            &format!("Got a pong from: {node_id} (#{seq})"),
        );

        self.heartbeat.receive_pong(node_id, seq.into());
        self.reputation.record_heartbeat(node_id, true);
    }

    pub fn handle_view_event(&mut self, message: &ViewEvent) {
        let node_id = message.sender();

        let event_id = message.event_id();
        let content_id = message.content_id();
        let count = message.count();
        let ad_id = message.ad_id().map(ToOwned::to_owned);
        let view_event = ViewEventRecord::new(
            node_id.clone(),
            event_id.to_owned(),
            content_id.to_owned(),
            count,
            ad_id,
        );
        crate::log(
            &self.node_id,
            &format!("View event from {node_id}: {view_event:?}"),
        );
        match self.view_events.get_mut(content_id) {
            Some(map) => {
                drop(map.insert(node_id.clone(), view_event));
            }
            None => {
                drop(self.view_events.insert(
                    content_id.to_owned(),
                    HashMap::from_iter([(node_id.clone(), view_event)]),
                ));
            }
        }
        self.choking.record_contribution(node_id, 1);
        self.reputation.record_contribution(node_id, 1);
    }

    pub fn handle_audit_result(&mut self, message: &AuditResult) {
        let node_id = message.sender();

        let content_id = message.content_id();

        let agreed_count = message.agreed_count();
        let confidence = message.confidence() * 100.0;
        let voters = message.voters().unwrap_or_default();

        let log = format!(
            "Audit result from {node_id}: \"{content_id}\", {agreed_count} \
            views, {confidence:.2}% confidence, as voted by {voters:?}"
        );
        crate::log(&self.node_id, &log);
        self.reputation.record_contribution(node_id, 1);
    }

    pub fn handle_choke(&self, message: &Choke) {
        let node_id = message.sender();
        crate::log(&self.node_id, &format!("Choked by {node_id}"));
    }

    pub fn handle_unchoke(&self, message: &Unchoke) {
        let node_id = message.sender();
        crate::log(&self.node_id, &format!("Unchoked by {node_id}"));
    }

    #[expect(
        clippy::future_not_send,
        reason = "I don't plan on sending these futures between threads and
        would rather not use a panicking RNG."
    )]
    pub async fn run_periodic_tasks(&mut self) {
        let now = time::Instant::now();

        if now - self.last_gossip
            >= time::Duration::from_secs(self.gossip_interval.into())
        {
            self.do_gossip().await;
            self.last_gossip = now;
        }

        if now - self.last_heartbeat
            >= time::Duration::from_secs(self.heartbeat_interval.into())
        {
            self.do_heartbeat().await;
            self.last_heartbeat = now;
        }

        if now - self.last_choking
            >= time::Duration::from_secs(self.choking_interval.into())
        {
            self.do_choking();
            self.last_choking = now;
        }

        if now - self.last_reputation
            >= time::Duration::from_secs(self.reputation_interval.into())
        {
            self.do_reputation();
            self.last_reputation = now;
        }

        if now - self.last_publish
            >= time::Duration::from_secs(self.publish_interval.into())
        {
            self.do_publish().await;
            self.last_publish = now;
        }

        if now - self.last_audit
            >= time::Duration::from_secs(self.audit_interval.into())
        {
            self.do_audit().await;
            self.last_audit = now;
        }
    }

    pub async fn do_gossip(&mut self) {
        if let Some(target) = self.gossip.pick_gossip_target() {
            let message = PeerList::new(
                self.node_id.clone(),
                self.gossip.get_peer_list_message(),
            );

            if self
                .transport
                .send(target.clone(), Message::PeerList { message })
                .await
            {
                crate::log(&self.node_id, &format!("Sent gossip to: {target}"));
            } else {
                crate::warn(
                    &self.node_id,
                    &format!("Failed to send gossip to: {target}"),
                );
            }
        }
    }

    pub async fn do_heartbeat(&mut self) {
        self.ping_seq += 1;
        let ping = Ping::new(self.node_id.clone(), self.ping_seq);

        for peer in self.heartbeat.send_pings(self.rounds) {
            if self
                .transport
                .send(
                    peer.clone(),
                    Message::Ping {
                        message: ping.clone(),
                    },
                )
                .await
            {
                crate::log(
                    &self.node_id,
                    &format!("Sent heartbeat to: {peer} (#{})", self.ping_seq),
                );
            } else {
                crate::warn(
                    &self.node_id,
                    &format!(
                        "Failed to send heartbeat to: {peer} (#{})",
                        self.ping_seq
                    ),
                );
            }
        }
    }

    pub fn do_choking(&mut self) {
        self.choking.run_choking_round();
        for log in self.choking.flush_log() {
            crate::log(&self.node_id, &log);
        }
    }

    pub fn do_reputation(&mut self) {
        self.reputation.update_all_scores();
        crate::log(&self.node_id, "Updated scores");
    }

    #[expect(
        clippy::future_not_send,
        reason = "I don't plan on sending these futures between threads and
        would rather not use a panicking RNG."
    )]
    pub async fn do_publish(&mut self) {
        match self.view_counts.iter_mut().choose(&mut rand::rng()) {
            Some((&key, count)) => {
                *count += 1;

                let event_id = match *uuid::Uuid::new_v4().as_bytes() {
                    [.., b0, b1, b2, b3] => {
                        format!(
                            "{}-{b0:02x}{b1:02x}{b2:02x}{b3:02x}",
                            self.node_id
                        )
                    }
                };

                let count_freeze = *count;

                crate::log(
                    &self.node_id,
                    &format!("Publishing: {event_id}, {key}, {count_freeze}"),
                );
                let message = ViewEvent::new(
                    self.node_id.clone(),
                    event_id,
                    key.to_owned(),
                    count_freeze,
                    None,
                );

                for peer in self.heartbeat.get_alive_peers() {
                    if !self
                        .transport
                        .send(
                            peer.clone(),
                            Message::ViewEvent {
                                message: message.clone(),
                            },
                        )
                        .await
                    {
                        crate::warn(
                            &self.node_id,
                            &format!("Failed to send view event to {peer}"),
                        );
                    }
                }
            }
            None => crate::warn(
                &self.node_id,
                "Failed to publish: no known content",
            ),
        }
    }

    #[expect(
        clippy::future_not_send,
        reason = "I don't plan on sending these futures between threads and
        would rather not use a panicking RNG."
    )]
    pub async fn do_audit(&mut self) {
        match self
            .view_events
            .iter()
            .filter(|&(_, map)| map.len() >= 2)
            .choose(&mut rand::rng())
        {
            Some((content_id, events)) => {
                let votes: HashMap<Id, u64> = events
                    .iter()
                    .map(|(peer_id, view_event)| {
                        (peer_id.clone(), view_event.count())
                    })
                    .collect();

                let (agreed_count, confidence) =
                    self.reputation.weighted_majority_vote(&votes, true);

                match agreed_count {
                    Some(count) => {
                        for (peer_id, view_event) in events {
                            let was_accurate = view_event.count() == count;
                            self.reputation
                                .record_report(peer_id, was_accurate);
                        }
                        let voter_ids: Vec<Id> =
                            votes.keys().cloned().collect();

                        let message = AuditResult::new(
                            self.node_id.clone(),
                            content_id.clone(),
                            count,
                            confidence,
                            Some(voter_ids),
                        );

                        for peer in self.heartbeat.get_alive_peers() {
                            if !self
                                .transport
                                .send(
                                    peer.clone(),
                                    Message::AuditResult {
                                        message: message.clone(),
                                    },
                                )
                                .await
                            {
                                crate::warn(
                                    &self.node_id,
                                    &format!(
                                        "Failed to send audit result to {peer}"
                                    ),
                                );
                            }
                        }
                    }
                    None => crate::warn(
                        &self.node_id,
                        "Audit failure: agreed count was None",
                    ),
                }
            }
            None => crate::warn(
                &self.node_id,
                "Audit failure: no eligible content_id",
            ),
        }
    }

    #[expect(
        clippy::future_not_send,
        reason = "I don't plan on sending these futures between threads and
        would rather not use a panicking RNG."
    )]
    pub async fn run(&mut self) {
        self.running = true;
        crate::log(&self.node_id, "Starting main loop...");
        crate::log(&self.node_id, "Node running. Press Ctrl+C to stop.");

        while self.running {
            self.rounds += 1;
            crate::log(&self.node_id, &format!("Round {} starts", self.rounds));
            let messages =
                self.transport.receive(self.node_id.clone(), 10, 5).await;

            for (message, receipt) in messages {
                self.handle_message(&message).await;

                self.transport
                    .delete(self.node_id.clone(), receipt.clone())
                    .await;
            }

            self.run_periodic_tasks().await;
            self.gossip.age_entries();
        }

        crate::log(&self.node_id, "Main loop exited.");
    }

    pub fn shutdown(&mut self) {
        crate::log(&self.node_id, "Shutting down...");
        self.running = false;
    }

    pub fn print_status(&self) {
        crate::error(&self.node_id, "todo!: print_status");
    }
}
