# Unlocking the Lookup Singularity with Lasso

**Authors**: Srinath Setty, Justin Thaler, Riad Wahby
**ePrint**: [2023/1216](https://eprint.iacr.org/2023/1216)

## Role in the VOLE-ZK Design Space

Lasso is a lookup argument that enables the companion work Jolt (a zkVM frontend). It allows a prover to commit to a vector `a in F^m` and prove that all entries reside in a predetermined table `t in F^N`, with prover costs that scale with the number of lookups `m` and (for structured tables) never require materializing the full table of size `N`. For structured (decomposable) tables, costs scale with `c` subtables of size `N^{1/c}` rather than `N`. This is relevant to a VOLE-based zkVM because instruction verification via lookups is a powerful paradigm -- the entire evaluation table of each RISC-V instruction can be expressed as a structured lookup table, replacing per-instruction circuit logic with table lookups. The question is whether Lasso's approach (sum-check + sparse polynomial commitments + offline memory checking) can be adapted to the VOLE setting, or whether a VOLE-native lookup argument with analogous "pay only for what you use" properties can be designed.

## Key Ideas

### The Core Reduction: Lookups to Sparse Polynomial Evaluation

The statement "all entries of `a` are in table `t`" is equivalent to: the prover knows a sparse matrix `M in {0,1}^{m x N}` with exactly one `1` per row, such that `M * t = a`. Via multilinear extensions (MLEs), this reduces to checking:

```
sum_{y in {0,1}^{log N}} M~(r, y) * t~(y) = a~(r)
```

for a random `r in F^{log m}` chosen by the verifier. `M~` is sparse (at most `m` nonzeros), so Lasso commits to `M~` using Spark and proves the equation using Surge.

### Spark: Sparse Polynomial Commitment from Spartan

Spark is a commitment scheme for sparse multilinear polynomials from Spartan [Set20]. A sparse `log(N)`-variate multilinear polynomial `g` with `m` nonzero evaluations over `{0,1}^{log N}` is committed by its dense representation: three `log(m)`-variate polynomials `(row, col, val)` listing the nonzero positions and values.

To prove `g(r)`, the prover proves it correctly ran a time-optimal evaluation algorithm:
1. Decompose `r = (r_1, ..., r_c)` into `c` blocks of `log(N)/c` variables each.
2. For each block `r_i`, evaluate all `N^{1/c}` Lagrange basis polynomials at `r_i` in time `O(N^{1/c})`, storing results in memory `i`.
3. For each of the `m` nonzero terms, compute the full Lagrange basis evaluation via `c` lookups (one per memory) and multiply results.

Total time: `O(c * m + c * N^{1/c})`.

Correctness of the memory reads is verified via **offline memory checking** [BEG+91]: each memory cell maintains a counter incremented on each read. The prover commits to read values, read counters, and final counters, then proves `WS = RS ∪ S` (the write-set equals read-set union final-state) using multiset hashing with a grand product argument.

**Lasso's strengthened analysis**: Spartan assumed the sparse polynomial metadata was committed honestly (by a setup). Lasso proves Spark remains secure even when committed by an adversarial prover, yielding the first "standard" sparse polynomial commitment with optimal prover costs.

### Surge: Generalizing Spark to Structured Tables

Spark computes the inner product `<u, t>` of a sparse committed vector `u` with a dense structured vector `t` (specifically, the Lagrange basis evaluations, which factor as a tensor product). Surge generalizes this to any table with **Spark-Only Structure (SOS)**, aka **decomposable** tables.

An SOS table `T` of size `N` decomposes into `alpha = k * c` subtables `T_1, ..., T_alpha`, each of size `N^{1/c}`, and a combining function `g` such that:

```
T[r] = g(T_1[r_1], ..., T_k[r_1], T_{k+1}[r_2], ..., T_{2k}[r_2], ..., T_{alpha}[r_c])
```

where `r = (r_1, ..., r_c)` breaks the index into `c` chunks. The algorithm for computing `sum_i eq~(i, r) * T[nz(i)]` iterates over the `m` lookups, performing `alpha` subtable reads per lookup and combining via `g`.

The Surge prover commits to:
- `c` polynomials `dim_1, ..., dim_c` (subtable indices, `log(m)` variables each)
- `alpha` polynomials `E_1, ..., E_alpha` (read values) and `read_ts_1, ..., read_ts_alpha` (read counters), each over `log(m)` variables
- `alpha` polynomials `final_cts_1, ..., final_cts_alpha` (final counters), each over `log(N^{1/c})` variables

A sum-check on `h(k) := eq~(r, k) * g(E_1(k), ..., E_alpha(k))` reduces to point evaluations, then memory-checking grand products verify each `E_i` is well-formed.

### Structured Table Examples

Tables that are decomposable (and hence support Lasso directly):

| Table | Size | Decomposition |
|-------|------|---------------|
| Range check `{0, ..., N-1}` | `N` | `T[r] = sum_j 2^j * r_j` (MLE is linear in bits) |
| Bitwise AND of `b`-bit inputs | `2^{2b}` | `T[x,y] = sum_i 2^{i-1} * x_i * y_i` (degree-2 MLE) |
| Bitwise XOR/OR | `2^{2b}` | Similar per-bit decomposition |
| RISC-V instructions (Jolt) | `2^{128}` | Each instruction decomposes into subtable lookups on chunked operands |

For Jolt applied to RISC-V with 64-bit data types: the prover commits to ~65 field elements per CPU step, of which ~1/3 are in `{0,1}`, only 5 exceed `2^{22}`, and none exceed `2^{64}`. Equivalent to ~6 MSMs of size `T` (number of execution steps).

### The "Small Witnesses" Property

All field elements committed by the Lasso prover are **small**: they lie in `{0, ..., max{m, N^{1/c}, q} - 1}` where `q` is the max subtable entry. For MSM-based commitments, Pippenger's algorithm on small exponents (bounded by `K`) uses roughly 1 group operation per term when `K <= n`, vs. `O(lambda / log(n))` group ops per term for random field elements. This yields a ~10x speedup over committing to arbitrary field elements.

### GeneralizedLasso: Beyond Decomposable Tables

For tables that are MLE-structured but not decomposable, GeneralizedLasso applies the sum-check directly to `M~(r, y) * t~(y)` using a **sparse-dense sum-check protocol**. The key technique is "expansion then consolidation": track entities until their first bit-difference is bound (expansion for `O(m)` per chunk of `log(m)` rounds), then consolidate in `O(m)` time. Total prover field work: `O(c * m)` for `N = m^c`. The disadvantage: `c * m` of the committed elements are random (not small), losing the MSM speedup on those elements.

## Cost Model

| Setting | Prover commits to | Element sizes | Prover field ops |
|---------|-------------------|---------------|------------------|
| Unstructured table (`c = 1`) | `m + N` field elements | all in `{0, ..., m}` | `O(m + N)` |
| SOS table (general `c`) | `3cm + alpha * N^{1/c}` elements (`alpha = kc`, typically `alpha = c` with `k=1`) | all in `{0, ..., max{m, N^{1/c}, q}}` | `O(c * m + alpha * N^{1/c})` |
| GeneralizedLasso (MLE-structured) | `3cm + c * N^{1/c}` elements | `cm` are random, rest small | `O(c * m)` field ops via sparse-dense sum-check |

- **No party commits to the table** (or subtables) if they are MLE-structured -- the verifier evaluates `t~(r)` in `O(log N)` time.
- **Verification**: `O(log m)` rounds, `O(c * k * log m)` field elements communicated, verifier time `O(k * log m)` field ops.
- **Soundness error**: `O(m + N^{1/c}) / |F|` from sum-check + multiset hashing.

### Comparison to Prior Lookup Arguments

| Scheme | Prover group work | Prover field work | Setup |
|--------|-------------------|-------------------|-------|
| Plookup | `O(N)` | `O(N log N)` | SRS of size `N` |
| Halo2 lookups | `O(N)` | `O(N log N)` | SRS of size `N` |
| cq | `7m + o(m)` | `O(m log m)` | SRS of size `N` |
| **Lasso (SOS)** | `o(cm + cN^{1/c})` | `O(cm)` | **None** (transparent) |
| **Lasso (unstructured)** | `min{2m+O(sqrt(N)), m+o(N)}` | `O(m + N)` | **None** |

Lasso is the first lookup argument where: (1) prover work is sublinear in `N` for structured tables with no SRS, (2) all committed values are small, and (3) the table need never be materialized.

## Key Definitions

- **Lookup argument**: Given commitment to `a in F^m` and table `t in F^N`, prove `a_i in {t_0, ..., t_{N-1}}` for all `i`.
- **Indexed lookup argument**: Additionally given commitment to index vector `b in F^m`, prove `a_i = t[b_i]` for all `i`.
- **Structured table (SOS / decomposable)**: A table `T` of size `N` such that `T[r] = g(T_1[r_1], ..., T_alpha[r_c])` for subtables `T_i` of size `N^{1/c}` and a combining polynomial `g`.
- **MLE-structured table**: A table whose multilinear extension `t~(r)` can be evaluated in `O(log N)` time.
- **Spark**: Sparse polynomial commitment scheme from Spartan. Commits to dense representation of a sparse multilinear polynomial, proves evaluations via offline memory checking.
- **Surge**: Generalization of Spark that proves `<u, t> = v` for sparse committed `u` and structured dense `t`, where `t` is an SOS table.
- **Offline memory checking**: Technique from [BEG+91]. A checker maintains read-set `RS` and write-set `WS` for an untrusted memory. If all reads return correct values, then `WS = RS ∪ S` for a computable final-state set `S`. Checked via multiset fingerprinting: `H_{tau,gamma}(WS) = H_{tau,gamma}(RS) * H_{tau,gamma}(S)` using Reed-Solomon + product fingerprints.
- **Small field elements**: Elements in `{0, ..., K}` for `K << |F|`. MSM-based commitments on small elements use `O(1)` group ops per element (via Pippenger) vs. `O(lambda / log n)` for random elements.

## Relevance to VOLE-Based zkVM

1. **Lookup paradigm for instruction verification**: Lasso + Jolt demonstrate that every RISC-V instruction can be verified via a single structured lookup, replacing per-instruction arithmetic circuits. A VOLE-based zkVM could adopt this paradigm if efficient VOLE-native lookups exist.

2. **Can VOLE implement lookup arguments?** Lasso's internals are sum-check + polynomial commitments + grand products. The VOLE setting has IT-MACs instead of polynomial commitments and operates in the designated-verifier model. The question is: can the "commit to sparse matrix + memory check" structure be realized with VOLE correlations? The offline memory checking pattern (read/write sets, counter polynomials, multiset equality) is structurally compatible with VOLE's commit-and-prove approach.

3. **The small-witnesses advantage in VOLE context**: In Lasso, all committed values being small is crucial for MSM efficiency. In a VOLE-based system, "small" committed values have a different (but potentially analogous) benefit: subfield VOLE correlations are cheaper to generate and authenticate when values are small or binary. A VOLE-native lookup could exploit this.

4. **Decomposable tables and VOLE**: The subtable decomposition `T[r] = g(T_1[r_1], ..., T_alpha[r_c])` is independent of the proof system backend. Any VOLE-based lookup could reuse the same decomposition framework to avoid materializing large tables.

5. **Grand products in VOLE**: Lasso's memory checking reduces to grand product arguments (product of fingerprinted multisets). Grand products via sum-check have VOLE-compatible analogs (e.g., QuickSilver-style random linear combination checks). The concrete question is whether VOLE can efficiently implement the `H_{tau,gamma}` multiset equality check without blowing up communication.
