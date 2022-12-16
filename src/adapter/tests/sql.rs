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
// Copyright 2018 sqlparser-rs contributors. All rights reserved.
// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// This file is derived from the sqlparser-rs project, available at
// https://github.com/andygrove/sqlparser-rs. It was incorporated
// directly into Materialize on December 21, 2019.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use tokio::sync::Mutex;

use mz_adapter::catalog::{Catalog, CatalogItem, Op, Table, SYSTEM_CONN_ID};
use mz_adapter::session::{Session, DEFAULT_DATABASE_NAME};
use mz_ore::now::NOW_ZERO;
use mz_repr::RelationDesc;
use mz_sql::ast::{Expr, Statement};
use mz_sql::catalog::CatalogDatabase;
use mz_sql::names::{self, ObjectQualifiers, QualifiedObjectName, ResolvedDatabaseSpecifier};
use mz_sql::plan::{PlanContext, QueryContext, QueryLifetime, StatementContext};
use mz_sql::DEFAULT_SCHEMA;

// This morally tests the name resolution stuff, but we need access to a
// catalog.

#[tokio::test]
async fn datadriven() {
    datadriven::walk_async("tests/testdata", |mut f| async {
        // The datadriven API takes an `FnMut` closure, and can't express to Rust that
        // it will finish polling each returned future before calling the closure
        // again, so we have to wrap the catalog in a share-able type. Datadriven
        // could be changed to maybe take some context that it passes as a &mut to each
        // test_case invocation, act on a stream of test_cases, or take and return a
        // Context. This is just a test, so the performance hit of this doesn't matter
        // (and in practice there will be no contention).
        let catalog = Arc::new(Mutex::new(
            Catalog::open_debug_memory(NOW_ZERO.clone()).await.unwrap(),
        ));
        f.run_async(|test_case| {
            let catalog = Arc::clone(&catalog);
            async move {
                let mut catalog = catalog.lock().await;
                match test_case.directive.as_str() {
                    "add-table" => {
                        let id = catalog.allocate_user_id().await.unwrap();
                        let oid = catalog.allocate_oid().unwrap();
                        let database_id = catalog
                            .resolve_database(DEFAULT_DATABASE_NAME)
                            .unwrap()
                            .id();
                        let database_spec = ResolvedDatabaseSpecifier::Id(database_id);
                        let schema_spec = catalog
                            .resolve_schema_in_database(
                                &database_spec,
                                DEFAULT_SCHEMA,
                                SYSTEM_CONN_ID,
                            )
                            .unwrap()
                            .id
                            .clone();
                        catalog
                            .transact(
                                None,
                                vec![Op::CreateItem {
                                    id,
                                    oid,
                                    name: QualifiedObjectName {
                                        qualifiers: ObjectQualifiers {
                                            database_spec,
                                            schema_spec,
                                        },
                                        item: test_case.input.trim_end().to_string(),
                                    },
                                    item: CatalogItem::Table(Table {
                                        create_sql: "TODO".to_string(),
                                        desc: RelationDesc::empty(),
                                        defaults: vec![Expr::null(); 0],
                                        conn_id: None,
                                        depends_on: vec![],
                                    }),
                                }],
                                |_| Ok(()),
                            )
                            .await
                            .unwrap();
                        format!("{}\n", id)
                    }
                    "resolve" => {
                        let sess = Session::dummy();
                        let catalog = catalog.for_session(&sess);

                        let parsed = mz_sql::parse::parse(&test_case.input).unwrap();
                        let pcx = &PlanContext::zero();
                        let scx = StatementContext::new(Some(pcx), &catalog);
                        let qcx =
                            QueryContext::root(&scx, QueryLifetime::OneShot(scx.pcx().unwrap()));
                        let q = parsed[0].clone();
                        let q = match q {
                            Statement::Select(s) => s.query,
                            _ => unreachable!(),
                        };
                        let resolved = names::resolve(qcx.scx.catalog, q);
                        match resolved {
                            Ok((q, _depends_on)) => format!("{}\n", q),
                            Err(e) => format!("error: {}\n", e),
                        }
                    }
                    dir => panic!("unhandled directive {}", dir),
                }
            }
        })
        .await;
        f
    })
    .await;
}
