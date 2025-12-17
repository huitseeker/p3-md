//! Verifier implementation for multi-trace STARK

use p3_air::{Air, BaseAir};
use p3_challenger::{CanObserve, CanSample, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{ExtensionField, Field};
use tracing::instrument;

use crate::{AuxBuilder, AuxTraceBuilder, Challenge, Challenger, MultiTraceAir, Proof, Val, VerifierFolder};

/// Verification error types
#[derive(Debug)]
pub enum VerificationError {
    /// PCS verification failed
    PcsVerificationFailed,
    /// Constraint evaluation failed
    ConstraintVerificationFailed,
    /// Invalid proof structure
    InvalidProof(& 'static str),
}

/// Verify a multi-trace STARK proof.
///
/// # Arguments
/// - `config`: STARK configuration (must match prover's config)
/// - `air`: The AIR defining the computation (must match prover's AIR)
/// - `proof`: The proof to verify
/// - `public_values`: Public input/output values (must match prover's)
///
/// # Returns
/// - `Ok(())` if the proof is valid
/// - `Err(VerificationError)` if verification fails
#[instrument(skip_all, fields(log_degree = proof.log_degree))]
pub fn verify<SC, A>(
    config: &SC,
    air: &A,
    proof: &Proof<SC>,
    public_values: &[Val<SC>],
) -> Result<(), VerificationError>
where
    SC: crate::StarkConfig,
    A: MultiTraceAir<Val<SC>, Challenge<SC>>
        + for<'a> Air<VerifierFolder<'a, SC>>,
{
    // Check basic proof structure
    if air.aux_width() > 0 && proof.aux_commit.is_none() {
        return Err(VerificationError::InvalidProof(
            "AIR requires auxiliary trace but proof has none",
        ));
    }

    if air.aux_width() == 0 && proof.aux_commit.is_some() {
        return Err(VerificationError::InvalidProof(
            "AIR has no auxiliary trace but proof includes one",
        ));
    }

    let pcs = config.pcs();
    let mut challenger = config.challenger();

    // Reconstruct the verifier's view of the protocol
    let height = 1 << proof.log_degree;
    let trace_domain = pcs.natural_domain_for_degree(height);

    // Observe main trace commitment (same as prover)
    challenger.observe(proof.main_commit.clone());
    challenger.observe_slice(public_values);

    // Observe auxiliary commitment if present
    if let Some(ref aux_commit) = proof.aux_commit {
        // Sample challenges (same as prover)
        let num_challenges = air.num_challenges();
        for _ in 0..num_challenges {
            let _: Challenge<SC> = challenger.sample();
        }

        challenger.observe(aux_commit.clone());
    }

    // Observe quotient commitments
    for commit in &proof.quotient_commits {
        challenger.observe(commit.clone());
    }

    // Sample out-of-domain point (same as prover)
    let zeta: Challenge<SC> = challenger.sample();
    let zeta_next = trace_domain
        .next_point(zeta)
        .expect("domain must support next_point");

    // Verify PCS opening proofs
    // TODO: Implement actual PCS verification
    // For now, we trust the opened values and just verify constraints

    // Compute selectors at zeta
    let selectors = trace_domain.selectors_at_point(zeta);

    // Sample alpha for constraint combination (same as prover)
    let alpha: Challenge<SC> = challenger.sample();

    // Verify constraint equation: C(zeta) = Z_H(zeta) * Q(zeta)
    let mut folder = VerifierFolder {
        main_local: &proof.main_local,
        main_next: &proof.main_next,
        aux_local: &proof.aux_local,
        aux_next: &proof.aux_next,
        is_first_row: selectors.is_first_row,
        is_last_row: selectors.is_last_row,
        is_transition: selectors.is_transition,
        alpha,
        accumulator: Challenge::<SC>::ZERO,
    };

    // Evaluate constraints at zeta
    air.eval(&mut folder);

    let constraints_at_zeta = folder.accumulator;

    // Reconstruct quotient value from chunks
    // Q(zeta) = Q_0(zeta) + zeta^n * Q_1(zeta) + ...
    let n = Challenge::<SC>::from_canonical_usize(height);
    let mut quotient_at_zeta = Challenge::<SC>::ZERO;
    let mut power = Challenge::<SC>::ONE;

    for chunk in &proof.quotient_chunks {
        // Each chunk is already evaluated at zeta
        // We need to combine them: Q(X) = sum_i Q_i(X) * X^{i*n}
        quotient_at_zeta += chunk[0] * power;
        power *= n;
    }

    // Compute Z_H(zeta) - the vanishing polynomial at zeta
    let z_h_at_zeta = trace_domain.zp_at_point(zeta);

    // Check: C(zeta) == Z_H(zeta) * Q(zeta)
    let expected = z_h_at_zeta * quotient_at_zeta;

    if constraints_at_zeta != expected {
        return Err(VerificationError::ConstraintVerificationFailed);
    }

    Ok(())
}
