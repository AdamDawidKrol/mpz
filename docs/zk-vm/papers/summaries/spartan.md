# Spartan: Efficient and General-Purpose zkSNARKs Without Trusted Setup

**Authors**: Srinath Setty
**Venue**: CRYPTO 2020 | **ePrint**: [2019/550](https://eprint.iacr.org/2019/550)

## Role in the VOLE-ZK Design Space

Spartan is the canonical sum-check-based zkSNARK for R1CS. It introduces SPARK, a compiler for efficient sparse polynomial evaluation, which is directly generalized by Lasso's lookup arguments. Spartan's approach to encoding R1CS as a low-degree polynomial and applying the sum-check protocol is foundational to the Lasso/Jolt zkVM design that is a key point of comparison for VOLE-based zkVMs.

## Key Ideas

### R1CS Encoding as a Degree-3 Polynomial

Given an R1CS instance (A, B, C, io, m, n), Spartan defines `F_io(x)` such that `F_io(x) = 0` for all `x in {0,1}^s` iff the R1CS instance is satisfiable. Using multilinear extensions (MLEs) of A, B, C, and the witness Z:

```
F_io(x) = (sum_y A~(x,y)*Z~(y)) * (sum_y B~(x,y)*Z~(y)) - (sum_y C~(x,y)*Z~(y))
```

To check that this polynomial vanishes on the Boolean hypercube, the verifier checks `Q_io(tau) = sum_x F_io(x) * eq~(tau, x) = 0` for a random `tau`, which is a degree-3 polynomial suitable for the sum-check protocol.

### Two Nested Sum-Check Instances

The protocol uses two rounds of sum-check:

1. **Sum-check #1**: Reduces the claim `sum_x G_{io,tau}(x) = 0` to evaluating `G_{io,tau}(r_x)` at a random point. The prover sends claimed values `v_A, v_B, v_C` and the verifier checks `G_{io,tau}(r_x) = (v_A * v_B - v_C) * eq~(r_x, tau)`.

2. **Sum-check #2**: Combines the three claims `A(r_x) = v_A`, `B(r_x) = v_B`, `C(r_x) = v_C` into a single check via random linear combination, then runs a second sum-check. At the end, the verifier needs to evaluate `Z~(r_y)`, which is handled by a polynomial commitment to the witness.

The polynomial decomposition into multilinear polynomials is critical: it enables a **time-optimal prover** using Thaler's linear-time sum-check technique.

### Computation Commitments (Preprocessing)

The verifier must evaluate the MLEs of A, B, C at random points, which costs O(n). Spartan introduces **computation commitments**: in a public preprocessing step, the verifier commits to the sparse MLEs of A~, B~, C~ using a polynomial commitment scheme. During verification, the prover evaluates these polynomials and proves consistency with the commitments. This is a transparent (no-trusted-setup) analogue of GGPR's preprocessing.

### SPARK: Sparse Polynomial Commitment

The key technical contribution. Existing polynomial commitment schemes for multilinear polynomials have prover cost O(2^mu) where mu is the number of variables. For sparse polynomials (like the MLEs of R1CS matrices), the dense representation has only n << 2^mu nonzero entries, but a naive commitment scheme pays quadratic cost.

SPARK transforms any polynomial commitment scheme for dense multilinear polynomials into one that efficiently handles sparse multilinear polynomials:

1. Decompose the evaluation `M~(r_x, r_y) = sum_{(i,j): M(i,j) != 0} M(i,j) * eq~(i, r_x) * eq~(j, r_y)`
2. Precompute tables `eq~(i, r_x)` and `eq~(j, r_y)` for all i, j in {0,1}^s in O(m) time
3. Use **offline memory checking** to verify that the table lookups are correct — the circuit checks that each read returns the value that was last written to that address, using timestamp-based multiset equality checks

The memory checking uses public-coin multiset hash functions: `h_gamma(a, v, t) = a*gamma^2 + v*gamma + t` and `H_gamma(M) = prod_{e in M} (e - gamma)`, verified via `H(Init) * H(WS) = H(RS) * H(Audit)`.

This yields an O(n)-sized circuit for sparse polynomial evaluation, giving the prover O(n) cost instead of O(n log n) or O(m^2).

### Performance Summary

| Variant | Setup | Prover | Proof Size | Verifier |
|---------|-------|--------|------------|----------|
| Spartan-DL | public | O(n) | O(sqrt(n)) | O(sqrt(n)) |
| Spartan-KE | universal | O(n) | O(log^2 n) | O(log^2 n) |
| Spartan-RO | public | O(n log n) | O(log^2 n) | O(log^2 n) |

## Relevance to VOLE-Based zkVM

Spartan is directly relevant to VOLE-based zkVM design for three reasons:

1. **SPARK -> Lasso -> Jolt**: Spartan's SPARK primitive is the direct predecessor of Lasso. Both solve the same problem — efficiently proving evaluations of sparse multilinear polynomials — using offline memory checking. Lasso generalizes SPARK's memory-checking approach into a general-purpose lookup argument and adds the "surge" optimization for structured tables. Jolt then uses Lasso lookups to build an instruction-set zkVM. Understanding SPARK is essential for understanding how the Lasso/Jolt zkVM achieves its efficiency, and how it compares to the VOLE-based approach.

2. **Sum-check as the core protocol**: Spartan demonstrates that the sum-check protocol is the fundamental building block for succinct arguments over R1CS. VOLE-based protocols do not use sum-check — they use IT-MACs and gate-by-gate evaluation instead. This is the fundamental architectural difference: sum-check gives succinctness (sublinear proofs) but requires polynomial commitments; IT-MACs give streaming evaluation and information-theoretic security but produce linear-size proofs. A hybrid approach might use VOLE for the witness-linear parts and sum-check for the succinctness.

3. **R1CS as the common target**: Both Spartan and VOLE-ZK protocols target arithmetic constraint satisfaction. Spartan works natively with R1CS; VOLE-ZK protocols work with arithmetic circuits. The translation between the two is mechanical (R1CS generalizes circuit satisfiability with at most a constant-factor overhead), so performance comparisons are meaningful.
