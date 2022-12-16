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
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use byteorder::{NetworkEndian, WriteBytesExt};
use criterion::{black_box, Criterion, Throughput};

use mz_avro::types::Value as AvroValue;
use mz_interchange::avro::{parse_schema, Decoder};
use mz_ore::cast::CastFrom;
use mz_repr::adt::date::Date;
use tokio::runtime::Runtime;

pub fn bench_avro(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    let schema_str = r#"
{
  "type": "record",
  "name": "Envelope",
  "namespace": "tpch.tpch.lineitem",
  "fields": [
    {
      "name": "before",
      "type": [
        "null",
        {
          "type": "record",
          "name": "Value",
          "fields": [
            {
              "name": "l_orderkey",
              "type": "int"
            },
            {
              "name": "l_partkey",
              "type": "int"
            },
            {
              "name": "l_suppkey",
              "type": "int"
            },
            {
              "name": "l_linenumber",
              "type": "int"
            },
            {
              "name": "l_quantity",
              "type": "double"
            },
            {
              "name": "l_extendedprice",
              "type": "double"
            },
            {
              "name": "l_discount",
              "type": "double"
            },
            {
              "name": "l_tax",
              "type": "double"
            },
            {
              "name": "l_returnflag",
              "type": "string"
            },
            {
              "name": "l_linestatus",
              "type": "string"
            },
            {
              "name": "l_shipdate",
              "type": {
                "type": "int",
                "connect.version": 1,
                "connect.name": "org.apache.kafka.connect.data.Date",
                "logicalType": "date"
              }
            },
            {
              "name": "l_commitdate",
              "type": {
                "type": "int",
                "connect.version": 1,
                "connect.name": "org.apache.kafka.connect.data.Date",
                "logicalType": "date"
              }
            },
            {
              "name": "l_receiptdate",
              "type": {
                "type": "int",
                "connect.version": 1,
                "connect.name": "org.apache.kafka.connect.data.Date",
                "logicalType": "date"
              }
            },
            {
              "name": "l_shipinstruct",
              "type": "string"
            },
            {
              "name": "l_shipmode",
              "type": "string"
            },
            {
              "name": "l_comment",
              "type": "string"
            }
          ],
          "connect.name": "tpch.tpch.lineitem.Value"
        }
      ],
      "default": null
    },
    {
      "name": "after",
      "type": [
        "null",
        "Value"
      ],
      "default": null
    },
    {
      "name": "source",
      "type": {
        "type": "record",
        "name": "Source",
        "namespace": "io.debezium.connector.mysql",
        "fields": [
          {
            "name": "version",
            "type": [
              "null",
              "string"
            ],
            "default": null
          },
          {
            "name": "connector",
            "type": [
              "null",
              "string"
            ],
            "default": null
          },
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "server_id",
            "type": "long"
          },
          {
            "name": "ts_sec",
            "type": "long"
          },
          {
            "name": "gtid",
            "type": [
              "null",
              "string"
            ],
            "default": null
          },
          {
            "name": "file",
            "type": "string"
          },
          {
            "name": "pos",
            "type": "long"
          },
          {
            "name": "row",
            "type": "int"
          },
          {
            "name": "snapshot",
            "type": [
              {
                "type": "boolean",
                "connect.default": false
              },
              "null"
            ],
            "default": false
          },
          {
            "name": "thread",
            "type": [
              "null",
              "long"
            ],
            "default": null
          },
          {
            "name": "db",
            "type": [
              "null",
              "string"
            ],
            "default": null
          },
          {
            "name": "table",
            "type": [
              "null",
              "string"
            ],
            "default": null
          },
          {
            "name": "query",
            "type": [
              "null",
              "string"
            ],
            "default": null
          }
        ],
        "connect.name": "io.debezium.connector.mysql.Source"
      }
    },
    {
      "name": "op",
      "type": "string"
    },
    {
      "name": "ts_ms",
      "type": [
        "null",
        "long"
      ],
      "default": null
    }
  ],
  "connect.name": "tpch.tpch.lineitem.Envelope"
}
"#;
    let schema = parse_schema(schema_str).unwrap();

    fn since_epoch(days: i64) -> i32 {
        Date::from_unix_epoch(days.try_into().unwrap())
            .unwrap()
            .unix_epoch_days()
    }
    let record = AvroValue::Record(vec![
        (
            "before".into(),
            AvroValue::Union {
                index: 0,
                inner: Box::new(AvroValue::Null),
                n_variants: 2,
                null_variant: Some(0),
            },
        ),
        (
            "after".into(),
            AvroValue::Union {
                index: 1,
                inner: Box::new(AvroValue::Record(vec![
                    ("l_orderkey".into(), AvroValue::Int(1)),
                    ("l_partkey".into(), AvroValue::Int(155_190)),
                    ("l_suppkey".into(), AvroValue::Int(7706)),
                    ("l_linenumber".into(), AvroValue::Int(1)),
                    ("l_quantity".into(), AvroValue::Double(17.0)),
                    ("l_extendedprice".into(), AvroValue::Double(21168.23)),
                    ("l_discount".into(), AvroValue::Double(0.04)),
                    ("l_tax".into(), AvroValue::Double(0.02)),
                    ("l_returnflag".into(), AvroValue::String("N".into())),
                    ("l_linestatus".into(), AvroValue::String("O".into())),
                    ("l_shipdate".into(), AvroValue::Date(since_epoch(9567))),
                    ("l_commitdate".into(), AvroValue::Date(since_epoch(9537))),
                    ("l_receiptdate".into(), AvroValue::Date(since_epoch(9567))),
                    (
                        "l_shipinstruct".into(),
                        AvroValue::String("DELIVER IN PERSON".into()),
                    ),
                    ("l_shipmode".into(), AvroValue::String("TRUCK".into())),
                    (
                        "l_comment".into(),
                        AvroValue::String("egular courts above the".into()),
                    ),
                ])),
                n_variants: 2,
                null_variant: Some(0),
            },
        ),
        (
            "source".into(),
            AvroValue::Record(vec![
                (
                    "version".into(),
                    AvroValue::Union {
                        index: 1,
                        inner: Box::new(AvroValue::String("0.9.5.Final".into())),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
                (
                    "connector".into(),
                    AvroValue::Union {
                        index: 1,
                        inner: Box::new(AvroValue::String("mysql".into())),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
                ("name".into(), AvroValue::String("tpch".into())),
                ("server_id".into(), AvroValue::Long(0)),
                ("ts_sec".into(), AvroValue::Long(0)),
                (
                    "gtid".into(),
                    AvroValue::Union {
                        index: 0,
                        inner: Box::new(AvroValue::Null),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
                ("file".into(), AvroValue::String("binlog.000004".into())),
                ("pos".into(), AvroValue::Long(951_896_181)),
                ("row".into(), AvroValue::Int(0)),
                (
                    "snapshot".into(),
                    AvroValue::Union {
                        index: 0,
                        inner: Box::new(AvroValue::Boolean(true)),
                        n_variants: 2,
                        null_variant: Some(1),
                    },
                ),
                (
                    "thread".into(),
                    AvroValue::Union {
                        index: 0,
                        inner: Box::new(AvroValue::Null),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
                (
                    "db".into(),
                    AvroValue::Union {
                        index: 1,
                        inner: Box::new(AvroValue::String("tpch".into())),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
                (
                    "table".into(),
                    AvroValue::Union {
                        index: 1,
                        inner: Box::new(AvroValue::String("lineitem".into())),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
                (
                    "query".into(),
                    AvroValue::Union {
                        index: 0,
                        inner: Box::new(AvroValue::Null),
                        n_variants: 2,
                        null_variant: Some(0),
                    },
                ),
            ]),
        ),
        ("op".into(), AvroValue::String("c".into())),
        (
            "ts_ms".into(),
            AvroValue::Union {
                index: 1,
                inner: Box::new(AvroValue::Long(1_560_886_948_093)),
                n_variants: 2,
                null_variant: Some(0),
            },
        ),
    ]);

    let mut buf = Vec::new();
    buf.write_u8(0).unwrap();
    buf.write_i32::<NetworkEndian>(0).unwrap();
    buf.extend(mz_avro::to_avro_datum(&schema, record).unwrap());
    let len = u64::cast_from(buf.len());

    let mut decoder =
        Decoder::<Box<mz_ccsr::Client>>::new(schema_str, None, "avro_bench".to_string(), false)
            .unwrap();

    let mut bg = c.benchmark_group("avro");
    bg.throughput(Throughput::Bytes(len));
    bg.bench_function("decode", move |b| {
        b.iter(|| {
            black_box(
                runtime
                    .block_on(decoder.decode(&mut buf.as_slice()))
                    .unwrap(),
            )
        })
    });
    bg.finish();
}
