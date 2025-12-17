//! Configuration types for multi-trace STARK

use p3_challenger::FieldChallenger;
use p3_commit::Pcs;
use p3_field::{ExtensionField, Field};

/// Generic STARK configuration for multi-trace proving.
///
/// This is similar to Plonky3's StarkConfig but explicitly includes the extension field
/// as a generic parameter.
pub trait StarkConfig {
    /// Base field for the main trace
    type Val: Field;

    /// Extension field for auxiliary trace and challenges
    type Challenge: ExtensionField<Self::Val>;

    /// Polynomial commitment scheme
    type Pcs: Pcs<Self::Challenge, Self::Val>;

    /// Fiat-Shamir challenger
    type Challenger: FieldChallenger<Self::Val>
        + FieldChallenger<Self::Challenge>;

    /// Get the PCS instance
    fn pcs(&self) -> &Self::Pcs;

    /// Create a new challenger for Fiat-Shamir
    fn challenger(&self) -> Self::Challenger;
}

/// Helper type aliases
pub type Val<SC> = <SC as StarkConfig>::Val;
pub type Challenge<SC> = <SC as StarkConfig>::Challenge;
pub type Pcs<SC> = <SC as StarkConfig>::Pcs;
pub type Challenger<SC> = <SC as StarkConfig>::Challenger;
