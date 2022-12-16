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

mod test {
    use mz_expr::canonicalize::{canonicalize_equivalences, canonicalize_predicates};
    use mz_expr::{MapFilterProject, MirScalarExpr};
    use mz_expr_test_util::*;
    use mz_lowertest::{deserialize, deserialize_optional, tokenize, MzReflect};
    use mz_ore::result::ResultExt;
    use mz_ore::str::separated;
    use mz_repr::ColumnType;

    use serde::{Deserialize, Serialize};

    fn reduce(s: &str) -> Result<MirScalarExpr, String> {
        let mut input_stream = tokenize(s)?.into_iter();
        let mut ctx = MirScalarExprDeserializeContext::default();
        let mut scalar: MirScalarExpr = deserialize(&mut input_stream, "MirScalarExpr", &mut ctx)?;
        let typ: Vec<ColumnType> = deserialize(&mut input_stream, "Vec<ColumnType> ", &mut ctx)?;
        let before = scalar.typ(&typ);
        scalar.reduce(&typ);
        let after = scalar.typ(&typ);
        // Verify that `reduce` did not change the type of the scalar.
        if before.scalar_type != after.scalar_type {
            return Err(format!(
                "FAIL: Type of scalar has changed:\nbefore: {:?}\nafter: {:?}\n",
                before, after
            ));
        }
        Ok(scalar)
    }

    fn test_canonicalize_pred(s: &str) -> Result<Vec<MirScalarExpr>, String> {
        let mut input_stream = tokenize(s)?.into_iter();
        let mut ctx = MirScalarExprDeserializeContext::default();
        let input_predicates: Vec<MirScalarExpr> =
            deserialize(&mut input_stream, "Vec<MirScalarExpr>", &mut ctx)?;
        let typ: Vec<ColumnType> = deserialize(&mut input_stream, "Vec<ColumnType>", &mut ctx)?;
        // predicate canonicalization is meant to produce the same output regardless of the
        // order of the input predicates.
        let mut predicates1 = input_predicates.clone();
        canonicalize_predicates(&mut predicates1, &typ);
        let mut predicates2 = input_predicates.clone();
        predicates2.sort();
        canonicalize_predicates(&mut predicates2, &typ);
        let mut predicates3 = input_predicates;
        predicates3.sort();
        predicates3.reverse();
        canonicalize_predicates(&mut predicates3, &typ);
        if predicates1 != predicates2 || predicates1 != predicates3 {
            Err(format!(
                "predicate canonicalization resulted in unrealiable output: [{}] vs [{}] vs [{}]",
                separated(", ", predicates1.iter().map(|p| p.to_string())),
                separated(", ", predicates2.iter().map(|p| p.to_string())),
                separated(", ", predicates3.iter().map(|p| p.to_string())),
            ))
        } else {
            Ok(predicates1)
        }
    }

    #[derive(Deserialize, Serialize, MzReflect)]
    enum MFPTestCommand {
        Map(Vec<MirScalarExpr>),
        Filter(Vec<MirScalarExpr>),
        Project(Vec<usize>),
        Optimize,
    }

    /// Builds a [MapFilterProject] of a certain arity, then modifies it.
    /// The test syntax is `<input_arity> [<commands>]`
    /// The syntax for a command is `<name_of_command> [<args>]`
    fn test_mfp(s: &str) -> Result<MapFilterProject, String> {
        let mut input_stream = tokenize(s)?.into_iter();
        let mut ctx = MirScalarExprDeserializeContext::default();
        let input_arity = input_stream
            .next()
            .unwrap()
            .to_string()
            .parse::<usize>()
            .map_err_to_string()?;
        let mut mfp = MapFilterProject::new(input_arity);
        while let Some(command) = deserialize_optional::<MFPTestCommand, _, _>(
            &mut input_stream,
            "MFPTestCommand",
            &mut ctx,
        )? {
            match command {
                MFPTestCommand::Map(map) => mfp = mfp.map(map),
                MFPTestCommand::Filter(filter) => mfp = mfp.filter(filter),
                MFPTestCommand::Project(project) => mfp = mfp.project(project),
                MFPTestCommand::Optimize => mfp.optimize(),
            }
        }
        Ok(mfp)
    }

    fn test_canonicalize_equiv(s: &str) -> Result<Vec<Vec<MirScalarExpr>>, String> {
        let mut input_stream = tokenize(s)?.into_iter();
        let mut ctx = MirScalarExprDeserializeContext::default();
        let mut equivalences: Vec<Vec<MirScalarExpr>> =
            deserialize(&mut input_stream, "Vec<Vec<MirScalarExpr>>", &mut ctx)?;
        let input_type: Vec<ColumnType> =
            deserialize(&mut input_stream, "Vec<ColumnType>", &mut ctx)?;
        canonicalize_equivalences(&mut equivalences, std::iter::once(&input_type));
        Ok(equivalences)
    }

    #[test]
    fn run() {
        datadriven::walk("tests/testdata", |f| {
            f.run(move |s| -> String {
                match s.directive.as_str() {
                    // tests simplification of scalars
                    "reduce" => match reduce(&s.input) {
                        Ok(scalar) => {
                            format!("{}\n", scalar)
                        }
                        Err(err) => format!("error: {}\n", err),
                    },
                    "canonicalize" => match test_canonicalize_pred(&s.input) {
                        Ok(preds) => {
                            format!("{}\n", separated("\n", preds.iter().map(|p| p.to_string())))
                        }
                        Err(err) => format!("error: {}\n", err),
                    },
                    "mfp" => match test_mfp(&s.input) {
                        Ok(mfp) => {
                            let (map, filter, project) = mfp.as_map_filter_project();
                            format!(
                                "[{}]\n[{}]\n[{}]\n",
                                separated(" ", map.iter()),
                                separated(" ", filter.iter()),
                                separated(" ", project.iter())
                            )
                        }
                        Err(err) => format!("error: {}\n", err),
                    },
                    "canonicalize-join" => match test_canonicalize_equiv(&s.input) {
                        Ok(equivalences) => {
                            format!(
                                "{}\n",
                                separated(
                                    "\n",
                                    equivalences.iter().map(|e| format!(
                                        "[{}]",
                                        separated(" ", e.iter().map(|expr| format!("{}", expr)))
                                    ))
                                )
                            )
                        }
                        Err(err) => format!("error: {}\n", err),
                    },
                    _ => panic!("unknown directive: {}", s.directive),
                }
            })
        });
    }
}
