// BEGIN LINT CONFIG
// DO NOT EDIT - see bin/gen-lints
#![allow(clippy::style)]
#![allow(clippy::complexity)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::mutable_key_type)]
#![allow(clippy::needless_collect)]
#![allow(clippy::stable_sort_primitive)]
#![allow(clippy::map_entry)]
#![allow(clippy::box_default)]
#![deny(warnings)]
#![deny(clippy::bool_comparison)]
#![deny(clippy::clone_on_ref_ptr)]
#![deny(clippy::no_effect)]
#![deny(clippy::unnecessary_unwrap)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::todo)]
#![deny(clippy::wildcard_dependencies)]
#![deny(clippy::zero_prefixed_literal)]
#![deny(clippy::borrowed_box)]
#![deny(clippy::deref_addrof)]
#![deny(clippy::double_must_use)]
#![deny(clippy::double_parens)]
#![deny(clippy::extra_unused_lifetimes)]
#![deny(clippy::needless_borrow)]
#![deny(clippy::needless_question_mark)]
#![deny(clippy::needless_return)]
#![deny(clippy::redundant_pattern)]
#![deny(clippy::redundant_slicing)]
#![deny(clippy::redundant_static_lifetimes)]
#![deny(clippy::single_component_path_imports)]
#![deny(clippy::unnecessary_cast)]
#![deny(clippy::useless_asref)]
#![deny(clippy::useless_conversion)]
#![deny(clippy::builtin_type_shadow)]
#![deny(clippy::duplicate_underscore_argument)]
#![deny(clippy::double_neg)]
#![deny(clippy::unnecessary_mut_passed)]
#![deny(clippy::wildcard_in_or_patterns)]
#![deny(clippy::collapsible_if)]
#![deny(clippy::collapsible_else_if)]
#![deny(clippy::crosspointer_transmute)]
#![deny(clippy::excessive_precision)]
#![deny(clippy::overflow_check_conditional)]
#![deny(clippy::as_conversions)]
#![deny(clippy::match_overlapping_arm)]
#![deny(clippy::zero_divided_by_zero)]
#![deny(clippy::must_use_unit)]
#![deny(clippy::suspicious_assignment_formatting)]
#![deny(clippy::suspicious_else_formatting)]
#![deny(clippy::suspicious_unary_op_formatting)]
#![deny(clippy::mut_mutex_lock)]
#![deny(clippy::print_literal)]
#![deny(clippy::same_item_push)]
#![deny(clippy::useless_format)]
#![deny(clippy::write_literal)]
#![deny(clippy::redundant_closure)]
#![deny(clippy::redundant_closure_call)]
#![deny(clippy::unnecessary_lazy_evaluations)]
#![deny(clippy::partialeq_ne_impl)]
#![deny(clippy::redundant_field_names)]
#![deny(clippy::transmutes_expressible_as_ptr_casts)]
#![deny(clippy::unused_async)]
#![deny(clippy::disallowed_methods)]
#![deny(clippy::from_over_into)]
// END LINT CONFIG
// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Segment library for Rust.
//!
//! This crate provides a library to the [Segment] analytics platform.
//! It is a small wrapper around the [`segment`] crate to provide a more
//! ergonomic interface.
//!
//! [Segment]: https://segment.com
//! [`segment`]: https://docs.rs/segment

use segment::message::{Batch, BatchMessage, Group, Message, Track, User};
use segment::{Batcher, Client as _, HttpClient};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::warn;
use uuid::Uuid;

/// The maximum number of undelivered events. Once this limit is reached,
/// new events will be dropped.
const MAX_PENDING_EVENTS: usize = 32_768;

/// A [Segment] API client.
///
/// Event delivery is best effort. There is no guarantee that a given event
/// will be delivered to Segment.
///
/// [Segment]: https://segment.com
#[derive(Clone)]
pub struct Client {
    tx: Sender<BatchMessage>,
}

impl Client {
    /// Creates a new client.
    pub fn new(api_key: String) -> Client {
        let (tx, rx) = mpsc::channel(MAX_PENDING_EVENTS);

        let send_task = SendTask {
            api_key,
            http_client: HttpClient::default(),
        };
        mz_ore::task::spawn(
            || "segment_send_task",
            async move { send_task.run(rx).await },
        );

        Client { tx }
    }

    /// Sends a new [track event] to Segment.
    ///
    /// Delivery happens asynchronously on a background thread. It is best
    /// effort. There is no guarantee that the event will be delivered to
    /// Segment. Events may be dropped when the client is backlogged. Errors are
    /// logged but not returned.
    ///
    /// [track event]: https://segment.com/docs/connections/spec/track/
    pub fn track<S>(
        &self,
        user_id: Uuid,
        event: S,
        properties: serde_json::Value,
        context: Option<serde_json::Value>,
    ) where
        S: Into<String>,
    {
        self.send(BatchMessage::Track(Track {
            user: User::UserId {
                user_id: user_id.to_string(),
            },
            event: event.into(),
            properties,
            context,
            ..Default::default()
        }));
    }

    /// Sends a new [group event] to Segment.
    ///
    /// Delivery happens asynchronously on a background thread. It is best
    /// effort. There is no guarantee that the event will be delivered to
    /// Segment. Events may be dropped when the client is backlogged. Errors are
    /// logged but not returned.
    ///
    /// [track event]: https://segment.com/docs/connections/spec/group/
    pub fn group(&self, user_id: Uuid, group_id: Uuid, traits: serde_json::Value) {
        self.send(BatchMessage::Group(Group {
            user: User::UserId {
                user_id: user_id.to_string(),
            },
            group_id: group_id.to_string(),
            traits,
            ..Default::default()
        }));
    }

    fn send(&self, message: BatchMessage) {
        match self.tx.try_send(message) {
            Ok(()) => (),
            Err(TrySendError::Closed(_)) => panic!("receiver must not drop first"),
            Err(TrySendError::Full(_)) => {
                warn!("dropping segment event because queue is full");
            }
        }
    }
}

struct SendTask {
    api_key: String,
    http_client: HttpClient,
}

impl SendTask {
    async fn run(&self, mut rx: Receiver<BatchMessage>) {
        // On each turn of the loop, we accumulate all outstanding messages and
        // send them to Segment in the largest batches possible. We never have
        // more than one outstanding request to Segment.
        loop {
            let mut batcher = Batcher::new(None);

            // Wait for the first event to arrive.
            match rx.recv().await {
                Some(message) => batcher = self.enqueue(batcher, message).await,
                None => return,
            };

            // Accumulate any other messages that are ready. `enqueue` may
            // flush the batch to Segment if we hit the maximum batch size.
            while let Ok(message) = rx.try_recv() {
                batcher = self.enqueue(batcher, message).await;
            }

            // Drain the queue.
            self.flush(batcher).await;
        }
    }

    async fn enqueue(&self, mut batcher: Batcher, message: BatchMessage) -> Batcher {
        match batcher.push(message) {
            Ok(None) => (),
            Ok(Some(message)) => {
                self.flush(batcher).await;
                batcher = Batcher::new(None);
                batcher
                    .push(message)
                    .expect("message cannot fail to enqueue twice");
            }
            Err(e) => {
                warn!("error enqueueing segment message: {}", e);
            }
        }
        batcher
    }

    async fn flush(&self, batcher: Batcher) {
        let message = batcher.into_message();
        if matches!(&message, Message::Batch(Batch { batch , .. }) if batch.is_empty()) {
            return;
        }
        if let Err(e) = self.http_client.send(self.api_key.clone(), message).await {
            warn!("error sending message to segment: {}", e);
        }
    }
}
