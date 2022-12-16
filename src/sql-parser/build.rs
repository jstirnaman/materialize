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
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use uncased::UncasedStr;

use mz_ore::codegen::CodegenBuf;

const AST_DEFS_MOD: &str = "src/ast/defs.rs";
const KEYWORDS_LIST: &str = "src/keywords.txt";

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").context("Cannot read OUT_DIR env var")?);

    // Generate keywords list and lookup table.
    {
        let file = fs::read_to_string(KEYWORDS_LIST)?;

        let keywords: Vec<_> = file
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .collect();

        // Enforce that the keywords file is kept sorted. This is purely
        // cosmetic, but it cuts down on diff noise and merge conflicts.
        if let Some([a, b]) = keywords.windows(2).find(|w| w[0] > w[1]) {
            bail!("keywords list is not sorted: {:?} precedes {:?}", a, b);
        }

        let mut buf = CodegenBuf::new();

        buf.writeln("#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]");
        buf.write_block("pub enum Keyword", |buf| {
            for kw in &keywords {
                buf.writeln(format!("{},", kw));
            }
        });

        buf.write_block("impl Keyword", |buf| {
            buf.write_block("pub fn as_str(&self) -> &'static str", |buf| {
                buf.write_block("match self", |buf| {
                    for kw in &keywords {
                        buf.writeln(format!("Keyword::{} => {:?},", kw, kw.to_uppercase()));
                    }
                });
            });
        });

        for kw in &keywords {
            buf.writeln(format!(
                "pub const {}: Keyword = Keyword::{};",
                kw.to_uppercase(),
                kw
            ));
        }

        let mut phf = phf_codegen::Map::new();
        for kw in &keywords {
            phf.entry(UncasedStr::new(kw), &format!("Keyword::{}", kw));
        }
        buf.writeln(format!(
            "static KEYWORDS: phf::Map<&'static UncasedStr, Keyword> = {};",
            phf.build()
        ));

        fs::write(out_dir.join("keywords.rs"), buf.into_string())?;
    }

    // Generate AST visitors.
    {
        let ir = mz_walkabout::load(AST_DEFS_MOD)?;
        let fold = mz_walkabout::gen_fold(&ir);
        let visit = mz_walkabout::gen_visit(&ir);
        let visit_mut = mz_walkabout::gen_visit_mut(&ir);
        fs::write(out_dir.join("fold.rs"), fold)?;
        fs::write(out_dir.join("visit.rs"), visit)?;
        fs::write(out_dir.join("visit_mut.rs"), visit_mut)?;
    }

    Ok(())
}
