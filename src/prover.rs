//! Prover implementation for multi-trace STARK

use alloc::vec;
use alloc::vec::Vec;

use p3_air::{Air, BaseAir};
use p3_challenger::{CanObserve, CanSample, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{ExtensionField, Field, PackedField};
use p3_matrix::dense::RowMajorMatrix;
use p3_util::log2_strict_usize;
use tracing::{info_span, instrument};

use crate::{
    AuxBuilder, AuxTraceBuilder, Challenge, Challenger, MultiTraceAir, Proof, ProverFolder, Val,
};

/// Prove a computation using a multi-trace AIR.
///
/// # Arguments
/// - `config`: STARK configuration (PCS, challenger)
/// - `air`: The AIR defining the computation
/// - `main_trace`: The main execution trace
/// - `public_values`: Public input/output values
///
/// # Returns
/// A proof that can be verified with [`crate::verify`]
///
/// # Panics
/// - If trace dimensions don't match AIR width
/// - If auxiliary trace building fails
#[instrument(skip_all, fields(trace_height = main_trace.height()))]
pub fn prove<SC, A>(
    config: &SC,
    air: &A,
    main_trace: RowMajorMatrix<Val<SC>>,
    public_values: &[Val<SC>],
) -> Proof<SC>
where
    SC: crate::StarkConfig,
    SC::Val: PackedField,
    A: MultiTraceAir<Val<SC>, Challenge<SC>>
        + for<'a> Air<ProverFolder<'a, SC>>
        + for<'a> Air<crate::VerifierFolder<'a, SC>>,
{
    assert_eq!(
        main_trace.width(),
        air.width(),
        "Main trace width mismatch"
    );

    let pcs = config.pcs();
    let mut challenger = config.challenger();

    // Trace dimensions
    let height = main_trace.height();
    let log_degree = log2_strict_usize(height) as u8;
    let trace_domain = pcs.natural_domain_for_degree(height);

    // ==================== PHASE 1: Main Trace ====================
    info_span!("commit main trace").in_scope(|| {
        tracing::info!("Committing main trace (height={})", height);
    });

    let (main_commit, main_data) =
        info_span!("pcs_commit_main").in_scope(|| pcs.commit(vec![(trace_domain, main_trace)]));

    // Observe main trace commitment
    challenger.observe(main_commit.clone());
    challenger.observe_slice(public_values);

    // ==================== PHASE 2: Auxiliary Trace ====================
    let (aux_commit, aux_data, aux_trace) = if air.aux_width() > 0 {
        info_span!("auxiliary phase").in_scope(|| {
            // Sample challenges
            let num_challenges = air.num_challenges();
            let challenges: Vec<Challenge<SC>> = (0..num_challenges)
                .map(|_| challenger.sample())
                .collect();

            tracing::info!(
                "Sampled {} challenges for auxiliary trace",
                num_challenges
            );

            // Build auxiliary trace using challenges
            let aux_trace = air.build_aux_trace(
                &main_data.get_ldes()[0],
                &challenges,
            );

            assert_eq!(
                aux_trace.width(),
                air.aux_width(),
                "Auxiliary trace width mismatch"
            );
            assert_eq!(
                aux_trace.height(),
                height,
                "Auxiliary trace height mismatch"
            );

            tracing::info!(
                "Built auxiliary trace ({}x{})",
                aux_trace.height(),
                aux_trace.width()
            );

            // Commit auxiliary trace
            let (aux_commit, aux_data) = info_span!("pcs_commit_aux")
                .in_scope(|| pcs.commit(vec![(trace_domain, aux_trace.clone())]));

            // Observe auxiliary commitment
            challenger.observe(aux_commit.clone());

            (Some(aux_commit), Some(aux_data), Some(aux_trace))
        })
    } else {
        (None, None, None)
    };

    // ==================== PHASE 3: Quotient Polynomial ====================
    info_span!("quotient computation").in_scope(|| {
        tracing::info!("Computing quotient polynomial");
    });

    // Sample challenge for combining constraints
    let alpha: Challenge<SC> = challenger.sample();

    // Compute constraint polynomial degree
    // TODO: For now using a simple heuristic; should compute symbolically
    let constraint_degree = 2; // Most common case
    let quotient_degree = 1 << constraint_degree;

    // Create larger domain for quotient evaluation
    let quotient_domain = trace_domain.create_disjoint_domain(height * quotient_degree);

    // Get trace evaluations on quotient domain
    let main_on_quotient = pcs.get_evaluations_on_domain(&main_data, 0, quotient_domain);
    let aux_on_quotient = aux_data
        .as_ref()
        .map(|data| pcs.get_evaluations_on_domain(data, 0, quotient_domain));

    // Compute quotient values
    let quotient_values = compute_quotient_values(
        air,
        trace_domain,
        quotient_domain,
        &main_on_quotient,
        aux_on_quotient.as_ref(),
        alpha,
        public_values,
    );

    // Commit to quotient polynomial chunks
    let quotient_flat = RowMajorMatrix::new_col(quotient_values).flatten_to_base();
    let quotient_chunks = quotient_domain.split_evals(quotient_degree, quotient_flat);
    let quotient_chunk_domains = quotient_domain.split_domains(quotient_degree);

    let (quotient_commit_vec, quotient_data_vec): (Vec<_>, Vec<_>) = quotient_chunks
        .into_iter()
        .zip(quotient_chunk_domains)
        .map(|(chunk, dom)| pcs.commit(vec![(dom, chunk)]))
        .unzip();

    // Observe quotient commitments
    for commit in &quotient_commit_vec {
        challenger.observe(commit.clone());
    }

    // ==================== PHASE 4: Opening ====================
    info_span!("opening").in_scope(|| {
        tracing::info!("Computing opening proofs");
    });

    // Sample out-of-domain evaluation point
    let zeta: Challenge<SC> = challenger.sample();
    let zeta_next = trace_domain.next_point(zeta).expect("domain must support next_point");

    // Open all committed polynomials
    let mut opening_points = vec![
        (&main_data, vec![vec![zeta, zeta_next]]),
    ];

    if let Some(ref aux_data) = aux_data {
        opening_points.push((aux_data, vec![vec![zeta, zeta_next]]));
    }

    for data in &quotient_data_vec {
        opening_points.push((data, vec![vec![zeta]]));
    }

    let (opened_values, opening_proof) =
        pcs.open(opening_points, &mut challenger);

    // Extract opened values
    let mut values_iter = opened_values.into_iter();

    // Main trace openings
    let main_openings = values_iter.next().unwrap();
    let main_local = main_openings[0][0].clone();
    let main_next = main_openings[0][1].clone();

    // Auxiliary trace openings (if present)
    let (aux_local, aux_next) = if aux_data.is_some() {
        let aux_openings = values_iter.next().unwrap();
        (aux_openings[0][0].clone(), aux_openings[0][1].clone())
    } else {
        (vec![], vec![])
    };

    // Quotient chunk openings
    let quotient_chunks = values_iter
        .map(|vals| vals[0][0].clone())
        .collect();

    Proof {
        main_commit,
        aux_commit,
        quotient_commits: quotient_commit_vec,
        main_local,
        main_next,
        aux_local,
        aux_next,
        quotient_chunks,
        opening_proof,
        log_degree,
    }
}

/// Compute quotient polynomial values by evaluating constraints on the quotient domain.
#[instrument(skip_all)]
fn compute_quotient_values<SC, A, M>(
    air: &A,
    trace_domain: <SC::Pcs as Pcs<SC::Challenge, SC::Val>>::Domain,
    quotient_domain: <SC::Pcs as Pcs<SC::Challenge, SC::Val>>::Domain,
    main_on_quotient: &M,
    aux_on_quotient: Option<&M>,
    alpha: Challenge<SC>,
    public_values: &[Val<SC>],
) -> Vec<Challenge<SC>>
where
    SC: crate::StarkConfig,
    SC::Val: PackedField,
    A: MultiTraceAir<Val<SC>, Challenge<SC>> + for<'a> Air<ProverFolder<'a, SC>>,
    M: p3_matrix::Matrix<Val<SC>> + Sync,
{
    let quotient_size = quotient_domain.size();
    let width_main = main_on_quotient.width();
    let width_aux = aux_on_quotient.map(|m| m.width()).unwrap_or(0);

    // Compute selectors
    let selectors = trace_domain.selectors_on_coset(quotient_domain);

    // Evaluate constraints at each point in quotient domain
    // For simplicity, we'll do this in a single-threaded manner
    // TODO: Add parallel evaluation
    let mut quotient_values = Vec::with_capacity(quotient_size);

    // Compute alpha powers (one per constraint)
    // TODO: Get exact constraint count symbolically
    let max_constraints = 100; // Conservative upper bound
    let mut alpha_powers: Vec<Challenge<SC>> = alpha.powers().take(max_constraints).collect();
    alpha_powers.reverse();

    for i in 0..quotient_size {
        let is_first_row = selectors.is_first_row[i];
        let is_last_row = selectors.is_last_row[i];
        let is_transition = selectors.is_transition[i];
        let inv_vanishing = selectors.inv_vanishing[i];

        // Get local and next row values
        let main_local: Vec<_> = (0..width_main)
            .map(|col| main_on_quotient.get(i, col))
            .collect();
        let main_next_idx = (i + 1) % quotient_size;
        let main_next: Vec<_> = (0..width_main)
            .map(|col| main_on_quotient.get(main_next_idx, col))
            .collect();

        let main_view = p3_matrix::dense::RowMajorMatrix::new(
            [main_local, main_next].concat(),
            width_main,
        );

        let aux_view = if let Some(aux) = aux_on_quotient {
            let aux_local: Vec<_> = (0..width_aux)
                .map(|col| aux.get(i, col).into())
                .collect();
            let aux_next: Vec<_> = (0..width_aux)
                .map(|col| aux.get(main_next_idx, col).into())
                .collect();
            p3_matrix::dense::RowMajorMatrix::new(
                [aux_local, aux_next].concat(),
                width_aux,
            )
        } else {
            p3_matrix::dense::RowMajorMatrix::new(vec![], 0)
        };

        // Evaluate constraints
        let mut folder = ProverFolder {
            main: main_view.as_view(),
            aux: aux_view.as_view(),
            is_first_row,
            is_last_row,
            is_transition,
            alpha_powers: &alpha_powers,
            accumulator: Challenge::<SC>::ZERO,
            constraint_index: 0,
        };

        air.eval(&mut folder);

        // quotient(x) = constraints(x) / Z_H(x)
        let quotient_value = folder.accumulator * inv_vanishing;
        quotient_values.push(quotient_value);
    }

    quotient_values
}
