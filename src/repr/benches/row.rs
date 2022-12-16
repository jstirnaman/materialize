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

use std::cmp::Ordering;

use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use mz_repr::adt::date::Date;
use mz_repr::{Datum, Row};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn bench_sort_datums(rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    b.iter_with_setup(|| rows.clone(), |mut rows| rows.sort())
}

fn bench_sort_row(rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    let rows = rows.into_iter().map(Row::pack).collect::<Vec<_>>();
    b.iter_with_setup(|| rows.clone(), |mut rows| rows.sort())
}

fn bench_sort_iter(rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    let rows = rows.into_iter().map(Row::pack).collect::<Vec<_>>();
    b.iter_with_setup(
        || rows.clone(),
        |mut rows| {
            rows.sort_by(move |a, b| {
                for (a, b) in a.iter().zip(b.iter()) {
                    match a.cmp(&b) {
                        Ordering::Equal => (),
                        non_equal => return non_equal,
                    }
                }
                Ordering::Equal
            });
        },
    )
}

fn bench_sort_unpack(rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    let rows = rows.into_iter().map(Row::pack).collect::<Vec<_>>();
    b.iter_with_setup(
        || rows.clone(),
        |mut rows| {
            #[allow(clippy::unnecessary_sort_by)]
            rows.sort_by(move |a, b| a.unpack().cmp(&b.unpack()));
        },
    )
}

fn bench_sort_unpacked(rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    let arity = rows[0].len();
    let rows = rows.into_iter().map(Row::pack).collect::<Vec<_>>();
    b.iter_with_setup(
        || rows.clone(),
        |rows| {
            let mut unpacked = vec![];
            for row in &rows {
                unpacked.extend(&**row);
            }
            let mut slices = unpacked.chunks(arity).collect::<Vec<_>>();
            slices.sort();
            slices.into_iter().map(Row::pack).collect::<Vec<_>>()
        },
    )
}

fn bench_filter_unpacked(filter: Datum, rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    let rows = rows.into_iter().map(Row::pack).collect::<Vec<_>>();
    b.iter_with_setup(
        || rows.clone(),
        |mut rows| rows.retain(|row| row.unpack()[0] == filter),
    )
}

fn bench_filter_packed(filter: Datum, rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    let filter = Row::pack_slice(&[filter]);
    let rows = rows.into_iter().map(Row::pack).collect::<Vec<_>>();
    b.iter_with_setup(
        || rows.clone(),
        |mut rows| rows.retain(|row| row.unpack()[0] == filter.unpack_first()),
    )
}

fn bench_pack_pack(rows: Vec<Vec<Datum>>, b: &mut Bencher) {
    b.iter(|| rows.iter().map(Row::pack).collect::<Vec<_>>())
}

fn seeded_rng() -> StdRng {
    SeedableRng::from_seed([
        224, 38, 155, 23, 190, 65, 147, 224, 136, 172, 167, 36, 125, 199, 232, 59, 191, 4, 243,
        175, 114, 47, 213, 46, 85, 226, 227, 35, 238, 119, 237, 21,
    ])
}

pub fn bench_sort(c: &mut Criterion) {
    let num_rows = 10_000;

    let mut rng = seeded_rng();
    let int_rows = (0..num_rows)
        .map(|_| {
            vec![
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
            ]
        })
        .collect::<Vec<_>>();
    let numeric_rows = (0..num_rows)
        .map(|_| {
            vec![
                Datum::Numeric(rng.gen::<i32>().into()),
                Datum::Numeric(rng.gen::<i32>().into()),
                Datum::Numeric(rng.gen::<i32>().into()),
                Datum::Numeric(rng.gen::<i32>().into()),
                Datum::Numeric(rng.gen::<i32>().into()),
                Datum::Numeric(rng.gen::<i32>().into()),
            ]
        })
        .collect::<Vec<_>>();

    let mut rng = seeded_rng();
    let byte_data = (0..num_rows)
        .map(|_| {
            let i: i32 = rng.gen();
            format!("{} and then {} and then {}", i, i + 1, i + 2).into_bytes()
        })
        .collect::<Vec<_>>();
    let byte_rows = byte_data
        .iter()
        .map(|bytes| vec![Datum::Bytes(bytes)])
        .collect::<Vec<_>>();

    c.bench_function("sort_datums_ints", |b| {
        bench_sort_datums(int_rows.clone(), b)
    });
    c.bench_function("sort_row_ints", |b| bench_sort_row(int_rows.clone(), b));
    c.bench_function("sort_iter_ints", |b| bench_sort_iter(int_rows.clone(), b));
    c.bench_function("sort_unpack_ints", |b| {
        bench_sort_unpack(int_rows.clone(), b)
    });
    c.bench_function("sort_unpacked_ints", |b| {
        bench_sort_unpacked(int_rows.clone(), b)
    });

    c.bench_function("sort_datums_numeric", |b| {
        bench_sort_datums(numeric_rows.clone(), b)
    });
    c.bench_function("sort_row_numeric", |b| {
        bench_sort_row(numeric_rows.clone(), b)
    });
    c.bench_function("sort_iter_numeric", |b| {
        bench_sort_iter(numeric_rows.clone(), b)
    });
    c.bench_function("sort_unpack_numeric", |b| {
        bench_sort_unpack(numeric_rows.clone(), b)
    });
    c.bench_function("sort_unpacked_numeric", |b| {
        bench_sort_unpacked(numeric_rows.clone(), b)
    });

    c.bench_function("sort_datums_bytes", |b| {
        bench_sort_datums(byte_rows.clone(), b)
    });
    c.bench_function("sort_row_bytes", |b| bench_sort_row(byte_rows.clone(), b));
    c.bench_function("sort_iter_bytes", |b| bench_sort_iter(byte_rows.clone(), b));
    c.bench_function("sort_unpack_bytes", |b| {
        bench_sort_unpack(byte_rows.clone(), b)
    });
    c.bench_function("sort_unpacked_bytes", |b| {
        bench_sort_unpacked(byte_rows.clone(), b)
    });
}

pub fn bench_pack(c: &mut Criterion) {
    let num_rows = 10_000;

    let mut rng = seeded_rng();
    let int_rows = (0..num_rows)
        .map(|_| {
            vec![
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
            ]
        })
        .collect::<Vec<_>>();

    let mut rng = seeded_rng();
    let byte_data = (0..num_rows)
        .map(|_| {
            let i: i32 = rng.gen();
            format!("{} and then {} and then {}", i, i + 1, i + 2).into_bytes()
        })
        .collect::<Vec<_>>();
    let byte_rows = byte_data
        .iter()
        .map(|bytes| vec![Datum::Bytes(bytes)])
        .collect::<Vec<_>>();

    c.bench_function("pack_pack_ints", |b| bench_pack_pack(int_rows.clone(), b));

    c.bench_function("pack_pack_bytes", |b| bench_pack_pack(byte_rows.clone(), b));
}

fn bench_filter(c: &mut Criterion) {
    let num_rows = 10_000;
    let mut rng = seeded_rng();
    let mut random_date =
        || Date::from_pg_epoch(rng.gen_range(Date::LOW_DAYS..=Date::HIGH_DAYS)).unwrap();
    let mut rng = seeded_rng();
    let date_rows = (0..num_rows)
        .map(|_| {
            vec![
                Datum::Date(random_date()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
                Datum::Int32(rng.gen()),
            ]
        })
        .collect::<Vec<_>>();
    let filter = random_date();

    c.bench_function("filter_unpacked", |b| {
        bench_filter_unpacked(Datum::Date(filter), date_rows.clone(), b)
    });
    c.bench_function("filter_packed", |b| {
        bench_filter_packed(Datum::Date(filter), date_rows.clone(), b)
    });
}

criterion_group!(benches, bench_sort, bench_pack, bench_filter);
criterion_main!(benches);
