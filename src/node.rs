// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! P2P Node
//! ========
//!
//! Architecture:
//!   run() polls SQS every few seconds, calls handle_message() for each
//!   received message, then calls run_periodic_tasks() for timers.

use core::str::FromStr as _;
use std::collections::HashMap;
use std::fs::File;
use std::{io, time};

use aws_sdk_sqs::Client;
use serde_json::{Map, Value};

use crate::algorithms::choking::ChokingNode;
use crate::algorithms::gossip::GossipNode;
use crate::algorithms::heartbeat::HeartbeatNode;
use crate::algorithms::reputation::ReputationNode;
use crate::protocol::{self, MessageKind};

#[derive(Debug, Clone)]
pub struct SqsTransport {
    sqs: Client,
    queue_url_cache: HashMap<String, String>,
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
            self.queue_url_cache = cache;
            true
        } else {
            false
        }
    }

    pub async fn get_queue_url(&mut self, node_id: String) -> Option<String> {
        if let Some(url) = self.queue_url_cache.get(&node_id) {
            return Some(url.clone());
        }

        let request = self.sqs.get_queue_url().queue_name("todo!()");

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

    pub async fn send(
        &mut self,
        target_node_id: String,
        message: Value,
    ) -> bool {
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
        node_id: String,
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

    pub async fn delete(&mut self, node_id: String, receipt_handle: String) {
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
    node_id: String,
    verbose: bool,
    running: bool,
    transport: SqsTransport,
    queue_url: Option<String>,
    gossip: GossipNode,
    heartbeat: HeartbeatNode,
    choking: ChokingNode,
    reputation: ReputationNode,
    gossip_interval: u16,
    heartbeat_interval: u16,
    choking_interval: u16,
    reputation_interval: u16,
    last_gossip: time::Instant,
    last_heartbeat: time::Instant,
    last_choking: time::Instant,
    last_reputation: time::Instant,
    ping_seq: u16,
    messages_received: u32,
    messages_sent: u32,
    rounds: u32,
}

impl P2PNode {
    pub async fn new(node_id: String, verbose: bool, sqs: Client) -> Self {
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
            gossip_interval: 15,
            heartbeat_interval: 10,
            choking_interval: 30,
            reputation_interval: 30,
            last_gossip: time::Instant::now(),
            last_heartbeat: time::Instant::now(),
            last_choking: time::Instant::now(),
            last_reputation: time::Instant::now(),
            ping_seq: 0,
            messages_received: 0,
            messages_sent: 0,
            rounds: 0,
        };
        this.log(&format!(
            "Initialized. Queue: {}",
            this.queue_url.clone().unwrap_or_default()
        ));
        this
    }

    pub async fn bootstrap(&mut self, bootstrap_nodes: Option<Vec<String>>) {
        let bootstrap_nodes = bootstrap_nodes.unwrap_or_else(|| {
            vec![
                "bot-alpha".to_owned(),
                "bot-bravo".to_owned(),
                "bot-charlie".to_owned(),
            ]
        });
        self.log(&format!("Bootstrapping via {bootstrap_nodes:?}..."));

        let hi = protocol::hello(
            self.node_id.clone(),
            self.queue_url.clone().unwrap_or_default(),
        );
        for node in bootstrap_nodes {
            let _: bool =
                self.transport.send(node, Value::Object(hi.clone())).await;
        }
    }

    pub fn handle_message(&mut self, message: Map<String, Value>) {
        let node_id = message.get("sender").map(ToString::to_string).unwrap();
        let message_type =
            message.get("type").map(ToString::to_string).unwrap();
        if node_id == self.node_id {
            return;
        }
        self.messages_received += 1;

        match MessageKind::from_str(&message_type) {
            Ok(kind) => match kind {
                MessageKind::Hello => self.handle_hello(&message),
                MessageKind::PeerList => self.handle_peer_list(&message),
                MessageKind::Ping => self.handle_ping(&message),
                MessageKind::Pong => self.handle_pong(&message),
                MessageKind::ViewEvent => self.handle_view_event(&message),
                MessageKind::AuditResult => self.handle_audit_result(&message),
                MessageKind::Choke => self.handle_choke(&message),
                MessageKind::Unchoke => self.handle_unchoke(message),
            },
            Err(()) => {
                self.log(&format!("Unknown message type: {message_type}"));
            }
        }
    }

    pub fn handle_hello(&mut self, message: &Map<String, Value>) {
        let node_id = message.get("sender").map(ToString::to_string).unwrap();
        let queue_url =
            message.get("queue_url").map(ToString::to_string).unwrap();
        self.log(&format!("Got a hello from: {node_id}"));
        self.gossip.add_peer(node_id.clone(), queue_url.clone());
        self.heartbeat.add_peer(node_id.clone());
        self.choking.add_peer(node_id.clone(), true);
        self.reputation.add_peer(node_id.clone());

        drop(
            self.transport
                .queue_url_cache
                .insert(node_id.clone(), queue_url),
        );

        let plist = protocol::peer_list(
            self.node_id.clone(),
            &self.gossip.get_peer_list_message(),
        );

        self.transport.send(node_id, Value::Object(plist));
    }

    pub fn handle_peer_list(&mut self, message: &Map<String, Value>) {
        let sender_id = message.get("sender").map(ToString::to_string).unwrap();
        let incoming_lol = message.get("peers").unwrap();
        let incoming: Vec<Value> = message
            .get("peers")
            .map(|f| f.as_array())
            .unwrap()
            .unwrap()
            .clone();
        self.log(&format!("Got a peer list from: {sender_id}"));
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
        let node_id = message.get("sender").map(ToString::to_string).unwrap();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "I have yet to find any good reason for this to even be
            bigger than a u8. We need a better spec."
        )]
        let seq =
            message.get("seq").map(|s| s.as_u64().unwrap()).unwrap() as u16;
        self.log(&format!("Got a ping from: {node_id} (#{seq})"));

        let response = protocol::pong(node_id.clone(), seq);
        self.transport
            .send(node_id.clone(), Value::Object(response));
        self.choking.record_contribution(&node_id, 1);
        self.reputation.record_contribution(&node_id, 1);
    }

    pub fn handle_pong(&mut self, message: &Map<String, Value>) {
        let node_id = message.get("sender").map(ToString::to_string).unwrap();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "I have yet to find any good reason for this to even be
            bigger than a u8. We need a better spec."
        )]
        let seq: u16 =
            message.get("seq").map(|s| s.as_u64().unwrap()).unwrap() as u16;
        self.log(&format!("Got a pong from: {node_id} (#{seq})"));

        self.heartbeat.receive_pong(&node_id, seq.into());
        self.reputation.record_heartbeat(&node_id, true);
    }

    pub fn handle_view_event(&self, message: &Map<String, Value>) {
        self.log("todo!: handle_view_event");
    }

    pub fn handle_audit_result(&self, message: &Map<String, Value>) {
        self.log("todo!: handle_audit_result");
    }

    pub fn handle_choke(&self, message: &Map<String, Value>) {
        self.log("todo!: handle_choke");
    }

    pub fn handle_unchoke(&self, message: Map<String, Value>) {
        self.log("todo!: handle_unchoke");
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
    }

    pub fn do_gossip(&mut self) {
        if let Some(target) = self.gossip.pick_gossip_target() {
            let peers = self.gossip.get_peer_list_message();
            let message = protocol::peer_list(self.node_id.clone(), &peers);
            self.transport.send(target.clone(), Value::Object(message));
            self.log(&format!("Sent gossip to: {target}"));
        }
    }

    pub fn do_heartbeat(&mut self) {
        self.ping_seq += 1;
        let ping = protocol::ping(self.node_id.clone(), self.ping_seq);
        for peer in self.heartbeat.send_pings(self.rounds) {
            self.transport
                .send(peer.clone(), Value::Object(ping.clone()));
            self.log(&format!(
                "Sent heartbeat to: {peer} (#{})",
                self.ping_seq
            ));
        }
    }

    pub fn do_choking(&mut self) {
        self.choking.run_choking_round();
        for log in self.choking.flush_log() {
            self.log(&log);
        }
    }

    pub fn do_reputation(&mut self) {
        self.reputation.update_all_scores();
        self.log("Updated scores");
    }

    pub async fn run(&mut self) {
        self.running = true;
        self.log("Starting main loop...");
        println!("\n[{}] Node running. Press Ctrl+C to stop.", self.node_id);

        while self.running {
            self.rounds += 1;
            self.log(&format!("Round {} starts", self.rounds));
            let messages =
                self.transport.receive(self.node_id.clone(), 10, 5).await;

            for message in messages {
                let receipt =
                    message.get("_receipt_handle").map(ToString::to_string);
                self.handle_message(message);
                if let Some(receipt) = receipt {
                    self.transport.delete(self.node_id.clone(), receipt);
                }
            }

            self.run_periodic_tasks();
            self.gossip.age_entries();
        }

        self.log("Main loop exited.");
    }

    pub fn shutdown(&mut self) {
        self.log("Shutting down...");
        self.running = false;
    }

    pub fn print_status(&self) {
        self.log("todo!: print_status");
    }

    pub fn log(&self, message: &str) {
        let timestamp = jiff::Timestamp::now().to_string();
        println!("[{timestamp}] [{}] {message}", self.node_id);
    }
}
