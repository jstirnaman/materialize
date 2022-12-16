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

//! Abstractions for secure management of user secrets.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use async_trait::async_trait;

use mz_repr::GlobalId;

/// Securely manages user secrets.
#[async_trait]
pub trait SecretsController: Debug + Send + Sync {
    /// Creates or updates the specified secret with the specified binary
    /// contents.
    async fn ensure(&self, id: GlobalId, contents: &[u8]) -> Result<(), anyhow::Error>;

    /// Deletes the specified secret.
    async fn delete(&self, id: GlobalId) -> Result<(), anyhow::Error>;

    /// Returns a reader for the secrets managed by this controller.
    fn reader(&self) -> Arc<dyn SecretsReader>;
}

/// Securely reads secrets that are managed by a [`SecretsController`].
///
/// Does not provide access to create, update, or delete the secrets within.
#[async_trait]
pub trait SecretsReader: Debug + Send + Sync {
    /// Returns the binary contents of the specified secret.
    async fn read(&self, id: GlobalId) -> Result<Vec<u8>, anyhow::Error>;

    /// Returns the string contents of the specified secret.
    ///
    /// Returns an error if the secret's contents cannot be decoded as UTF-8.
    async fn read_string(&self, id: GlobalId) -> Result<String, anyhow::Error> {
        let contents = self.read(id).await?;
        String::from_utf8(contents).context("converting secret value to string")
    }
}

#[derive(Debug)]
pub struct InMemorySecretsController {
    data: Arc<Mutex<HashMap<GlobalId, Vec<u8>>>>,
}

impl InMemorySecretsController {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SecretsController for InMemorySecretsController {
    async fn ensure(&self, id: GlobalId, contents: &[u8]) -> Result<(), anyhow::Error> {
        self.data.lock().unwrap().insert(id, contents.to_vec());
        Ok(())
    }

    async fn delete(&self, id: GlobalId) -> Result<(), anyhow::Error> {
        self.data.lock().unwrap().remove(&id);
        Ok(())
    }

    fn reader(&self) -> Arc<dyn SecretsReader> {
        Arc::new(InMemorySecretsController {
            data: Arc::clone(&self.data),
        })
    }
}

#[async_trait]
impl SecretsReader for InMemorySecretsController {
    async fn read(&self, id: GlobalId) -> Result<Vec<u8>, anyhow::Error> {
        let contents = self.data.lock().unwrap().get(&id).cloned();
        contents.ok_or_else(|| anyhow::anyhow!("secret does not exist"))
    }
}
