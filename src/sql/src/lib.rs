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

//! SQL-dataflow translation.
//!
//! There are two main parts of the SQL–dataflow translation process:
//!
//!   * **Purification** eliminates any external state from a SQL AST. It is an
//!     asynchronous process that may make network calls to external services.
//!     The input and output of purification is a SQL AST.
//!
//!   * **Planning** converts a purified AST to a [`Plan`], which describes an
//!     action that the system should take to effect the results of the query.
//!     Planning is a fast, pure function that always produces the same plan for
//!     a given input.
//!
//! # Details
//!
//! The purification step is, to our knowledge, unique to Materialize. In other
//! SQL databases, there is no concept of purifying a statement before planning
//! it. The reason for this difference is that in Materialize SQL statements can
//! depend on external state: local files, Confluent Schema Registries, etc.
//!
//! Presently only `CREATE SOURCE` statements can depend on external state,
//! though this could change in the future. Consider, for example:
//!
//! ```sql
//! CREATE SOURCE ... FORMAT AVRO USING CONFLUENT SCHEMA REGISTRY 'http://csr:8081'
//! ```
//!
//! The shape of the created source is dependent on the Avro schema that is
//! stored in the schema registry running at `csr:8081`.
//!
//! This is problematic, because we need planning to be a pure function of its
//! input. Why?
//!
//!   * Planning locks the catalog while it operates. Therefore it needs to be
//!     fast, because only one SQL query can be planned at a time. Depending on
//!     external state while holding a lock on the catalog would be seriously
//!     detrimental to the latency of other queries running on the system.
//!
//!   * The catalog persists SQL ASTs across restarts of Materialize. If those
//!     ASTs depend on external state, then changes to that external state could
//!     corrupt Materialize's catalog.
//!
//! Purification is the escape hatch. It is a transformation from SQL AST to SQL
//! AST that "inlines" any external state. For example, we purify the schema
//! above by fetching the schema from the schema registry and inlining it.
//!
//! ```sql
//! CREATE SOURCE ... FORMAT AVRO USING SCHEMA '{"name": "foo", "fields": [...]}'
//! ```
//!
//! Importantly, purification cannot hold its reference to the catalog across an
//! await point. That means it can run in its own Tokio task so that it does not
//! block any other SQL commands on the server.
//!
//! [`Plan`]: crate::plan::Plan

#![warn(missing_debug_implementations)]

macro_rules! bail_unsupported {
    ($feature:expr) => {
        return Err(crate::plan::error::PlanError::Unsupported {
            feature: $feature.to_string(),
            issue_no: None,
        }
        .into())
    };
    ($issue:expr, $feature:expr) => {
        return Err(crate::plan::error::PlanError::Unsupported {
            feature: $feature.to_string(),
            issue_no: Some($issue),
        }
        .into())
    };
}

// TODO(benesch): delete these macros once we use structured errors everywhere.
macro_rules! sql_bail {
    ($($e:expr),* $(,)?) => {
        return Err(sql_err!($($e),*))
    }
}
macro_rules! sql_err {
    ($($e:expr),* $(,)?) => {
        crate::plan::error::PlanError::Unstructured(format!($($e),*))
    }
}

pub const DEFAULT_SCHEMA: &str = "public";

pub mod ast;
pub mod catalog;
pub mod func;
pub mod kafka_util;
pub mod names;
#[macro_use]
pub mod normalize;
pub mod parse;
pub mod plan;
pub mod pure;
pub mod query_model;
