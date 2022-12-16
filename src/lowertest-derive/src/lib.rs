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

//! Macros needed by the `mz_lowertest` crate.
//!
//! TODO: eliminate macros in favor of using `walkabout`?

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse, Data, DeriveInput, Fields};

/// Types defined outside of Materialize used to build test objects.
const EXTERNAL_TYPES: &[&str] = &["String", "FixedOffset", "Tz", "NaiveDateTime", "Regex"];
const SUPPORTED_ANGLE_TYPES: &[&str] = &["Vec", "Box", "Option"];

/// Macro generating an implementation for the trait MzReflect
#[proc_macro_derive(MzReflect, attributes(mzreflect))]
pub fn mzreflect_derive(input: TokenStream) -> TokenStream {
    // The intended trait implementation is
    // ```
    // impl MzReflect for #name {
    //    /// Adds the information required to create an object of this type
    //    /// to `enum_dict` if it is an enum and to `struct_dict` if it is a
    //    /// struct.
    //    fn add_to_reflected_type_info(
    //        rti: &mut mz_lowertest::ReflectedTypeInfo
    //    )
    //    {
    //       // if the object is an enum
    //       if rti.enum_dict.contains_key(#name) { return; }
    //       use std::collections::HashMap;
    //       let mut result = HashMap::new();
    //       // repeat line below for all variants
    //       result.insert(variant_name, (<field_names>, <field_types>));
    //       rti.enum_dist.insert(<enum_name>, result);
    //
    //       // if the object is a struct
    //       if rti.struct_dict.contains_key(#name) { return ; }
    //       rti.struct_dict.insert(#name, (<field_names>, <field_types>));
    //
    //       // for all object types, repeat line below for each field type
    //       // that should be recursively added to the reflected type info
    //       <field_type>::add_reflect_type_info(enum_dict, struct_dict);
    //    }
    // }
    // ```
    let ast: DeriveInput = parse(input).unwrap();

    let object_name = &ast.ident;
    let object_name_as_string = object_name.to_string();
    let mut referenced_types = Vec::new();
    let add_object_info = if let Data::Enum(enumdata) = &ast.data {
        let variants = enumdata
            .variants
            .iter()
            .map(|v| {
                let variant_name = v.ident.to_string();
                let (names, types_as_string, mut types_as_syn) = get_fields_names_types(&v.fields);
                referenced_types.append(&mut types_as_syn);
                quote! {
                    result.insert(#variant_name, (vec![#(#names),*], vec![#(#types_as_string),*]));
                }
            })
            .collect::<Vec<_>>();
        quote! {
            if rti.enum_dict.contains_key(#object_name_as_string) { return; }
            use std::collections::HashMap;
            let mut result = HashMap::new();
            #(#variants)*
            rti.enum_dict.insert(#object_name_as_string, result);
        }
    } else if let Data::Struct(structdata) = &ast.data {
        let (names, types_as_string, mut types_as_syn) = get_fields_names_types(&structdata.fields);
        referenced_types.append(&mut types_as_syn);
        quote! {
            if rti.struct_dict.contains_key(#object_name_as_string) { return; }
            rti.struct_dict.insert(#object_name_as_string,
                (vec![#(#names),*], vec![#(#types_as_string),*]));
        }
    } else {
        unreachable!("Not a struct or enum")
    };

    let referenced_types = referenced_types
        .into_iter()
        .flat_map(extract_reflected_type)
        .map(|typ| quote! { #typ::add_to_reflected_type_info(rti); })
        .collect::<Vec<_>>();

    let gen = quote! {
      impl mz_lowertest::MzReflect for #object_name {
        fn add_to_reflected_type_info(
            rti: &mut mz_lowertest::ReflectedTypeInfo
        )
        {
           #add_object_info
           #(#referenced_types)*
        }
      }
    };
    gen.into()
}

/* #region Helper methods */

/// Gets the names and the types of the fields of an enum variant or struct.
///
/// The result has three parts:
/// 1. The names of the fields. If the fields are unnamed, this is empty.
/// 2. The types of the fields as strings.
/// 3. The types of the fields as [syn::Type]
///
/// Fields with the attribute `#[mzreflect(ignore)]` are not returned.
fn get_fields_names_types(f: &syn::Fields) -> (Vec<String>, Vec<String>, Vec<&syn::Type>) {
    match f {
        Fields::Named(named_fields) => {
            let (names, types): (Vec<_>, Vec<_>) = named_fields
                .named
                .iter()
                .flat_map(get_field_name_type)
                .unzip();
            let (types_as_string, types_as_syn) = types.into_iter().unzip();
            (names, types_as_string, types_as_syn)
        }
        Fields::Unnamed(unnamed_fields) => {
            let (types_as_string, types_as_syn): (Vec<_>, Vec<_>) = unnamed_fields
                .unnamed
                .iter()
                .flat_map(get_field_name_type)
                .map(|(_, (type_as_string, type_as_syn))| (type_as_string, type_as_syn))
                .unzip();
            (Vec::new(), types_as_string, types_as_syn)
        }
        Fields::Unit => (Vec::new(), Vec::new(), Vec::new()),
    }
}

/// Gets the name and the type of a field of an enum variant or struct.
///
/// The result has three parts:
/// 1. The name of the field. If the field is unnamed, this is empty.
/// 2. The type of the field as a string.
/// 3. The type of the field as [syn::Type].
///
/// Returns None if the field has the attribute `#[mzreflect(ignore)]`.
fn get_field_name_type(f: &syn::Field) -> Option<(String, (String, &syn::Type))> {
    for attr in f.attrs.iter() {
        if let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() {
            if meta_list.path.segments.last().unwrap().ident == "mzreflect" {
                for nested_meta in meta_list.nested.iter() {
                    if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = nested_meta {
                        if path.segments.last().unwrap().ident == "ignore" {
                            return None;
                        }
                    }
                }
            }
        }
    }
    let name = if let Some(name) = f.ident.as_ref() {
        name.to_string()
    } else {
        "".to_string()
    };
    Some((name, (get_type_as_string(&f.ty), &f.ty)))
}

/// Gets the type name from the [`syn::Type`] object
fn get_type_as_string(t: &syn::Type) -> String {
    // convert type back into a token stream and then into a string
    let mut token_stream = proc_macro2::TokenStream::new();
    t.to_tokens(&mut token_stream);
    token_stream.to_string()
}

/// If `t` is a supported type, extracts from `t` types defined in a
/// Materialize package.
///
/// Returns an empty vector if `t` is of an unsupported type.
///
/// Supported types are:
/// A plain path type A -> extracts A
/// `Box<A>`, `Vec<A>`, `Option<A>` -> extracts A
/// Tuple (A, (B, C)) -> extracts A, B, C.
/// Remove A, B, C from expected results if they are primitive types or listed
/// in [EXTERNAL_TYPES].
fn extract_reflected_type(t: &syn::Type) -> Vec<&syn::Type> {
    match t {
        syn::Type::Group(tg) => {
            return extract_reflected_type(&tg.elem);
        }
        syn::Type::Path(tp) => {
            let last_segment = tp.path.segments.last().unwrap();
            let type_name = last_segment.ident.to_string();
            match &last_segment.arguments {
                syn::PathArguments::None => {
                    if EXTERNAL_TYPES.contains(&&type_name[..])
                        || type_name.starts_with(|c: char| c.is_lowercase())
                    {
                        // Ignore primitive types and types
                        return Vec::new();
                    } else {
                        return vec![t];
                    }
                }
                syn::PathArguments::AngleBracketed(args) => {
                    if SUPPORTED_ANGLE_TYPES.contains(&&type_name[..]) {
                        return args
                            .args
                            .iter()
                            .flat_map(|arg| {
                                if let syn::GenericArgument::Type(typ) = arg {
                                    extract_reflected_type(typ)
                                } else {
                                    Vec::new()
                                }
                            })
                            .collect::<Vec<_>>();
                    }
                }
                _ => {}
            }
        }
        syn::Type::Tuple(tt) => {
            return tt
                .elems
                .iter()
                .flat_map(extract_reflected_type)
                .collect::<Vec<_>>();
        }
        _ => {}
    }
    Vec::new()
}

/* #endregion */
