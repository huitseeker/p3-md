# Comparison: p3-uni-stark-mt vs Alternatives

This document compares p3-uni-stark-mt to other multi-trace STARK implementations.

## Quick Reference

| Implementation | Repository | Main Use Case | Status |
|----------------|------------|---------------|--------|
| **p3-uni-stark-mt** (this) | New crate | Single-AIR, single-aux-phase | Prototype |
| **0xMiden/Plonky3** | 0xMiden/Plonky3 | Miden zkVM | Production |
| **han0110/uni-stark-ext** | han0110/p3-playground | Multi-AIR exploration | Experimental |
| **OpenVM/stark-backend** | openvm-org/stark-backend | OpenVM zkVM | Production |

## Detailed Comparison

### Dependencies

| Crate | Plonky3 Source | Modifications |
|-------|----------------|---------------|
| p3-uni-stark-mt | Upstream | ✅ None - uses standard P3 |
| 0xMiden | Fork (0xMiden/Plonky3) | ❌ Modified air, uni-stark, symbolic |
| han0110 | Upstream | ✅ None - uses standard P3 |
| OpenVM | Upstream | ✅ None - uses standard P3 |

**Winner**: p3-uni-stark-mt, han0110, OpenVM (tie)

### Complexity (Lines of Code)

| Component | p3-uni-stark-mt | 0xMiden | han0110 | OpenVM |
|-----------|-----------------|---------|---------|--------|
| **Trait system** | 80 | 47 | 150 | 300 |
| **Prover** | 250 | 300 | 400 | 600 |
| **Verifier** | 100 | 150 | 200 | 400 |
| **Folders** | 250 | 180 | 300 | 500 |
| **Interaction/RAP** | 0 | 0 | 600 | 1,500 |
| **Keygen** | 0 | 0 | 400 | 1,000 |
| **Other** | 120 | 300 | 450 | 6,000+ |
| **TOTAL** | **~800** | **~1,000** | **~2,500** | **~10,000+** |

**Winner**: p3-uni-stark-mt (simplest)

### Capabilities

#### Multi-AIR Support

| Feature | p3-uni-stark-mt | 0xMiden | han0110 | OpenVM |
|---------|-----------------|---------|---------|--------|
| Multiple AIRs in one proof | ❌ | ❌ | ✅ | ✅ |
| Cross-AIR interactions | ❌ | ❌ | ✅ (via buses) | ✅ (via buses) |
| Heterogeneous AIR widths | ❌ | ❌ | ✅ | ✅ |

**Winner**: han0110, OpenVM (tie)

#### Challenge Phases

| Feature | p3-uni-stark-mt | 0xMiden | han0110 | OpenVM |
|---------|-----------------|---------|---------|--------|
| Main trace phase | ✅ | ✅ | ✅ | ✅ |
| Single aux phase | ✅ | ✅ | ✅ | ✅ |
| Multiple aux phases | ❌ | ❌ | ❌ | ✅ |
| Arbitrary phase protocols | ❌ | ❌ | ❌ | ✅ |

**Winner**: OpenVM

#### Abstraction Level

| Feature | p3-uni-stark-mt | 0xMiden | han0110 | OpenVM |
|---------|-----------------|---------|---------|--------|
| Direct aux trace access | ✅ | ✅ | ❌ | ❌ |
| InteractionBuilder | ❌ | ❌ | ✅ | ✅ |
| Bus abstraction | ❌ | ❌ | ✅ | ✅ |
| RAP framework | ❌ | ❌ | ❌ | ✅ |
| Exposed values | ❌ | ❌ | ❌ | ✅ |

**Winner**: Depends on use case
- Simple: p3-uni-stark-mt, 0xMiden
- Complex: han0110, OpenVM

### Production Readiness

| Aspect | p3-uni-stark-mt | 0xMiden | han0110 | OpenVM |
|--------|-----------------|---------|---------|--------|
| **Tests** | ❌ TODO | ✅ 449 passing | ⚠️ Experimental | ✅ Comprehensive |
| **Documentation** | ✅ Good | ✅ Good | ⚠️ Limited | ✅ Excellent |
| **Used in production** | ❌ | ✅ Miden zkVM | ❌ | ✅ OpenVM zkVM |
| **Maintenance** | ❌ New | ✅ Active | ⚠️ Experimental | ✅ Active |
| **Security audit** | ❌ | ❓ | ❌ | ❓ |

**Winner**: 0xMiden, OpenVM (tie)

## Architectural Differences

### Trait Design

#### p3-uni-stark-mt (This Crate)

```rust
trait AuxTraceBuilder<F, EF> {
    fn aux_width(&self) -> usize;
    fn num_challenges(&self) -> usize;
    fn build_aux_trace(&self, main: &RowMajorMatrix<F>, challenges: &[EF])
        -> RowMajorMatrix<EF>;
}

// In constraints:
impl<AB: AuxBuilder> Air<AB> for MyAir {
    fn eval(&self, builder: &mut AB) {
        let aux = builder.aux(); // Direct access
    }
}
```

**Style**: Explicit, low-level, direct access

#### 0xMiden/Plonky3

```rust
trait BaseAirWithAuxTrace<F, EF> {
    fn aux_width(&self) -> usize;
    fn num_randomness(&self) -> usize;
}

// Uses PermutationAirBuilder
impl<AB: PermutationAirBuilder> Air<AB> for MyAir {
    fn eval(&self, builder: &mut AB) {
        let aux = builder.permutation(); // Direct access
    }
}
```

**Style**: Very similar to p3-uni-stark-mt, slightly different naming

#### han0110/uni-stark-ext

```rust
trait InteractionBuilder: AirBuilder {
    fn push_interaction(
        &mut self,
        bus_index: usize,
        fields: impl IntoIterator,
        count: impl Into<Self::Expr>,
        interaction_type: InteractionType,
    );
}

// In constraints:
impl<AB: InteractionBuilder> Air<AB> for MyAir {
    fn eval(&self, builder: &mut AB) {
        builder.push_send(bus, fields, count);
        builder.push_receive(bus, fields, count);
    }
}
```

**Style**: High-level, declarative, bus-based routing

#### OpenVM/stark-backend

```rust
trait Rap<AB: PermutationAirBuilder> {
    fn eval(&self, builder: &mut AB);
}

trait RapPhaseSeq<F, Challenge, Challenger> {
    fn partially_prove(...) -> (PartialProof, RapPhaseProverData);
    fn partially_verify(...) -> Result<(), Error>;
}

// In constraints:
LookupBus::new(0).lookup_key(builder, query, enabled);
PermutationCheckBus::new(1).send(builder, message, enabled);
```

**Style**: Very high-level, protocol-oriented, security-focused

### Prover Flow Comparison

#### p3-uni-stark-mt

```
1. Commit main trace
2. Sample challenges
3. Build aux trace (user's build_aux_trace method)
4. Commit aux trace
5. Compute quotient
6. Open
```

#### 0xMiden

```
1. Commit main trace
2. Sample challenges
3. Build aux trace (AIR's aux logic)
4. Commit aux trace
5. Compute quotient
6. Open
```

**Identical to p3-uni-stark-mt**

#### han0110

```
1. Commit all main traces
2. Sample β, γ
3. Collect interactions (two-pass AIR eval)
4. Build LogUp traces (one per AIR)
5. Commit LogUp traces
6. Sample α
7. Compute quotients (with LogUp constraints)
8. Open
```

**More complex**: Interaction collection phase, per-AIR LogUp traces

#### OpenVM

```
1. Commit cached main traces
2. Commit common main traces
3. Sample challenges
4. For each RAP phase:
   a. Partially prove phase
   b. Commit after-challenge traces
   c. Sample next challenges
5. Compute quotient
6. Open
```

**Most complex**: Multiple phases, cached vs common distinction, HAL

## Use Case Recommendations

### Use p3-uni-stark-mt when:

✅ You have a **single AIR** to prove
✅ You need **one auxiliary trace phase** (e.g., LogUp)
✅ You want to use **upstream Plonky3** without modifications
✅ You prefer **simplicity** over advanced features
✅ You're **prototyping** or learning

### Use 0xMiden/Plonky3 when:

✅ You're building **Miden zkVM**
✅ You need **production-ready** code
✅ You're okay with **maintaining a fork**
✅ Miden's specific needs align with yours

### Use han0110/uni-stark-ext when:

✅ You need **multiple AIRs** in one proof
✅ You want **cross-AIR interactions**
✅ You prefer **InteractionBuilder** abstraction
✅ You're okay with **experimental** status
✅ You want to explore **interaction patterns**

### Use OpenVM/stark-backend when:

✅ You need **multiple challenge phases**
✅ You're building a **complex zkVM**
✅ You need **GPU acceleration**
✅ You want a **general RAP framework**
✅ Security is paramount (**count_weight**, PoW)
✅ You need **production-grade** infrastructure

## Migration Paths

### From 0xMiden to p3-uni-stark-mt

**Difficulty**: Easy

1. Change `BaseAirWithAuxTrace` → `AuxTraceBuilder`
2. Rename `num_randomness()` → `num_challenges()`
3. Update imports to use upstream P3
4. Adjust `build_aux_trace` signature if needed

**Benefit**: Use upstream P3, simpler dependencies

### From p3-uni-stark-mt to han0110

**Difficulty**: Medium

1. Implement `InteractionBuilder` for your AIR
2. Convert aux trace logic to `push_send`/`push_receive` calls
3. Add keygen phase to precompute interaction metadata
4. Update constraint evaluation to use interaction chunks

**Benefit**: Multi-AIR support, better abstractions

### From han0110 to OpenVM

**Difficulty**: Hard

1. Replace `Air` trait with `Rap` trait
2. Implement `RapPhaseSeq` for your challenge protocol
3. Refactor to partitioned main traces (cached/common)
4. Add security parameter validation
5. Integrate with HAL for GPU support

**Benefit**: Full generality, production features

### From p3-uni-stark-mt to OpenVM

**Difficulty**: Very Hard

Not recommended - too large a jump. Go through han0110 first.

## Performance Comparison

### Proving Time (Estimated)

For 2^20 rows, 10 main columns, 2 aux columns, BabyBear field:

| Implementation | Time | Notes |
|----------------|------|-------|
| p3-uni-stark-mt | ~3-5s | Baseline (single-threaded quotient) |
| 0xMiden | ~2-4s | Optimized (parallel quotient) |
| han0110 | ~3-5s | Baseline + interaction overhead |
| OpenVM (CPU) | ~2-4s | Highly optimized |
| OpenVM (GPU) | ~0.5-1s | GPU acceleration |

### Memory Usage

| Implementation | Main | Aux | Quotient | Overhead | Total |
|----------------|------|-----|----------|----------|-------|
| p3-uni-stark-mt | 40MB | 32MB | 64MB | ~10MB | ~146MB |
| 0xMiden | 40MB | 32MB | 64MB | ~10MB | ~146MB |
| han0110 | 40MB | 32MB | 64MB | ~20MB | ~156MB |
| OpenVM | 40MB | 32MB | 64MB | ~50MB | ~186MB |

**Note**: OpenVM's overhead includes keygen data, HAL, metrics.

## Conclusion

### TL;DR

- **Simplest**: p3-uni-stark-mt (800 lines)
- **Most production-ready**: 0xMiden, OpenVM (tie)
- **Best for multi-AIR**: han0110, OpenVM (tie)
- **Most general**: OpenVM
- **Best for learning**: p3-uni-stark-mt

### Recommendation Matrix

| Your Need | Recommended Choice |
|-----------|-------------------|
| Learn multi-trace STARKs | **p3-uni-stark-mt** |
| Contribute to Miden | **0xMiden/Plonky3** |
| Prototype multi-AIR ideas | **han0110/uni-stark-ext** |
| Build production zkVM | **OpenVM/stark-backend** |
| Simple single-AIR with LogUp | **p3-uni-stark-mt** or **0xMiden** |
| Complex multi-table zkVM | **OpenVM/stark-backend** |
