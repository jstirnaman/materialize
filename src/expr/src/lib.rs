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

//! Core expression language.

#![warn(missing_debug_implementations)]

use std::collections::BTreeSet;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use mz_repr::GlobalId;

mod id;
mod linear;
mod relation;
mod scalar;

pub mod explain;
pub mod virtual_syntax;
pub mod visit;

pub use relation::canonicalize;

pub use id::{Id, LocalId, PartitionId, SourceInstanceId};
pub use id::{ProtoId, ProtoLocalId};
pub use linear::{
    memoize_expr,
    plan::{MfpPlan, SafeMfpPlan},
    util::{join_permutations, permutation_for_arrangement},
    MapFilterProject, ProtoMapFilterProject, ProtoMfpPlan, ProtoSafeMfpPlan,
};
pub use relation::func::{AggregateFunc, LagLeadType, TableFunc};
pub use relation::func::{AnalyzedRegex, CaptureGroupDesc};
pub use relation::join_input_mapper::JoinInputMapper;
pub use relation::{
    compare_columns, AggregateExpr, CollectionPlan, ColumnOrder, JoinImplementation,
    MirRelationExpr, ProtoAggregateExpr, RowSetFinishing, WindowFrame, WindowFrameBound,
    WindowFrameUnits, RECURSION_LIMIT,
};
pub use relation::{
    JoinInputCharacteristics, ProtoAggregateFunc, ProtoColumnOrder, ProtoRowSetFinishing,
    ProtoTableFunc,
};
pub use scalar::func::{self, BinaryFunc, UnaryFunc, UnmaterializableFunc, VariadicFunc};
pub use scalar::{like_pattern, EvalError, FilterCharacteristics, MirScalarExpr};
pub use scalar::{ProtoDomainLimit, ProtoEvalError, ProtoMirScalarExpr};

/// A [`MirRelationExpr`] that claims to have been optimized, e.g., by an
/// `transform::Optimizer`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct OptimizedMirRelationExpr(pub MirRelationExpr);

impl OptimizedMirRelationExpr {
    /// Declare that the input `expr` is optimized, without actually running it
    /// through an optimizer. This can be useful to mark as optimized literal
    /// `MirRelationExpr`s that are obviously optimal, without invoking the whole
    /// machinery of the optimizer.
    pub fn declare_optimized(expr: MirRelationExpr) -> OptimizedMirRelationExpr {
        OptimizedMirRelationExpr(expr)
    }

    /// Get mutable access to the inner [MirRelationExpr]
    ///
    /// Callers of this method need to ensure that the underlying expression stays optimized after
    /// any mutations are applied
    pub fn as_inner(&self) -> &MirRelationExpr {
        &self.0
    }

    /// Get mutable access to the inner [MirRelationExpr]
    ///
    /// Callers of this method need to ensure that the underlying expression stays optimized after
    /// any mutations are applied
    pub fn as_inner_mut(&mut self) -> &mut MirRelationExpr {
        &mut self.0
    }

    pub fn into_inner(self) -> MirRelationExpr {
        self.0
    }
}

impl Deref for OptimizedMirRelationExpr {
    type Target = MirRelationExpr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CollectionPlan for OptimizedMirRelationExpr {
    fn depends_on_into(&self, out: &mut BTreeSet<GlobalId>) {
        self.0.depends_on_into(out)
    }
}
