# QuickSilver: Efficient and Affordable Zero-Knowledge Proofs for Circuits and Polynomials over Any Field

**Authors**: Kang Yang, Pratik Sarkar, Chenkai Weng, Xiao Wang
**Venue**: ACM CCS 2021 | **ePrint**: [2021/076](https://eprint.iacr.org/2021/076)

## Role in the VOLE-ZK Design Space

QuickSilver is the most practically efficient VOLE-based ZK protocol for general circuits. It achieves the communication-optimal rate of **1 field element per non-linear gate** for any field size (including binary), with optimal streaming memory `O(M)`. It also introduces a **polynomial mode** that achieves sublinear communication for structured computations (e.g., matrix multiplication: `O(n^2)` instead of `O(n^3)`). Built on top of Wolverine's IT-MAC/sVOLE paradigm and LPZK's linear-relation insight, it unifies the best of both: LPZK's optimal communication with Wolverine's support for arbitrary fields.

## Key Ideas

### Core Insight: Multiplication Check as Linear Relation

Wolverine checks multiplication gates using cut-and-choose (small fields) or Beaver triples (large fields), costing 2--7 field elements per gate. LPZK observed that the IT-MAC structure itself can be exploited, but only for large fields. QuickSilver generalizes this to any field via subfield VOLE.

For a multiplication gate with wire values `(w_alpha, w_beta, w_gamma)` where `w_gamma = w_alpha * w_beta`, each party holds authenticated values `[w_i]` with `k_i = m_i + w_i * Delta`. The verifier computes:

```
B_i = k_alpha * k_beta - k_gamma * Delta
```

Expanding using `k_i = m_i + w_i * Delta`:

```
B_i = m_alpha * m_beta + (w_beta * m_alpha + w_alpha * m_beta - m_gamma) * Delta + (w_alpha * w_beta - w_gamma) * Delta^2
      \_____________/   \_______________________________________________/     \_________________________/
       A_{0,i}                          A_{1,i}                              = 0 if correct
     (known to P)                     (known to P)
```

If `w_gamma = w_alpha * w_beta`, the `Delta^2` term vanishes and we get a linear relation `B_i = A_{0,i} + A_{1,i} * Delta`. If the gate is wrong, this becomes a degree-2 equation in `Delta`, holding with probability at most `2/p^r`.

### Batch Verification via Random Linear Combination

All `t` per-gate linear relations are checked in a single batch. V samples `chi <- F_{p^r}` after all values are defined, then checks:

```
sum_{i in [t]} B_i * chi^i  =  sum_{i in [t]} A_{0,i} * chi^i  +  (sum_{i in [t]} A_{1,i} * chi^i) * Delta
         B (known to V)                  A_0 (known to P)                     A_1 (known to P)
```

This reduces `t` checks to one. The single check is masked using a random VOPE correlation `(A_0*, A_1*)` with `B* = A_0* + A_1* * Delta`: P sends `U = A_0 + A_0*` and `V = A_1 + A_1*` to V, who checks `B + B* = U + V * Delta`.

### Extended sVOLE: Vector Oblivious Polynomial Evaluation (VOPE)

QuickSilver introduces `F_{ext-sVOLE}`, which extends sVOLE with a VOPE command. Given degree `d`:
- P gets random coefficients `{A_i}_{i in [0,d]}` over `F_{p^r}`
- V gets `B = sum_{i in [0,d]} A_i * Delta^i`

This is built from `2d - 1` sVOLE correlations over `F_{p^r}` (packed from subfield sVOLE), iteratively multiplied with re-randomization at each step to prevent a malicious V from forcing zero coefficients. Statistical error: `(d-1)/p^r`.

### Circuit ZK Protocol (Protocol Pi_{ZK})

**Preprocessing** (circuit/witness unknown):
1. Initialize ext-sVOLE, V gets `Delta`.
2. Generate `n + t` sVOLE correlations: `{[mu_i]}_{i in [n]}` for inputs, `{[nu_i]}_{i in [t]}` for mult gates.
3. Generate one VOPE correlation of degree 1: P gets `(A_0*, A_1*)`, V gets `B* = A_0* + A_1* * Delta`.

**Online** (circuit and witness known):
1. **Input commitment**: For each input wire `i`, P sends `delta_i := w_i - mu_i` to V. Both compute `[w_i] := [mu_i] + delta_i`.
2. **Gate evaluation** (topological order):
   - **Add**: `[w_gamma] := [w_alpha] + [w_beta]`. Free.
   - **Mult** (i-th): P sends `d_i := w_alpha * w_beta - nu_i` to V. Both set `[w_gamma] := [nu_i] + d_i`. Cost: **1 field element**.
3. **Consistency check**: For each mult gate `i`, P computes `A_{0,i}, A_{1,i}`, V computes `B_i`. V sends random `chi`. P sends masked `(U, V)`. V checks `W = U + V * Delta`.
4. **Output check**: P opens `m_h` for output wire `h`. V checks `k_h = m_h + Delta`.

Online phase is 3 rounds, or non-interactive via Fiat-Shamir (hash `chi` from transcript).

Soundness: `(t + 3)/p^r` (information-theoretic).

### Polynomial ZK Protocol (Protocol Pi_{polyZK})

For `t` polynomials `f_1, ..., f_t` each of degree at most `d` over `n` variables, where each `f_i` is in "degree-separated" format: `f_i = sum_{h in [0,d]} f_{i,h}` with all terms in `f_{i,h}` having degree exactly `h`.

The key idea: instead of evaluating `f(k_1, ..., k_n)` directly, the verifier computes a **degree-shifted** evaluation:

```
sum_{h in [0,d]} f_{i,h}(k_1, ..., k_n) * Delta^{d-h}
```

Substituting `k_j = m_j + w_j * Delta` and expanding, the `Delta^d` coefficient becomes `f_i(w_1,...,w_n)`, which is 0 if the polynomial is satisfied. What remains is a degree `(d-1)` polynomial in `Delta`:

```
B_i = sum_{h in [0,d-1]} A_{i,h} * Delta^h
```

where P can compute `{A_{i,h}}` locally. This is checked using a VOPE correlation of degree `d-1`.

**Protocol**:

**Preprocessing**:
1. `n` sVOLE correlations for input commitment.
2. One VOPE correlation of degree `d - 1`.

**Online**:
1. Commit witness: P sends `delta_i := w_i - s_i` for each variable.
2. For each polynomial `f_i`: V computes `B_i`, P computes coefficients `{A_{i,h}}` of `g_i(x) = sum_{h in [0,d]} f_{i,h}(m_1 + w_1*x, ..., m_n + w_n*x) * x^{d-h}`.
3. Batch check all `t` polynomials with random `chi`: P sends `U_h := sum_{i in [t]} A_{i,h} * chi^i + A_h*` for each `h in [0, d-1]`. V checks `W = sum_{h in [0,d-1]} U_h * Delta^h`.

Communication: **`n` elements over `F_p` + `d` elements over `F_{p^r}`**, independent of the number of multiplications in the polynomials or the number of polynomials `t`.

Soundness: `(d + t)/p^r`.

**Computing polynomial coefficients**: P evaluates `g_i` at `d+1` fixed points over `F_{p^r}`, then recovers coefficients via Lagrange interpolation. For simple polynomials (inner product, matrix mult), coefficients can be derived directly.

**Computational complexity** (in `F_{ext-sVOLE}`-hybrid model, `z` = max terms in any polynomial):
- Prover: `O(t * d^2 * z + d * n)`
- Verifier: `O(t * d * z)`

### Difference from Wolverine

| Aspect | Wolverine | QuickSilver |
|--------|-----------|-------------|
| Mult gate check | Cut-and-choose (small fields) or Beaver triples (large fields) | Exploit IT-MAC linearity for any field |
| Comm/gate (Boolean) | ~7 bits (with cut-and-choose, rho=40) | **1 bit** |
| Comm/gate (large field) | 2--4 elements | **1 element** |
| Computation/gate | ~0.66--1.25 M gates/sec | **4.8--7.7 M gates/sec** |
| Beyond gate-by-gate | No | Polynomial mode (sublinear) |

QuickSilver eliminates the need for multiplication triples entirely. The check is derived from the algebraic structure of IT-MACs, so no auxiliary correlated randomness is needed beyond the sVOLE correlations that commit the wire values.

## Cost Model

### Circuit Mode

| Resource | Cost |
|----------|------|
| Communication per mult gate | 1 element over `F_p` (online), plus sVOLE generation (sublinear) |
| sVOLE correlations | `n + t` (one per input wire + one per mult gate) |
| VOPE correlations | 1 of degree 1 (requires `1 * r` subfield sVOLE) |
| Batch check overhead | 2 elements over `F_{p^r}` (the masked `U, V`) |
| Addition gates | **Free** |
| Total (sVOLE-hybrid) | `(n + t) * log(p) + 2 * kappa` bits |
| Prover memory | `O(M)` (streaming, same as clear evaluation) |

### Polynomial Mode

| Resource | Cost |
|----------|------|
| Witness commitment | `n` elements over `F_p` |
| Polynomial check | `d` elements over `F_{p^r}` (d-1 VOPE + batch) |
| Total (sVOLE-hybrid) | `(n + d*r) * log(p)` bits |
| Independent of | Number of multiplications, number of polynomials `t` |

### Application-Specific Costs

| Application | Circuit-mode cost | Polynomial-mode cost |
|-------------|-------------------|---------------------|
| Matrix mult `A*B = C` (`n x n`) | `(2n^2 + n^3) * log(p) + 2*kappa` | `2n^2 * log(p) + 2*kappa` |
| SIS proof (binary `s`, length `m`) | `m * log(q)` + per-gate | `m * log(q) + 2*kappa` |
| SIS proof (`s in [-B,B]^m`) | large | `m * log(q) + (2B+1)*kappa` |
| Integer mult (n-bit, amortized over `t ~ 8n*kappa`) | `O(n^2)` bits/mult | `~4n` bits/mult |

### Dollar Costs (AWS EC2, cheapest instances, single thread)

- Boolean: ~$1 per **trillion** AND gates (c6g.medium, ARM, 1.9 cents/hr)
- Arithmetic (`F_{2^61-1}`): ~$2.50 per **trillion** mult gates

## Concrete Performance

### Circuit Mode (kappa=128, rho~100 for Boolean / rho>=40 for arithmetic)

**Throughput** (million gates/second):

| Threads | Boolean (50 Mbps) | Boolean (local) | Arith (2 Gbps) | Arith (local) |
|---------|-------------------|-----------------|----------------|---------------|
| 1 | 7.5 M | 7.6 M | 4.8 M | 4.8 M |
| 4 | 14.9 M | 15.8 M | 8.9 M | 8.9 M |

Computation becomes the bottleneck at ~50 Mbps (Boolean) / ~2 Gbps (arithmetic).

**vs. prior work** (1 thread, local):

| Protocol | Boolean comm | Boolean speed | Arith comm | Arith speed |
|----------|-------------|---------------|------------|-------------|
| Wolverine | 7 elements | 1.25 M/s | 4 elements | 0.66 M/s |
| Mac'n'Cheese | -- | -- | 3 elements | 0.4 M/s |
| LPZK | -- | -- | 1 element | -- (no impl) |
| **QuickSilver** | **1 element** | **7.7 M/s** | **1 element** | **4.8 M/s** |

Improvement over Wolverine: **6x computation, 7x communication** (Boolean); **7x computation, 3--4x communication** (arithmetic).

### Polynomial Mode

**Inner product** (single thread, 500 Mbps for large field):

| Vector length | 10^6 | 10^7 | 10^8 |
|---------------|------|------|------|
| Process witness (F_{2^61-1}) | 0.39 s | 3.9 s | 39.2 s |
| Prove inner product (ms) | 42.8 | 100.3 | 703.8 |

**Matrix multiplication** (1024 x 1024, F_{2^61-1}, 1 thread, 1 GB memory, 500 Mbps):

| Protocol | Time | Communication | Memory |
|----------|------|--------------|--------|
| Spartan | >= 5000 s | <= 100 KB | 600 GB (crashed) |
| Virgo | 357 s | 221 KB | 148 GB |
| Wolverine | 1627 s | 34 GB | small |
| Mac'n'Cheese | 2684 s | 25.8 GB | small |
| **QS (Circuit)** | **316 s** | **8.6 GB** | **1 GB** |
| **QS (Polynomial)** | **10 s** | **25.2 MB** | **1 GB** |

Polynomial mode is **31x faster** and uses **340x less communication** than QuickSilver's own circuit mode for this workload.

**SIS proof** (ternary vector, n=2048, m=1024, log q=32):

| Protocol | Communication | Time |
|----------|--------------|------|
| ENS | 53 KB | -- |
| Wolverine | 32.8 KB | 220 ms |
| **QuickSilver** | **4.1 KB** | **2 ms** |

8x less communication, 110x faster than Wolverine.

## Key Definitions

- **`[x]`** -- Authenticated value (same as Wolverine): P holds `(x, m)`, V holds `k`, with `m = k - Delta * x` where `Delta in F_{p^r}` is V's global key.
- **sVOLE** -- Subfield VOLE: generates `[x]` for random `x in F_p` with tags/keys in `F_{p^r}`.
- **VOPE** -- Vector Oblivious Polynomial Evaluation: P gets random `{A_i}_{i in [0,d]}` over `F_{p^r}`, V gets `B = sum A_i * Delta^i`. Built from `2d-1` packed sVOLE correlations.
- **`F_{ext-sVOLE}`** -- Extended sVOLE functionality combining both sVOLE and VOPE with the same global key `Delta`.
- **Degree-separated format** -- A polynomial `f` of degree `d` written as `f = sum_{h=0}^{d} f_h` where every term in `f_h` has degree exactly `h`.
- **`B_i = A_{0,i} + A_{1,i} * Delta`** -- The per-gate linear relation derived from the IT-MAC structure. `B_i = k_alpha * k_beta - k_gamma * Delta` (V computes), `A_{0,i} = m_alpha * m_beta`, `A_{1,i} = w_alpha * m_beta + w_beta * m_alpha - m_gamma` (P computes).

## Relevance to VOLE-Based zkVM

1. **1 element/gate is the baseline for general Wasm execution.** Every Wasm instruction that maps to multiplication gates costs exactly 1 field element in the circuit mode. This is the tightest possible bound in the gate-by-gate paradigm.

2. **Polynomial mode for structured host functions.** Wasm host functions like matrix operations, inner products, or hash preimage checks can be expressed as low-degree polynomial sets and proven with sublinear communication. The matrix multiplication result (`O(n^2)` instead of `O(n^3)`) is directly applicable.

3. **Weak-uniformity optimization for repeated sub-circuits.** In a Wasm VM, many instructions (e.g., memory loads, table lookups) compile to the same sub-circuit repeated many times. If these sub-circuits have bounded polynomial degree `d`, the polynomial mode can "dig holes" in the circuit and prove them collectively with `d * kappa` bits instead of per-gate cost. The amortized cost per sub-circuit becomes `O(sqrt(N))` when `t = O((n + 2^d) / sqrt(N))`.

4. **Non-interactive online phase.** Via Fiat-Shamir, the online phase (where the circuit/witness are known) requires no interaction from V. This simplifies integration into a zkVM pipeline where the prover runs asynchronously.

5. **Streaming memory model.** Both protocols require only `O(M)` memory (same as evaluating the circuit in the clear). This is essential for a zkVM proving long Wasm executions -- memory does not grow with trace length.

6. **VOPE as a building block.** The extended sVOLE functionality with VOPE support is a reusable primitive. Any future zkVM gadget that benefits from oblivious polynomial evaluation (e.g., lookup arguments, range checks modeled as low-degree polynomials) can leverage this.
