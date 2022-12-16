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

//! AWS Kinesis utilities.

use aws_sdk_kinesis::error::{GetShardIteratorError, ListShardsError};
use aws_sdk_kinesis::model::{Shard, ShardIteratorType};
use aws_sdk_kinesis::types::SdkError;
use aws_sdk_kinesis::Client;

/// Lists the shards of the named Kinesis stream.
///
/// This function wraps the `ListShards` API call. It returns all shards in a
/// given Kinesis stream, automatically handling pagination if required.
///
/// # Errors
///
/// Any errors from the underlying `GetShardIterator` API call are surfaced
/// directly.
pub async fn list_shards(
    client: &aws_sdk_kinesis::Client,
    stream_name: &str,
) -> Result<Vec<Shard>, SdkError<ListShardsError>> {
    let mut next_token = None;
    let mut shards = Vec::new();
    loop {
        let res = client
            .list_shards()
            .set_next_token(next_token)
            .stream_name(stream_name)
            .send()
            .await?;
        shards.extend(res.shards.unwrap_or_else(Vec::new));
        if res.next_token.is_some() {
            next_token = res.next_token;
        } else {
            return Ok(shards);
        }
    }
}

/// Gets the shard IDs of the named Kinesis stream.
///
/// This function is like [`list_shards`], but
///
/// # Errors
///
/// Any errors from the underlying `GetShardIterator` API call are surfaced
/// directly.
pub async fn get_shard_ids(
    client: &Client,
    stream_name: &str,
) -> Result<impl Iterator<Item = String>, SdkError<ListShardsError>> {
    let res = list_shards(client, stream_name).await?;
    Ok(res
        .into_iter()
        .map(|s| s.shard_id.unwrap_or_else(|| "".into())))
}

/// Constructs an iterator over a Kinesis shard.
///
/// This function is a wrapper around around the `GetShardIterator` API. It
/// returns the `TRIM_HORIZON` shard iterator of a given stream and shard,
/// meaning it will return the location in the shard with the oldest data
/// record.
///
/// # Errors
///
/// Any errors from the underlying `GetShardIterator` API call are surfaced
/// directly.
pub async fn get_shard_iterator(
    client: &Client,
    stream_name: &str,
    shard_id: &str,
) -> Result<Option<String>, SdkError<GetShardIteratorError>> {
    let res = client
        .get_shard_iterator()
        .stream_name(stream_name)
        .shard_id(shard_id)
        .shard_iterator_type(ShardIteratorType::TrimHorizon)
        .send()
        .await?;
    Ok(res.shard_iterator)
}
