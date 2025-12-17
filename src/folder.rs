//! Constraint folders for prover and verifier

use alloc::vec::Vec;

use p3_air::{AirBuilder, ExtensionBuilder};
use p3_field::{Algebra, ExtensionField, Field, PackedField};
use p3_matrix::dense::RowMajorMatrixView;

use crate::{Challenge, Val};

/// Builder for evaluating constraints during proving.
///
/// This folder accumulates constraints using random challenges, computing:
/// `C_0 + α·C_1 + α²·C_2 + ...`
pub struct ProverFolder<'a, SC: crate::StarkConfig>
where
    SC::Val: PackedField,
{
    /// Main trace values (local and next rows, packed)
    pub main: RowMajorMatrixView<'a, SC::Val>,

    /// Auxiliary trace values (local and next rows, packed)
    /// Empty if no auxiliary trace
    pub aux: RowMajorMatrixView<'a, SC::Challenge>,

    /// Selector: 1 on first row, 0 elsewhere
    pub is_first_row: SC::Val,

    /// Selector: 1 on last row, 0 elsewhere
    pub is_last_row: SC::Val,

    /// Selector: 1 on all rows except last, 0 on last
    pub is_transition: SC::Val,

    /// Powers of α for constraint randomization
    pub alpha_powers: &'a [SC::Challenge],

    /// Accumulated constraint value
    pub accumulator: SC::Challenge,

    /// Current constraint index
    pub constraint_index: usize,
}

impl<'a, SC> AirBuilder for ProverFolder<'a, SC>
where
    SC: crate::StarkConfig,
    SC::Val: PackedField,
{
    type F = Val<SC>;
    type Expr = SC::Val;
    type Var = SC::Val;
    type M = RowMajorMatrixView<'a, SC::Val>;

    fn main(&self) -> Self::M {
        self.main
    }

    fn is_first_row(&self) -> Self::Expr {
        self.is_first_row
    }

    fn is_last_row(&self) -> Self::Expr {
        self.is_last_row
    }

    fn is_transition_window(&self, size: usize) -> Self::Expr {
        assert_eq!(size, 2, "Only window size 2 is supported");
        self.is_transition
    }

    fn assert_zero<I: Into<Self::Expr>>(&mut self, x: I) {
        let x = x.into();
        let alpha = self.alpha_powers[self.constraint_index];
        self.accumulator += alpha * x;
        self.constraint_index += 1;
    }
}

impl<'a, SC> ExtensionBuilder for ProverFolder<'a, SC>
where
    SC: crate::StarkConfig,
    SC::Val: PackedField,
{
    type EF = SC::Challenge;
    type ExprEF = SC::Challenge;
    type VarEF = SC::Challenge;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        let x = x.into();
        let alpha = self.alpha_powers[self.constraint_index];
        self.accumulator += alpha * x;
        self.constraint_index += 1;
    }
}

/// Extension trait for accessing auxiliary trace in constraints.
pub trait AuxBuilder: ExtensionBuilder {
    /// Matrix type for auxiliary trace
    type MAux;

    /// Access the auxiliary trace columns
    fn aux(&self) -> Self::MAux;
}

impl<'a, SC> AuxBuilder for ProverFolder<'a, SC>
where
    SC: crate::StarkConfig,
    SC::Val: PackedField,
{
    type MAux = RowMajorMatrixView<'a, SC::Challenge>;

    fn aux(&self) -> Self::MAux {
        self.aux
    }
}

/// Builder for verifying constraints.
///
/// Similar to [`ProverFolder`] but operates on opened polynomial values rather than
/// full trace matrices.
pub struct VerifierFolder<'a, SC: crate::StarkConfig> {
    /// Main trace values (local row)
    pub main_local: &'a [SC::Challenge],

    /// Main trace values (next row)
    pub main_next: &'a [SC::Challenge],

    /// Auxiliary trace values (local row)
    pub aux_local: &'a [SC::Challenge],

    /// Auxiliary trace values (next row)
    pub aux_next: &'a [SC::Challenge],

    /// Selector: 1 on first row, 0 elsewhere
    pub is_first_row: SC::Challenge,

    /// Selector: 1 on last row, 0 elsewhere
    pub is_last_row: SC::Challenge,

    /// Selector: 1 on all rows except last, 0 on last
    pub is_transition: SC::Challenge,

    /// Randomness for combining constraints
    pub alpha: SC::Challenge,

    /// Accumulated constraint value
    pub accumulator: SC::Challenge,
}

/// Simple view for verifier (just vectors of challenges)
pub struct VerifierView<'a, EF> {
    local: &'a [EF],
    next: &'a [EF],
}

impl<'a, EF: ExtensionField<impl Field>> VerifierView<'a, EF> {
    pub fn new(local: &'a [EF], next: &'a [EF]) -> Self {
        Self { local, next }
    }

    pub fn get_local(&self, col: usize) -> EF {
        self.local[col]
    }

    pub fn get_next(&self, col: usize) -> EF {
        self.next[col]
    }
}

impl<'a, SC> AirBuilder for VerifierFolder<'a, SC>
where
    SC: crate::StarkConfig,
{
    type F = Val<SC>;
    type Expr = Challenge<SC>;
    type Var = Challenge<SC>;
    type M = VerifierView<'a, SC::Challenge>;

    fn main(&self) -> Self::M {
        VerifierView::new(self.main_local, self.main_next)
    }

    fn is_first_row(&self) -> Self::Expr {
        self.is_first_row
    }

    fn is_last_row(&self) -> Self::Expr {
        self.is_last_row
    }

    fn is_transition_window(&self, size: usize) -> Self::Expr {
        assert_eq!(size, 2, "Only window size 2 is supported");
        self.is_transition
    }

    fn assert_zero<I: Into<Self::Expr>>(&mut self, x: I) {
        self.accumulator = self.accumulator * self.alpha + x.into();
    }
}

impl<'a, SC> ExtensionBuilder for VerifierFolder<'a, SC>
where
    SC: crate::StarkConfig,
{
    type EF = SC::Challenge;
    type ExprEF = SC::Challenge;
    type VarEF = SC::Challenge;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        self.accumulator = self.accumulator * self.alpha + x.into();
    }
}

impl<'a, SC> AuxBuilder for VerifierFolder<'a, SC>
where
    SC: crate::StarkConfig,
{
    type MAux = VerifierView<'a, SC::Challenge>;

    fn aux(&self) -> Self::MAux {
        VerifierView::new(self.aux_local, self.aux_next)
    }
}
