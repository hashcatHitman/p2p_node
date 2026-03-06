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
use std::io;

use aws_sdk_sqs::Client;
use serde_json::Map;

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
        let cache: Result<serde_json::Value, _> =
            serde_json::from_reader(disk_cache);
        let cache: Option<HashMap<String, String>> = cache
            .map(|cache: serde_json::Value| {
                let cache = cache.as_object()?;
                cache.get("queues").and_then(|cache: &serde_json::Value| {
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
        message: serde_json::Value,
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
    ) -> Vec<Map<String, serde_json::Value>> {
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
                                    serde_json::from_str::<
                                        Map<String, serde_json::Value>,
                                    >(body)
                                && let Some(handle) = sqs_msg.receipt_handle
                            {
                                drop(deserialized.insert(
                                    "_receipt_handle".to_owned(),
                                    serde_json::Value::String(handle),
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
