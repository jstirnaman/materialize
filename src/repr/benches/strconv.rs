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

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use mz_repr::strconv;

fn bench_parse_float32(c: &mut Criterion) {
    for s in &["-3.0", "9.7", "NaN", "inFiNiTy"] {
        c.bench_with_input(BenchmarkId::new("parse_float32", s), s, |b, s| {
            b.iter(|| strconv::parse_float32(s).unwrap())
        });
    }
}

fn bench_parse_numeric(c: &mut Criterion) {
    for s in &["-135412353251", "1.030340E11"] {
        c.bench_with_input(BenchmarkId::new("parse_numeric", s), s, |b, s| {
            b.iter(|| strconv::parse_numeric(s).unwrap())
        });
    }
}

fn bench_parse_jsonb(c: &mut Criterion) {
    let input = include_str!("testdata/twitter.json");
    c.bench_function("parse_jsonb", |b| {
        b.iter(|| black_box(strconv::parse_jsonb(input).unwrap()))
    });
}

fn bench_format_list_simple(c: &mut Criterion) {
    let mut rng = StdRng::from_seed([0; 32]);
    let list: Vec<i32> = (0..(1 << 12)).map(|_| rng.gen()).collect();
    c.bench_function("format_list simple", |b| {
        b.iter(|| {
            let mut buf = String::new();
            strconv::format_list(&mut buf, black_box(&list), |lw, i| {
                Ok::<_, ()>(strconv::format_int32(lw.nonnull_buffer(), *i))
            })
            .unwrap()
        })
    });
}

fn bench_format_list_nested(c: &mut Criterion) {
    let mut rng = StdRng::from_seed([0; 32]);
    const STRINGS: &[&str] = &[
        "NULL",
        "Po1bcC3mQWeYrMh6XaAM3ibM9CDDOoZK",
        r#""Elementary, my dear Watson," said Sherlock."#,
        "14VyaJllwQiPHRO2aNBo7p3P4v8cTLVB",
    ];
    let list: Vec<Vec<Vec<String>>> = (0..8)
        .map(|_| {
            (0..rng.gen_range(0..16))
                .map(|_| {
                    (1..rng.gen_range(0..16))
                        .map(|_| STRINGS.choose(&mut rng).unwrap())
                        .map(|s| (*s).to_owned())
                        .collect()
                })
                .collect()
        })
        .collect();

    c.bench_function("format_list nested", |b| {
        b.iter(|| {
            let mut buf = String::new();
            strconv::format_list(&mut buf, black_box(&list), |lw, list| {
                strconv::format_list(lw.nonnull_buffer(), list, |lw, list| {
                    strconv::format_list(lw.nonnull_buffer(), list, |lw, s| {
                        Ok::<_, ()>(strconv::format_string(lw.nonnull_buffer(), s))
                    })
                })
            })
            .unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_format_list_simple,
    bench_format_list_nested,
    bench_parse_numeric,
    bench_parse_float32,
    bench_parse_jsonb
);
criterion_main!(benches);
