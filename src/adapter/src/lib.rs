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

#![cfg_attr(nightly_doc_features, feature(doc_cfg))]

//! Coordinates client requests with the dataflow layer.
//!
//! This crate hosts the "coordinator", an object which sits at the center of
//! the system and coordinates communication between the various components.
//! Responsibilities of the coordinator include:
//!
//!   * Launching the dataflow workers.
//!   * Periodically allowing the dataflow workers to compact existing data.
//!   * Executing SQL queries from clients by parsing and planning them, sending
//!     the plans to the dataflow layer, and then streaming the results back to
//!     the client.
//!   * Assigning timestamps to incoming source data.
//!
//! The main interface to the coordinator is [`Client`]. To start a coordinator,
//! use the [`serve`] function.

// TODO(benesch): delete this once we use structured errors everywhere.
macro_rules! coord_bail {
    ($($e:expr),*) => {
        return Err(crate::error::AdapterError::Unstructured(::anyhow::anyhow!($($e),*)))
    }
}

mod command;
mod coord;
mod error;
mod explain_new;
mod notice;
mod subscribe;
mod util;

pub mod catalog;
pub mod client;
pub mod config;
pub mod metrics;
pub mod session;
pub mod telemetry;

pub use crate::client::{Client, ConnClient, Handle, SessionClient};
pub use crate::command::{
    Canceled, ExecuteResponse, ExecuteResponseKind, RowsFuture, StartupMessage, StartupResponse,
};
pub use crate::coord::peek::PeekResponseUnary;
pub use crate::coord::timestamp_selection::TimestampExplanation;
pub use crate::coord::{serve, Config, DUMMY_AVAILABILITY_ZONE};
pub use crate::error::AdapterError;
pub use crate::notice::AdapterNotice;
