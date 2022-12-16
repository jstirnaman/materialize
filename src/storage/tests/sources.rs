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

//! Basic unit tests for sources.

use std::collections::HashMap;

use mz_storage::source::testscript::ScriptCommand;
use mz_storage_client::types::sources::{encoding::SourceDataEncoding, SourceEnvelope};

mod setup;

#[test]
fn test_datadriven() {
    datadriven::walk("tests/datadriven", |f| {
        let mut sources: HashMap<String, (Vec<ScriptCommand>, SourceDataEncoding, SourceEnvelope)> =
            HashMap::new();

        // Note we unwrap and panic liberally here as we
        // expect tests to be properly written.
        f.run(move |tc| -> String {
            match tc.directive.as_str() {
                "register-source" => {
                    // we just use the serde json representations.
                    let source: serde_json::Value = serde_json::from_str(&tc.input).unwrap();
                    let source = source.as_object().unwrap();
                    sources.insert(
                        tc.args["name"][0].clone(),
                        (
                            serde_json::from_value(source["script"].clone()).unwrap(),
                            serde_json::from_value(source["encoding"].clone()).unwrap(),
                            serde_json::from_value(source["envelope"].clone()).unwrap(),
                        ),
                    );

                    "<empty>\n".to_string()
                }
                "run-source" => {
                    let (script, encoding, envelope) = sources[&tc.args["name"][0]].clone();

                    // We just use the `Debug` representation here.
                    // REWRITE=true makes this reasonable!
                    format!(
                        "{:#?}\n",
                        setup::run_script_source(
                            script,
                            encoding,
                            envelope,
                            tc.args["expected_len"][0].parse().unwrap(),
                        )
                        .unwrap()
                    )
                }
                _ => panic!("unknown directive {:?}", tc),
            }
        })
    });
}
