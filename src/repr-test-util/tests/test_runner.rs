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

#[cfg(test)]
mod tests {
    use mz_lowertest::{deserialize_optional_generic, tokenize};
    use mz_ore::str::separated;
    use mz_repr::ScalarType;
    use mz_repr_test_util::*;

    fn build_datum(s: &str) -> Result<String, String> {
        // 1) Convert test spec to the row containing the datum.
        let mut stream_iter = tokenize(s)?.into_iter();
        let litval =
            extract_literal_string(&stream_iter.next().ok_or("Empty test")?, &mut stream_iter)?
                .unwrap();
        let scalar_type = get_scalar_type_or_default(&litval[..], &mut stream_iter)?;
        let row = test_spec_to_row(std::iter::once((&litval[..], &scalar_type)))?;
        // 2) It should be possible to unpack the row and then convert the datum
        // back to the test spec.
        let datum = row.unpack_first();
        let roundtrip_s = datum_to_test_spec(datum);
        if roundtrip_s != litval {
            Err(format!(
                "Round trip failed. Old spec: {}. New spec: {}.",
                litval, roundtrip_s
            ))
        } else {
            Ok(format!("{:?}", datum))
        }
    }

    fn build_row(s: &str) -> Result<String, String> {
        let mut stream_iter = tokenize(s)?.into_iter();
        let litvals = parse_vec_of_literals(
            &stream_iter
                .next()
                .ok_or_else(|| "Empty row spec".to_string())?,
        )?;
        let scalar_types: Option<Vec<ScalarType>> =
            deserialize_optional_generic(&mut stream_iter, "Vec<ScalarType>")?;
        let scalar_types = if let Some(scalar_types) = scalar_types {
            scalar_types
        } else {
            litvals
                .iter()
                .map(|l| get_scalar_type_or_default(l, &mut std::iter::empty()))
                .collect::<Result<Vec<_>, String>>()?
        };
        let row = test_spec_to_row(litvals.iter().map(|s| &s[..]).zip(scalar_types.iter()))?;
        let roundtrip_litvals = row
            .unpack()
            .into_iter()
            .map(datum_to_test_spec)
            .collect::<Vec<_>>();
        if roundtrip_litvals != litvals {
            Err(format!(
                "Round trip failed. Old spec: [{}]. New spec: [{}].",
                separated(" ", litvals),
                separated(" ", roundtrip_litvals)
            ))
        } else {
            Ok(format!(
                "{}",
                separated("\n", row.unpack().into_iter().map(|d| format!("{:?}", d)))
            ))
        }
    }

    fn build_scalar_type(s: &str) -> Result<ScalarType, String> {
        get_scalar_type_or_default("", &mut tokenize(s)?.into_iter())
    }

    #[test]
    fn run() {
        datadriven::walk("tests/testdata", |f| {
            f.run(move |s| -> String {
                match s.directive.as_str() {
                    "build-scalar-type" => match build_scalar_type(&s.input) {
                        Ok(scalar_type) => format!("{:?}\n", scalar_type),
                        Err(err) => format!("error: {}\n", err),
                    },
                    "build-datum" => match build_datum(&s.input) {
                        Ok(result) => format!("{}\n", result),
                        Err(err) => format!("error: {}\n", err),
                    },
                    "build-row" => match build_row(&s.input) {
                        Ok(result) => format!("{}\n", result),
                        Err(err) => format!("error: {}\n", err),
                    },
                    _ => panic!("unknown directive: {}", s.directive),
                }
            })
        });
    }
}
