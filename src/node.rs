// SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! P2P Node
//! ========
//!
//! Architecture:
//!   run() polls SQS every few seconds, calls handle_message() for each
//!   received message, then calls run_periodic_tasks() for timers.

use std::collections::HashMap;
use std::fs::File;
use std::{io, time};

use aws_sdk_sqs::Client;
use serde_json::{Map, Value};

use crate::algorithms::choking::ChokingNode;
use crate::algorithms::gossip::GossipNode;
use crate::algorithms::heartbeat::HeartbeatNode;
use crate::algorithms::reputation::ReputationNode;
use crate::protocol;

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

    pub fn handle_message(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_hello(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_peer_list(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_ping(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_pong(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_view_event(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_audit_result(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_choke(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn handle_unchoke(&self, message: Map<String, Value>) {
        todo!()
    }

    pub fn run_periodic_tasks(&self) {
        todo!()
    }

    pub fn do_gossip(&self) {
        todo!()
    }

    pub fn do_heartbeat(&self) {
        todo!()
    }

    pub fn do_choking(&self) {
        todo!()
    }

    pub fn do_reputation(&self) {
        todo!()
    }

    pub fn run(&self) {
        todo!()
    }

    pub fn shutdown(&self) {
        todo!()
    }

    pub fn print_status(&self) {
        todo!()
    }

    pub fn log(&self, message: &str) {
        todo!()
    }
}
