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

//! Types for the persist crate.

#![warn(missing_docs)]
#![warn(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use bytes::BufMut;

mod codec_impls;

/// Encoding and decoding operations for a type usable as a persisted key or
/// value.
pub trait Codec: Sized + 'static {
    /// Name of the codec.
    ///
    /// This name is stored for the key and value when a stream is first created
    /// and the same key and value codec must be used for that stream afterward.
    fn codec_name() -> String;
    /// Encode a key or value for permanent storage.
    ///
    /// This must perfectly round-trip Self through [Codec::decode]. If the
    /// encode function for this codec ever changes, decode must be able to
    /// handle bytes output by all previous versions of encode.
    fn encode<B>(&self, buf: &mut B)
    where
        B: BufMut;
    /// Decode a key or value previous encoded with this codec's
    /// [Codec::encode].
    ///
    /// This must perfectly round-trip Self through [Codec::encode]. If the
    /// encode function for this codec ever changes, decode must be able to
    /// handle bytes output by all previous versions of encode.
    ///
    /// It should also gracefully handle data encoded by future versions of
    /// encode (likely with an error).
    //
    // TODO: Mechanically, this could return a ref to the original bytes
    // without any copies, see if we can make the types work out for that.
    fn decode<'a>(buf: &'a [u8]) -> Result<Self, String>;
}

/// Encoding and decoding operations for a type usable as a persisted timestamp
/// or diff.
pub trait Codec64: Sized + 'static {
    /// Name of the codec.
    ///
    /// This name is stored for the timestamp and diff when a stream is first
    /// created and the same timestamp and diff codec must be used for that
    /// stream afterward.
    fn codec_name() -> String;

    /// Encode a timestamp or diff for permanent storage.
    ///
    /// This must perfectly round-trip Self through [Codec64::decode]. If the
    /// encode function for this codec ever changes, decode must be able to
    /// handle bytes output by all previous versions of encode.
    fn encode(&self) -> [u8; 8];

    /// Decode a timestamp or diff previous encoded with this codec's
    /// [Codec64::encode].
    ///
    /// This must perfectly round-trip Self through [Codec64::encode]. If the
    /// encode function for this codec ever changes, decode must be able to
    /// handle bytes output by all previous versions of encode.
    fn decode(buf: [u8; 8]) -> Self;
}

/// An opaque fencing token used in compare_and_downgrade_since.
pub trait Opaque: PartialEq + Clone + Sized + 'static {
    /// The value of the opaque token when no compare_and_downgrade_since calls
    /// have yet been successful.
    fn initial() -> Self;
}
