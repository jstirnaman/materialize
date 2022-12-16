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

//! Utilities to build objects from the `repr` crate for unit testing.
//!
//! These test utilities are relied by crates other than `repr`.

use chrono::NaiveDateTime;
use mz_repr::adt::timestamp::CheckedTimestamp;
use proc_macro2::TokenTree;

use mz_lowertest::deserialize_optional_generic;
use mz_ore::str::StrExt;
use mz_repr::adt::numeric::Numeric;
use mz_repr::{Datum, Row, RowArena, ScalarType};

/* #endregion */

fn parse_litval<'a, F>(litval: &'a str, littyp: &str) -> Result<F, String>
where
    F: std::str::FromStr,
    F::Err: ToString,
{
    litval.parse::<F>().map_err(|e| {
        format!(
            "error when parsing {} into {}: {}",
            litval,
            littyp,
            e.to_string()
        )
    })
}

/// Constructs a `Row` from a sequence of `litval` and `littyp`.
///
/// See [get_scalar_type_or_default] for creating a `ScalarType`.
///
/// Generally, each `litval` can be parsed into a Datum in the manner you would
/// imagine. Exceptions:
/// * A Timestamp should be in the format `"\"%Y-%m-%d %H:%M:%S%.f\""` or
///   `"\"%Y-%m-%d %H:%M:%S\""`
///
/// Not all types are supported yet. Currently supported types:
/// * string, bool, timestamp
/// * all flavors of numeric types
pub fn test_spec_to_row<'a, I>(datum_iter: I) -> Result<Row, String>
where
    I: Iterator<Item = (&'a str, &'a ScalarType)>,
{
    let temp_storage = RowArena::new();
    Row::try_pack(datum_iter.map(|(litval, littyp)| {
        if litval == "null" {
            Ok(Datum::Null)
        } else {
            match littyp {
                ScalarType::Bool => Ok(Datum::from(parse_litval::<bool>(litval, "bool")?)),
                ScalarType::Numeric { .. } => {
                    Ok(Datum::from(parse_litval::<Numeric>(litval, "Numeric")?))
                }
                ScalarType::Int16 => Ok(Datum::from(parse_litval::<i16>(litval, "i16")?)),
                ScalarType::Int32 => Ok(Datum::from(parse_litval::<i32>(litval, "i32")?)),
                ScalarType::Int64 => Ok(Datum::from(parse_litval::<i64>(litval, "i64")?)),
                ScalarType::Float32 => Ok(Datum::from(parse_litval::<f32>(litval, "f32")?)),
                ScalarType::Float64 => Ok(Datum::from(parse_litval::<f64>(litval, "f64")?)),
                ScalarType::String => Ok(Datum::from(
                    temp_storage.push_string(mz_lowertest::unquote(litval)),
                )),
                ScalarType::Timestamp => {
                    let datetime = if litval.contains('.') {
                        NaiveDateTime::parse_from_str(litval, "\"%Y-%m-%d %H:%M:%S%.f\"")
                    } else {
                        NaiveDateTime::parse_from_str(litval, "\"%Y-%m-%d %H:%M:%S\"")
                    };
                    Ok(Datum::from(
                        CheckedTimestamp::from_timestamplike(
                            datetime
                                .map_err(|e| format!("Error while parsing NaiveDateTime: {}", e))?,
                        )
                        .unwrap(),
                    ))
                }
                _ => Err(format!("Unsupported literal type {:?}", littyp)),
            }
        }
    }))
}

/// Convert a Datum to a String such that [test_spec_to_row] can convert the
/// String back into a row containing the same Datum.
///
/// Currently supports only Datums supported by [test_spec_to_row].
pub fn datum_to_test_spec(datum: Datum) -> String {
    let result = format!("{}", datum);
    match datum {
        Datum::Timestamp(_) => result.quoted().to_string(),
        _ => result,
    }
}

/// Parses `ScalarType` from `scalar_type_stream` or infers it from `litval`
///
/// See [mz_lowertest::to_json] for the syntax for specifying a `ScalarType`.
/// If `scalar_type_stream` is empty, will attempt to guess a `ScalarType` for
/// the literal:
/// * If `litval` is "true", "false", or "null", will return `Bool`.
/// * Else if starts with `'"'`, will return String.
/// * Else if contains `'.'`, will return Float64.
/// * Otherwise, returns Int64.
pub fn get_scalar_type_or_default<I>(
    litval: &str,
    scalar_type_stream: &mut I,
) -> Result<ScalarType, String>
where
    I: Iterator<Item = TokenTree>,
{
    let typ: Option<ScalarType> = deserialize_optional_generic(scalar_type_stream, "ScalarType")?;
    match typ {
        Some(typ) => Ok(typ),
        None => {
            if ["true", "false", "null"].contains(&litval) {
                Ok(ScalarType::Bool)
            } else if litval.starts_with('\"') {
                Ok(ScalarType::String)
            } else if litval.contains('.') {
                Ok(ScalarType::Float64)
            } else {
                Ok(ScalarType::Int64)
            }
        }
    }
}

/// If the stream starts with a sequence of tokens that can be parsed as a datum,
/// return those tokens as one string.
///
/// Sequences of tokens that can be parsed as a datum:
/// * A Literal token, which is anything in quotations or a positive number
/// * An null, false, or true Ident token
/// * Punct(-) + a literal token
///
/// If the stream starts with a sequence of tokens that can be parsed as a
/// datum, 1) returns Ok(Some(..)) 2) advances the stream to the first token
/// that is not part of the sequence.
/// If the stream does not start with tokens that can be parsed as a datum:
/// * Return Ok(None) if `rest_of_stream` has not been advanced.
/// * Returns Err(..) otherwise.
pub fn extract_literal_string<I>(
    first_arg: &TokenTree,
    rest_of_stream: &mut I,
) -> Result<Option<String>, String>
where
    I: Iterator<Item = TokenTree>,
{
    match first_arg {
        TokenTree::Ident(ident) => {
            if ["true", "false", "null"].contains(&&ident.to_string()[..]) {
                Ok(Some(ident.to_string()))
            } else {
                Ok(None)
            }
        }
        TokenTree::Literal(literal) => Ok(Some(literal.to_string())),
        TokenTree::Punct(punct) if punct.as_char() == '-' => {
            match rest_of_stream.next() {
                Some(TokenTree::Literal(literal)) => {
                    Ok(Some(format!("{}{}", punct.as_char(), literal)))
                }
                None => Ok(None),
                // Must error instead of handling the tokens using default
                // behavior since `stream_iter` has advanced.
                Some(other) => Err(format!(
                    "{}{:?} is not a valid literal",
                    punct.as_char(),
                    other
                )),
            }
        }
        _ => Ok(None),
    }
}

/// Parse a token as a vec of strings that can be parsed as datums in a row.
///
/// The token is assumed to be of the form `[datum1 datum2 .. datumn]`.
pub fn parse_vec_of_literals(token: &TokenTree) -> Result<Vec<String>, String> {
    match token {
        TokenTree::Group(group) => {
            let mut inner_iter = group.stream().into_iter();
            let mut result = Vec::new();
            while let Some(symbol) = inner_iter.next() {
                match extract_literal_string(&symbol, &mut inner_iter)? {
                    Some(dat) => result.push(dat),
                    None => {
                        return Err(format!("{:?} cannot be interpreted as a literal.", symbol));
                    }
                }
            }
            Ok(result)
        }
        invalid => Err(format!(
            "{:?} cannot be parsed as a vec of literals",
            invalid
        )),
    }
}
