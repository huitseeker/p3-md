//! Proof structures

use alloc::vec::Vec;

/// A multi-trace STARK proof.
#[derive(Clone, Debug)]
pub struct Proof<SC: crate::StarkConfig> {
    /// Commitment to the main trace
    pub main_commit: <SC::Pcs as p3_commit::Pcs<SC::Challenge, SC::Val>>::Commitment,

    /// Commitment to the auxiliary trace (None if no aux trace)
    pub aux_commit: Option<<SC::Pcs as p3_commit::Pcs<SC::Challenge, SC::Val>>::Commitment>,

    /// Commitments to quotient polynomial chunks
    pub quotient_commits: Vec<<SC::Pcs as p3_commit::Pcs<SC::Challenge, SC::Val>>::Commitment>,

    /// Opened values of main trace at ζ (out-of-domain point)
    pub main_local: Vec<SC::Challenge>,

    /// Opened values of main trace at ζ·g (next row)
    pub main_next: Vec<SC::Challenge>,

    /// Opened values of aux trace at ζ (if aux trace exists)
    pub aux_local: Vec<SC::Challenge>,

    /// Opened values of aux trace at ζ·g (if aux trace exists)
    pub aux_next: Vec<SC::Challenge>,

    /// Opened values of quotient chunks at ζ
    pub quotient_chunks: Vec<SC::Challenge>,

    /// PCS opening proof
    pub opening_proof: <SC::Pcs as p3_commit::Pcs<SC::Challenge, SC::Val>>::Proof,

    /// Degree (log2 of trace height)
    pub log_degree: u8,
}
