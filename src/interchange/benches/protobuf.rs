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

use criterion::{black_box, Criterion, Throughput};
use prost::Message;

use mz_interchange::protobuf::{DecodedDescriptors, Decoder};
use mz_ore::cast::CastFrom;

use self::gen::benchmark::{Connector, Record, Value};

mod gen {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

pub fn bench_protobuf(c: &mut Criterion) {
    let value = Value {
        l_orderkey: 155_190,
        l_suppkey: 7706,
        l_linenumber: 1,
        l_quantity: 17.0,
        l_extendedprice: 21168.23,
        l_discount: 0.04,
        l_tax: 0.02,
        l_returnflag: "N".into(),
        l_linestatus: "O".into(),
        l_shipdate: 9567,
        l_commitdate: 9537,
        l_receiptdate: 9537,
        l_shipinstruct: "DELIVER IN PERSON".into(),
        l_shipmode: "TRUCK".into(),
        l_comment: "egular courts above the".into(),
        ..Default::default()
    };

    let connector = Connector {
        version: "0.9.5.Final".into(),
        connector: "mysql".into(),
        name: "tcph".into(),
        server_id: 0,
        ts_sec: 0,
        gtid: "".into(),
        file: "binlog.000004".into(),
        pos: 951_896_181,
        row: 0,
        snapshot: true,
        thread: 0,
        db: "tcph".into(),
        table: "lineitem".into(),
        query: "".into(),
    };

    let record = Record {
        tcph_tcph_lineitem_value: Some(value),
        source: Some(connector),
        op: "c".into(),
        ts_ms: 1_560_886_948_093,
    };

    let buf = record.encode_to_vec();
    let len = u64::cast_from(buf.len());
    let mut decoder = Decoder::new(
        DecodedDescriptors::from_bytes(
            &include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.pb"))[..],
            ".benchmark.Record".to_string(),
        )
        .unwrap(),
        false,
    )
    .unwrap();

    let mut bg = c.benchmark_group("protobuf");
    bg.throughput(Throughput::Bytes(len));
    bg.bench_function("decode", move |b| {
        b.iter(|| black_box(decoder.decode(&buf).unwrap()))
    });
    bg.finish();
}
