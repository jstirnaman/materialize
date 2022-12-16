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

//! Abstractions for management of cloud resources that have no equivalent when running
//! locally, like AWS PrivateLink endpoints.

use std::collections::HashSet;
use std::fmt::{self, Debug};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use mz_repr::GlobalId;

pub mod crd;

/// A prefix for an [external ID] to use for all AWS AssumeRole operations. It
/// should be concatenanted with a non-user-provided suffix identifying the
/// source or sink. The ID used for the suffix should never be reused if the
/// source or sink is deleted.
///
/// **WARNING:** it is critical for security that this ID is **not** provided by
/// end users of Materialize. It must be provided by the operator of the
/// Materialize service.
///
/// This type protects against accidental construction of an
/// `AwsExternalIdPrefix` through the use of an unwieldy and overly descriptive
/// constructor method name.
///
/// [external ID]: https://docs.aws.amazon.com/IAM/latest/UserGuide/id_roles_create_for-user_externalid.html
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AwsExternalIdPrefix(String);

impl AwsExternalIdPrefix {
    /// Creates a new AWS external ID prefix from a command-line argument or
    /// an environment variable.
    ///
    /// **WARNING:** it is critical for security that this ID is **not**
    /// provided by end users of Materialize. It must be provided by the
    /// operator of the Materialize service.
    ///
    pub fn new_from_cli_argument_or_environment_variable(
        aws_external_id_prefix: &str,
    ) -> AwsExternalIdPrefix {
        AwsExternalIdPrefix(aws_external_id_prefix.into())
    }
}

impl fmt::Display for AwsExternalIdPrefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Configures a VPC endpoint.
pub struct VpcEndpointConfig {
    /// The name of the service to connect to.
    pub aws_service_name: String,
    /// The IDs of the availability zones in which the service is available.
    pub availability_zone_ids: Vec<String>,
}

#[async_trait]
pub trait CloudResourceController: Debug + Send + Sync {
    /// Creates or updates the specified `VpcEndpoint` Kubernetes object.
    async fn ensure_vpc_endpoint(
        &self,
        id: GlobalId,
        vpc_endpoint: VpcEndpointConfig,
    ) -> Result<(), anyhow::Error>;

    /// Deletes the specified `VpcEndpoint` Kubernetes object.
    async fn delete_vpc_endpoint(&self, id: GlobalId) -> Result<(), anyhow::Error>;

    /// Lists existing `VpcEndpoint` Kubernetes objects.
    async fn list_vpc_endpoints(&self) -> Result<HashSet<GlobalId>, anyhow::Error>;
}

/// Returns the name to use for the VPC endpoint with the given ID.
pub fn vpc_endpoint_name(id: GlobalId) -> String {
    // This is part of the contract with the VpcEndpointController in the
    // cloud infrastructure layer.
    format!("connection-{id}")
}
