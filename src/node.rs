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
use core::str::FromStr as _;
use std::collections::HashMap;
use std::fs::File;
use std::{io, time};

use aws_sdk_sqs::Client;
use owo_colors::OwoColorize as _;
use rand::seq::IteratorRandom as _;
use serde_json::{Map, Value};

use crate::algorithms::choking::ChokingNode;
use crate::algorithms::content::ViewEvent;
use crate::algorithms::gossip::GossipNode;
use crate::algorithms::heartbeat::HeartbeatNode;
use crate::algorithms::reputation::ReputationNode;
use crate::protocol::{self, MessageKind};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
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

impl SqsTransport {
    pub fn new(sqs: Client) -> Self {
        Self {
            sqs,
            queue_url_cache: HashMap::new(),
        }
    }

    pub fn load_resources(&mut self, disk_cache: File) -> bool {
        let disk_cache = io::BufReader::new(disk_cache);
        let cache: Result<Value, _> = serde_json::from_reader(disk_cache);
        let cache: Option<HashMap<String, String>> = cache
            .map(|cache: Value| {
                let cache = cache.as_object()?;
                cache.get("queues").and_then(|cache: &Value| {
                    serde_json::from_value(cache.clone()).ok()
                })
            })
            .ok()
            .flatten();

        if let Some(cache) = cache {
            self.queue_url_cache = cache
                .into_iter()
                .map(|(id, url)| (Id::new(id), url))
                .collect();
            true
        } else {
            false
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

    pub async fn send(&mut self, target_node_id: Id, message: Value) -> bool {
        if let Some(url) = self.get_queue_url(target_node_id.clone()).await {
            match self
                .sqs
                .send_message()
                .queue_url(url)
                .message_body(message.to_string())
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
        false
    }

    pub async fn receive(
        &mut self,
        node_id: Id,
        max_messages: i32,
        wait_seconds: i32,
    ) -> Vec<Map<String, Value>> {
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
                                && let Ok(mut deserialized) =
                                    serde_json::from_str::<Map<String, Value>>(
                                        body,
                                    )
                                && let Some(handle) = sqs_msg.receipt_handle
                            {
                                drop(deserialized.insert(
                                    "_receipt_handle".to_owned(),
                                    Value::String(handle),
                                ));

                                messages.push(deserialized);
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
            self.sqs
                .delete_message()
                .queue_url(url)
                .receipt_handle(receipt_handle)
                .send()
                .await;
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
    view_events: HashMap<String, HashMap<Id, ViewEvent>>,
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
        let disk_cache = File::open("resources.json").unwrap();
        let _: bool = transport.load_resources(disk_cache);
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

        let hi = protocol::hello(
            &self.node_id,
            self.queue_url.clone().unwrap_or_default(),
        );
        for node in bootstrap_nodes {
            let _: bool =
                self.transport.send(node, Value::Object(hi.clone())).await;
        }
    }

    pub fn handle_message(&mut self, message: &Map<String, Value>) {
        let node_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();
        let message_type = message.get("type").and_then(Value::as_str).unwrap();
        if node_id == self.node_id {
            return;
        }
        self.messages_received += 1;

        if let Some(sender) = self.gossip.peers_mut().get_mut(&node_id) {
            *sender.time_to_live_mut() = 5;
        }

        match MessageKind::from_str(message_type) {
            Ok(kind) => match kind {
                MessageKind::Hello => self.handle_hello(message),
                MessageKind::PeerList => self.handle_peer_list(message),
                MessageKind::Ping => self.handle_ping(message),
                MessageKind::Pong => self.handle_pong(message),
                MessageKind::ViewEvent => self.handle_view_event(message),
                MessageKind::AuditResult => self.handle_audit_result(message),
                MessageKind::Choke => self.handle_choke(message),
                MessageKind::Unchoke => self.handle_unchoke(message),
            },
            Err(()) => {
                crate::warn(
                    &self.node_id,
                    &format!("Unknown message type: {message_type}"),
                );
            }
        }
    }

    pub fn handle_hello(&mut self, message: &Map<String, Value>) {
        let node_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();
        let queue_url =
            message.get("queue_url").and_then(Value::as_str).unwrap();
        crate::log(&self.node_id, &format!("Got a hello from: {node_id}"));
        self.gossip.add_peer(node_id.clone(), queue_url.to_owned());
        self.heartbeat.add_peer(node_id.clone());
        self.choking.add_peer(node_id.clone(), true);
        self.reputation.add_peer(node_id.clone());

        drop(
            self.transport
                .queue_url_cache
                .insert(node_id.clone(), queue_url.to_owned()),
        );

        let plist = protocol::peer_list(
            &self.node_id,
            &self.gossip.get_peer_list_message(),
        );

        self.transport.send(node_id, Value::Object(plist));
    }

    pub fn handle_peer_list(&mut self, message: &Map<String, Value>) {
        let sender_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();
        let incoming_lol = message.get("peers").unwrap();
        let incoming: Vec<Value> = message
            .get("peers")
            .map(|f| f.as_array())
            .unwrap()
            .unwrap()
            .clone();
        crate::log(
            &self.node_id,
            &format!("Got a peer list from: {sender_id}"),
        );
        let _: u8 = self.gossip.receive_peer_list(incoming, &sender_id);

        for (peer, record) in self.gossip.peers() {
            self.heartbeat.add_peer(peer.clone());
            self.choking.add_peer(peer.clone(), true);
            self.reputation.add_peer(peer.clone());
            drop(
                self.transport
                    .queue_url_cache
                    .insert(peer.clone(), record.queue_url().to_owned()),
            );
        }
    }

    pub fn handle_ping(&mut self, message: &Map<String, Value>) {
        let node_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "I have yet to find any good reason for this to even be
            bigger than a u8. We need a better spec."
        )]
        let seq =
            message.get("seq").map(|s| s.as_u64().unwrap()).unwrap() as u16;
        crate::log(
            &self.node_id,
            &format!("Got a ping from: {node_id} (#{seq})"),
        );

        let response = protocol::pong(&node_id.clone(), seq);
        self.transport
            .send(node_id.clone(), Value::Object(response));
        self.choking.record_contribution(&node_id, 1);
        self.reputation.record_contribution(&node_id, 1);
    }

    pub fn handle_pong(&mut self, message: &Map<String, Value>) {
        let node_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "I have yet to find any good reason for this to even be
            bigger than a u8. We need a better spec."
        )]
        let seq: u16 =
            message.get("seq").map(|s| s.as_u64().unwrap()).unwrap() as u16;
        crate::log(
            &self.node_id,
            &format!("Got a pong from: {node_id} (#{seq})"),
        );

        self.heartbeat.receive_pong(&node_id, seq.into());
        self.reputation.record_heartbeat(&node_id, true);
    }

    pub fn handle_view_event(&mut self, message: &Map<String, Value>) {
        let node_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();
        let event_id = message
            .get("event_id")
            .and_then(Value::as_str)
            .unwrap()
            .to_owned();
        let content_id = message
            .get("content_id")
            .and_then(Value::as_str)
            .unwrap()
            .to_owned();
        let count = message.get("count").and_then(Value::as_u64).unwrap();
        let ad_id = message
            .get("ad_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .filter(|inner| !inner.is_empty());
        let view_event = ViewEvent::new(
            node_id.clone(),
            event_id,
            content_id.clone(),
            count,
            ad_id,
        );
        crate::log(
            &self.node_id,
            &format!("View event from {node_id}: {view_event:?}"),
        );
        match self.view_events.get_mut(&content_id) {
            Some(map) => {
                drop(map.insert(node_id.clone(), view_event));
            }
            None => {
                drop(self.view_events.insert(
                    content_id,
                    HashMap::from_iter([(node_id.clone(), view_event)]),
                ));
            }
        }
        self.choking.record_contribution(&node_id, 1);
        self.reputation.record_contribution(&node_id, 1);
    }

    pub fn handle_audit_result(&mut self, message: &Map<String, Value>) {
        let node_id = message
            .get("sender")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .map(Id::new)
            .unwrap();

        let content_id =
            message.get("content_id").and_then(Value::as_str).unwrap();

        let agreed_count =
            message.get("agreed_count").and_then(Value::as_u64).unwrap();
        let confidence =
            message.get("confidence").and_then(Value::as_f64).unwrap() * 100.0;
        let voters: Vec<&str> = message
            .get("voters")
            .and_then(Value::as_array)
            .map(|vec| vec.iter().map(|v| v.as_str().unwrap()).collect())
            .unwrap();

        let log = format!(
            "Audit result from {node_id}: \"{content_id}\", {agreed_count} \
            views, {confidence:.2}% confidence, as voted by {voters:?}"
        );
        crate::log(&self.node_id, &log);
        self.reputation.record_contribution(&node_id, 1);
    }

    pub fn handle_choke(&self, message: &Map<String, Value>) {
        let node_id = message.get("sender").and_then(Value::as_str).unwrap();
        crate::log(&self.node_id, &format!("Choked by {node_id}"));
    }

    pub fn handle_unchoke(&self, message: &Map<String, Value>) {
        let node_id = message.get("sender").and_then(Value::as_str).unwrap();
        crate::log(&self.node_id, &format!("Unchoked by {node_id}"));
    }

    pub fn run_periodic_tasks(&mut self) {
        let now = time::Instant::now();

        if now - self.last_gossip
            >= time::Duration::from_secs(self.gossip_interval.into())
        {
            self.do_gossip();
            self.last_gossip = now;
        }

        if now - self.last_heartbeat
            >= time::Duration::from_secs(self.heartbeat_interval.into())
        {
            self.do_heartbeat();
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
            self.do_publish();
            self.last_publish = now;
        }

        if now - self.last_audit
            >= time::Duration::from_secs(self.audit_interval.into())
        {
            self.do_audit();
            self.last_audit = now;
        }
    }

    pub fn do_gossip(&mut self) {
        if let Some(target) = self.gossip.pick_gossip_target() {
            let peers = self.gossip.get_peer_list_message();
            let message = protocol::peer_list(&self.node_id, &peers);
            self.transport.send(target.clone(), Value::Object(message));
            crate::log(&self.node_id, &format!("Sent gossip to: {target}"));
        }
    }

    pub fn do_heartbeat(&mut self) {
        self.ping_seq += 1;
        let ping = protocol::ping(&self.node_id, self.ping_seq);
        for peer in self.heartbeat.send_pings(self.rounds) {
            self.transport
                .send(peer.clone(), Value::Object(ping.clone()));
            crate::log(
                &self.node_id,
                &format!("Sent heartbeat to: {peer} (#{})", self.ping_seq),
            );
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

    pub fn do_publish(&mut self) {
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
                let message = protocol::view_event(
                    &self.node_id,
                    event_id,
                    key.to_owned(),
                    count_freeze,
                    String::new(),
                );

                for peer in self.heartbeat.get_alive_peers() {
                    self.transport.send(peer, Value::Object(message.clone()));
                }
            }
            None => crate::warn(
                &self.node_id,
                "Failed to publish: no known content",
            ),
        }
    }

    pub fn do_audit(&mut self) {
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

                        let message = protocol::audit_result(
                            &self.node_id,
                            content_id.clone(),
                            count,
                            confidence,
                            Some(voter_ids),
                        );

                        for peer in self.heartbeat.get_alive_peers() {
                            self.transport
                                .send(peer, Value::Object(message.clone()));
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

    pub async fn run(&mut self) {
        self.running = true;
        crate::log(&self.node_id, "Starting main loop...");
        crate::log(&self.node_id, "Node running. Press Ctrl+C to stop.");

        while self.running {
            self.rounds += 1;
            crate::log(&self.node_id, &format!("Round {} starts", self.rounds));
            let messages =
                self.transport.receive(self.node_id.clone(), 10, 5).await;

            for message in messages {
                let receipt =
                    message.get("_receipt_handle").and_then(Value::as_str);
                self.handle_message(&message);
                if let Some(receipt) = receipt {
                    self.transport
                        .delete(self.node_id.clone(), receipt.to_owned());
                }
            }

            self.run_periodic_tasks();
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
