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

use std::error::Error;

use mz_adapter::catalog::Catalog;
use mz_ore::collections::CollectionExt;
use mz_ore::now::NOW_ZERO;
use mz_repr::ScalarType;
use mz_sql::plan::PlanContext;

#[tokio::test]
async fn test_parameter_type_inference() -> Result<(), Box<dyn Error>> {
    let test_cases = vec![
        (
            "SELECT $1, $2, $3",
            vec![ScalarType::String, ScalarType::String, ScalarType::String],
        ),
        (
            "VALUES($1, $2, $3)",
            vec![ScalarType::String, ScalarType::String, ScalarType::String],
        ),
        (
            "SELECT 1 GROUP BY $1, $2, $3",
            vec![ScalarType::String, ScalarType::String, ScalarType::String],
        ),
        (
            "SELECT 1 ORDER BY $1, $2, $3",
            vec![ScalarType::String, ScalarType::String, ScalarType::String],
        ),
        (
            "SELECT ($1), (((($2))))",
            vec![ScalarType::String, ScalarType::String],
        ),
        ("SELECT $1::pg_catalog.int4", vec![ScalarType::Int32]),
        ("SELECT 1 WHERE $1", vec![ScalarType::Bool]),
        ("SELECT 1 HAVING $1", vec![ScalarType::Bool]),
        (
            "SELECT 1 FROM (VALUES (1)) a JOIN (VALUES (1)) b ON $1",
            vec![ScalarType::Bool],
        ),
        (
            "SELECT CASE WHEN $1 THEN 1 ELSE 0 END",
            vec![ScalarType::Bool],
        ),
        (
            "SELECT CASE WHEN true THEN $1 ELSE $2 END",
            vec![ScalarType::String, ScalarType::String],
        ),
        (
            "SELECT CASE WHEN true THEN $1 ELSE 1 END",
            vec![ScalarType::Int32],
        ),
        ("SELECT pg_catalog.abs($1)", vec![ScalarType::Float64]),
        ("SELECT pg_catalog.ascii($1)", vec![ScalarType::String]),
        (
            "SELECT coalesce($1, $2, $3)",
            vec![ScalarType::String, ScalarType::String, ScalarType::String],
        ),
        ("SELECT coalesce($1, 1)", vec![ScalarType::Int32]),
        (
            "SELECT pg_catalog.substr($1, $2)",
            vec![ScalarType::String, ScalarType::Int64],
        ),
        (
            "SELECT pg_catalog.substring($1, $2)",
            vec![ScalarType::String, ScalarType::Int64],
        ),
        (
            "SELECT $1 LIKE $2",
            vec![ScalarType::String, ScalarType::String],
        ),
        ("SELECT NOT $1", vec![ScalarType::Bool]),
        ("SELECT $1 AND $2", vec![ScalarType::Bool, ScalarType::Bool]),
        ("SELECT $1 OR $2", vec![ScalarType::Bool, ScalarType::Bool]),
        ("SELECT +$1", vec![ScalarType::Float64]),
        ("SELECT $1 < 1", vec![ScalarType::Int32]),
        (
            "SELECT $1 < $2",
            vec![ScalarType::String, ScalarType::String],
        ),
        ("SELECT $1 + 1", vec![ScalarType::Int32]),
        (
            "SELECT $1 + 1.0",
            vec![ScalarType::Numeric { max_scale: None }],
        ),
        (
            "SELECT '1970-01-01 00:00:00'::pg_catalog.timestamp + $1",
            vec![ScalarType::Interval],
        ),
        (
            "SELECT $1 + '1970-01-01 00:00:00'::pg_catalog.timestamp",
            vec![ScalarType::Interval],
        ),
        (
            "SELECT $1::pg_catalog.int4, $1 + $2",
            vec![ScalarType::Int32, ScalarType::Int32],
        ),
        (
            "SELECT '[0, 1, 2]'::pg_catalog.jsonb - $1",
            vec![ScalarType::String],
        ),
    ];

    let catalog = Catalog::open_debug_memory(NOW_ZERO.clone()).await?;
    let catalog = catalog.for_system_session();
    for (sql, types) in test_cases {
        let stmt = mz_sql::parse::parse(sql)?.into_element();
        let (stmt, _) = mz_sql::names::resolve(&catalog, stmt)?;
        let desc = mz_sql::plan::describe(&PlanContext::zero(), &catalog, stmt, &[])?;
        assert_eq!(desc.param_types, types);
    }
    Ok(())
}
