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

use std::iter::{repeat, repeat_with};
use std::str::FromStr;

use criterion::{criterion_group, criterion_main, Criterion};
use once_cell::sync::Lazy;
use timely::progress::Antichain;
use tokio::runtime::Runtime;

use mz_ore::metrics::MetricsRegistry;
use mz_stash::{Append, Postgres, PostgresFactory, Stash, StashError};

pub static FACTORY: Lazy<PostgresFactory> =
    Lazy::new(|| PostgresFactory::new(&MetricsRegistry::new()));

fn init_bench() -> (Runtime, Postgres) {
    let runtime = Runtime::new().unwrap();
    let connstr = std::env::var("POSTGRES_URL").unwrap();
    let tls = mz_postgres_util::make_tls(
        &tokio_postgres::config::Config::from_str(&connstr)
            .expect("invalid postgres url for storage stash"),
    )
    .unwrap();
    runtime
        .block_on(Postgres::clear(&connstr, tls.clone()))
        .unwrap();
    let stash = runtime
        .block_on((*FACTORY).open(connstr, None, tls))
        .unwrap();
    (runtime, stash)
}

fn bench_update(c: &mut Criterion) {
    c.bench_function("update", |b| {
        b.iter(|| {
            let (runtime, mut stash) = init_bench();

            let orders = runtime
                .block_on(stash.collection::<String, String>("orders"))
                .unwrap();
            let mut ts = 1;
            runtime.block_on(async {
                let data = ("widgets1".into(), "1".into());
                stash.update(orders, data, ts, 1).await.unwrap();
                ts += 1;
            })
        })
    });
}

fn bench_update_many(c: &mut Criterion) {
    c.bench_function("update_many", |b| {
        let (runtime, mut stash) = init_bench();

        let orders = runtime
            .block_on(stash.collection::<String, String>("orders"))
            .unwrap();
        let mut ts = 1;
        b.iter(|| {
            runtime.block_on(async {
                let data = ("widgets2".into(), "1".into());
                stash
                    .update_many(orders, repeat((data, ts, 1)).take(10))
                    .await
                    .unwrap();
                ts += 1;
            })
        })
    });
}

fn bench_consolidation(c: &mut Criterion) {
    c.bench_function("consolidation", |b| {
        let (runtime, mut stash) = init_bench();

        let orders = runtime
            .block_on(stash.collection::<String, String>("orders"))
            .unwrap();
        let mut ts = 1;
        b.iter(|| {
            runtime.block_on(async {
                let data = ("widgets3".into(), "1".into());
                stash.update(orders, data.clone(), ts, 1).await.unwrap();
                stash.update(orders, data, ts + 1, -1).await.unwrap();
                let frontier = Antichain::from_elem(ts + 2);
                stash.seal(orders, frontier.borrow()).await.unwrap();
                stash.compact(orders, frontier.borrow()).await.unwrap();
                stash.consolidate(orders.id).await.unwrap();
                ts += 2;
            })
        })
    });
}

fn bench_consolidation_large(c: &mut Criterion) {
    c.bench_function("consolidation large", |b| {
        let (runtime, mut stash) = init_bench();

        let mut ts = 0;
        let (orders, kv) = runtime.block_on(async {
            let orders = stash.collection::<String, String>("orders").await.unwrap();

            // Prepopulate the database with 100k records
            let kv = ("widgets4".into(), "1".into());
            stash
                .update_many(
                    orders,
                    repeat_with(|| {
                        let update = (kv.clone(), ts, 1);
                        ts += 1;
                        update
                    })
                    .take(100_000),
                )
                .await
                .unwrap();
            let frontier = Antichain::from_elem(ts);
            stash.seal(orders, frontier.borrow()).await.unwrap();
            (orders, kv)
        });

        let mut compact_ts = 0;
        b.iter(|| {
            runtime.block_on(async {
                ts += 1;
                // add 10k records
                stash
                    .update_many(
                        orders,
                        repeat_with(|| {
                            let update = (kv.clone(), ts, 1);
                            ts += 1;
                            update
                        })
                        .take(10_000),
                    )
                    .await
                    .unwrap();
                let frontier = Antichain::from_elem(ts);
                stash.seal(orders, frontier.borrow()).await.unwrap();
                // compact + consolidate
                compact_ts += 10_000;
                let compact_frontier = Antichain::from_elem(compact_ts);
                stash
                    .compact(orders, compact_frontier.borrow())
                    .await
                    .unwrap();
                stash.consolidate(orders.id).await.unwrap();
            })
        })
    });
}

fn bench_append(c: &mut Criterion) {
    c.bench_function("append", |b| {
        let (runtime, mut stash) = init_bench();
        const MAX: i64 = 1000;

        let orders = runtime
            .block_on(async {
                let orders = stash.collection::<String, String>("orders").await?;
                let mut batch = orders.make_batch(&mut stash).await?;
                // Skip 0 so it can be added initially.
                for i in 1..MAX {
                    orders.append_to_batch(&mut batch, &i.to_string(), &format!("_{i}"), 1);
                }
                stash.append(&[batch]).await?;
                Result::<_, StashError>::Ok(orders)
            })
            .unwrap();
        let mut i = 0;
        b.iter(|| {
            runtime.block_on(async {
                let mut batch = orders.make_batch(&mut stash).await.unwrap();
                let j = i % MAX;
                let k = (i + 1) % MAX;
                // Add the current i which doesn't exist, delete the next i
                // which is known to exist.
                orders.append_to_batch(&mut batch, &j.to_string(), &format!("_{j}"), 1);
                orders.append_to_batch(&mut batch, &k.to_string(), &format!("_{k}"), -1);
                stash.append(&[batch]).await.unwrap();
                i += 1;
            })
        })
    });
}

criterion_group!(
    benches,
    bench_append,
    bench_update,
    bench_update_many,
    bench_consolidation,
    bench_consolidation_large
);
criterion_main!(benches);
