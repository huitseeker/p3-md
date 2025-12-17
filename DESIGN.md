# Design Document: p3-uni-stark-mt

## Overview

This crate provides a minimal extension to Plonky3's univariate STARK framework to support **two-phase proving** with auxiliary traces. It implements the flow:

```
Main trace → Commit → Sample challenges → Build auxiliary trace → Commit → Quotient → Open
```

## Design Principles

### 1. Minimalism

**Goal**: Smallest possible API surface that enables multi-trace proving.

**Implementation**:
- Single trait extension: `AuxTraceBuilder`
- Reuses Plonky3's `AirBuilder` and `ExtensionBuilder` traits
- No new abstraction layers (no buses, no interaction types)

**Trade-offs**:
- ✅ Easy to understand and adopt
- ✅ Minimal code to maintain (~800 lines)
- ❌ Less expressive than InteractionBuilder
- ❌ AIRs must implement LogUp logic directly

### 2. Upstream Compatibility

**Goal**: Depend only on unmodified Plonky3 crates.

**Implementation**:
- All dependencies point to `github.com/Plonky3/Plonky3`
- No forked or modified P3 crates
- Compatible with latest P3 main branch

**Benefits**:
- ✅ Users can mix with other P3 crates
- ✅ Automatically gets P3 upstream improvements
- ✅ Clear separation of concerns

### 3. Single-Phase Auxiliary

**Goal**: Support exactly one auxiliary trace phase.

**Rationale**:
- Most practical use cases need only one phase (LogUp, permutation checks)
- Multiple phases add significant complexity
- Users needing multi-phase can use OpenVM stark-backend

**Implementation**:
- `aux_width()`: number of auxiliary columns
- `num_challenges()`: challenges needed for that phase
- `build_aux_trace()`: builds all auxiliary columns at once

### 4. Explicit Extension Field

**Goal**: Make extension field relationships explicit in types.

**Implementation**:
```rust
pub trait AuxTraceBuilder<F: Field, EF: ExtensionField<F>> {
    fn build_aux_trace(&self, main: &RowMajorMatrix<F>, challenges: &[EF])
        -> RowMajorMatrix<EF>;
}
```

**Benefits**:
- ✅ Type system enforces correct field usage
- ✅ Clear that aux trace lives in extension field
- ✅ Prevents accidental base field usage

## Architecture

### Trait Hierarchy

```
BaseAir<F>
    ├─→ AuxTraceBuilder<F, EF>  [this crate]
    └─→ Air<AB: AirBuilder>     [from Plonky3]

AirBuilder
    ├─→ ExtensionBuilder
    └─→ AuxBuilder              [this crate]
```

**`AuxTraceBuilder`**: Defines auxiliary trace metadata and generation.
- Separate from `Air` trait (composition, not inheritance)
- Allows AIRs to opt-in to auxiliary traces

**`AuxBuilder`**: Extends constraint builders with aux trace access.
- Provides `aux()` method returning auxiliary columns
- Implemented by both `ProverFolder` and `VerifierFolder`

### Prover Flow

```rust
// Phase 1: Main Trace
let (main_commit, main_data) = pcs.commit(main_trace);
challenger.observe(main_commit);

// Phase 2: Auxiliary Trace
if air.aux_width() > 0 {
    // Sample challenges
    let challenges = (0..air.num_challenges())
        .map(|_| challenger.sample())
        .collect();

    // Build aux trace using challenges
    let aux_trace = air.build_aux_trace(&main_trace, &challenges);

    // Commit
    let (aux_commit, aux_data) = pcs.commit(aux_trace);
    challenger.observe(aux_commit);
}

// Phase 3: Quotient (same as standard uni-stark)
// ...
```

### Constraint Evaluation

**Prover**: Evaluates constraints on packed field elements over quotient domain.

```rust
impl<AB: AirBuilder + AuxBuilder> Air<AB> for MyAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let aux = builder.aux();  // Access auxiliary columns

        // Write constraints using both main and aux
    }
}
```

**Verifier**: Evaluates constraints on opened values at out-of-domain point ζ.

### Comparison to Alternatives

| Feature | This Crate | 0xMiden | han0110 | OpenVM |
|---------|-----------|---------|---------|--------|
| **Dependencies** | Upstream P3 only | Modified P3 | Upstream P3 | Upstream P3 |
| **Lines of Code** | ~800 | ~1,000 | ~2,500 | ~10,000+ |
| **Abstraction** | Direct aux access | Direct aux access | InteractionBuilder | RAP framework |
| **Multi-AIR** | No | No | Yes | Yes |
| **Multi-Phase** | No (1 aux phase) | No (1 aux phase) | No (1 LogUp phase) | Yes (arbitrary) |
| **Learning Curve** | Low | Low | Medium | High |

## Implementation Status

### ✅ Implemented

- Core trait system (`AuxTraceBuilder`, `AuxBuilder`)
- Prover flow with two phases
- Verifier flow
- Constraint folders (prover and verifier)
- Example AIR (Fibonacci with LogUp skeleton)
- Documentation

### ⚠️ Partially Implemented

- **Quotient computation**: Works but needs:
  - Symbolic constraint degree calculation
  - Parallel evaluation
  - Proper chunking logic

- **Verifier**: Structure is correct but needs:
  - Actual PCS verification (currently trusts opened values)
  - Proper ZK support (if needed)

### ❌ Not Implemented (Out of Scope)

- Multiple auxiliary phases
- Cross-AIR interactions
- InteractionBuilder abstraction
- GPU/hardware acceleration
- Preprocessing/transparent columns (P3 has this)

## Future Extensions

### Easy Additions

1. **Parallel quotient evaluation** (~50 lines)
   - Add rayon parallel iterator
   - Batch constraint evaluation

2. **Symbolic constraint degree** (~100 lines)
   - Port from Plonky3 uni-stark
   - Compute exact quotient degree

3. **Proper PCS verification** (~50 lines)
   - Call `pcs.verify()` in verifier
   - Handle proof structure properly

### Medium Extensions

1. **Multiple auxiliary phases** (~200 lines)
   - Change `aux_width()` → `aux_widths() -> Vec<usize>`
   - Loop over phases in prover/verifier
   - Backward compatible if done carefully

2. **Batched inversion in aux trace** (~100 lines)
   - Helper for LogUp implementations
   - Batch all denominators and invert once

### Major Extensions (New Crate Territory)

1. **InteractionBuilder abstraction**
   - Separate interaction specification from trace building
   - Requires keygen phase
   - See han0110/uni-stark-ext

2. **Full RAP framework**
   - Multiple phase protocols
   - Pluggable phase implementations
   - See OpenVM stark-backend

## Testing Strategy

### Unit Tests (TODO)

- `AuxTraceBuilder` with various aux widths
- Constraint folder accumulation
- Proof serialization

### Integration Tests (TODO)

- Full prove/verify cycle
- Fibonacci with dummy LogUp
- Boundary case: zero auxiliary width

### Comparison Tests (TODO)

- Generate same proof as 0xMiden implementation
- Verify interoperability

## Performance Considerations

### Current Bottlenecks

1. **Quotient evaluation**: Single-threaded
   - Fix: Add parallel iteration
   - Expected: 4-8x speedup on multi-core

2. **Auxiliary trace building**: User responsibility
   - Fix: Provide batched inversion helper
   - Expected: 2-3x speedup for LogUp

3. **PCS operations**: Inherited from Plonky3
   - No changes needed

### Memory Usage

- Main trace: `height * width * sizeof(F)`
- Aux trace: `height * aux_width * sizeof(EF)`
- Quotient: `height * quotient_degree * sizeof(EF)`

**Typical**: For 2^20 rows, 10 main columns, 2 aux columns, BabyBear/EF4:
- Main: 4 bytes * 2^20 * 10 ≈ 40 MB
- Aux: 16 bytes * 2^20 * 2 ≈ 32 MB
- Quotient: 16 bytes * 2^20 * 4 ≈ 64 MB
- **Total**: ~136 MB (reasonable)

## Contributing

See README.md for contribution guidelines.

Key areas for contribution:
- Complete verifier PCS verification
- Add parallel quotient evaluation
- Implement symbolic constraint degree
- Write comprehensive tests
- Add more examples (range check, permutation, etc.)
