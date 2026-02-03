# JesseQ: Efficient Zero-Knowledge Proofs for Circuits over Any Field

**Authors**: Mengling Liu, Yang Heng, Xingye Lu, Man Ho Au
**Venue**: IEEE S&P 2025 | **ePrint**: [2025/533](https://eprint.iacr.org/2025/533)

## Role in the VOLE-ZK Design Space

JesseQ advances the base layer of VOLE-based zero-knowledge proofs by halving the per-gate computational cost compared to QuickSilver. It introduces two protocols -- JQv1 (arbitrary circuits, any field) and JQv2 (layered circuits, any field) -- that replace the degree-2 polynomial checking of QuickSilver/LPZKv2 with a degree-1 checking polynomial, eliminating expensive extension-field multiplications on the prover side. JQv1 requires only **2 scalar multiplications** (element in `F_p` times element in `F_{p^r}`) per AND gate versus QuickSilver's **4 full extension-field multiplications**, and **2 field multiplications** per arithmetic multiplication gate versus IT-LPZKv2's 3. Batch verification is done via a hash function (BLAKE3) instead of algebraic random-linear-combination, yielding further concrete speedups. JesseQ is not a disjunction protocol but improves the foundation that Batchman and every other VOLE-ZK framework builds on.

## Key Ideas

### Core Insight: Degree-1 Checking Polynomial via qsVOLE

QuickSilver's multiplication check constructs `f(X) = p_alpha(X) * p_rho(X) - X * p_upsilon(X)`, a degree-2 polynomial. Its `X^2` coefficient is `w_alpha * w_rho - w_upsilon`, which vanishes iff the gate is correct. The prover sends `A_0, A_1` and the verifier checks linearity -- but computing `A_0` requires a full extension-field multiplication (`m_alpha * m_rho`).

JesseQ's key observation is that if the preprocessing provides **quadratic subfield VOLE (qsVOLE)** -- i.e., `[y = u_alpha * u_rho]` alongside `[u_alpha], [u_rho], [u_upsilon]` -- then the checking polynomial can be reduced to degree 1. After P sends `d_i := w_i - u_i` for each wire, the checking polynomial becomes:

```
f(X) = d_rho * p_{u_alpha}(X) + d_alpha * p_{u_rho}(X) + p_y(X) + d_rho * d_alpha * X - p_{upsilon}(X)

     = [d_rho * m_{u_alpha} + d_alpha * m_{u_rho} + m_y - m_{w_upsilon}]      (constant term: a_0)
     + [(u_alpha + d_alpha) * (u_rho + d_rho) - w_upsilon] * X              (linear term: a_1)
```

where `p_{u_i}(X) = m_{u_i} + u_i * X`, `p_y(X) = m_y + y * X`, `p_{upsilon}(X) = m_{w_upsilon} + w_upsilon * X`.

The linear coefficient `a_1 = w_alpha * w_rho - w_upsilon` vanishes iff the gate is correct. If so, `f(X)` is constant and `a_0 = f(x)`.

**Why this is cheaper:** The constant term `a_0` requires only **scalar multiplications** (`d_alpha in F_p` times `m_{u_rho} in F_{p^r}`), never full extension-field multiplications. For Boolean circuits where `d_alpha in F_2`, the scalar multiplication is just a conditional copy (free or nearly free).

### Quadratic Subfield VOLE (qsVOLE)

The `F_qsVOLE` functionality extends standard sVOLE with a "Quadratic" command. Given stored authenticated random values `{[u_i]}` and a circuit `{(f_j, I_j)}`:

1. Compute `y_j := f_j({u_i}_{i in I_j}) - g_{j,0}` (the degree-2 polynomial minus its constant, evaluated on random u-values).
2. Generate authenticated values `[y_j]` such that `k_{y_j} = m_{y_j} + y_j * x`.

This is realized from extended sVOLE (`F_{ext-sVOLE}`) using the same approach as QuickSilver's degree-2 extension. The `Quadratic` command is verified internally using a random-linear-combination batch check with a VOPE masking correlation. Soundness: `(2 + |G|) / p^r`.

### Hash-Based Batch Checking (JQv1)

Instead of QuickSilver's algebraic batch check (random `chi`, compute `sum B_i * chi^i`), JQv1 uses a cryptographic hash `H: {0,1}* -> {0,1}^kappa`:

1. P computes `A := H(m_{1,zero}, ..., m_{|G|,zero})` and sends `A` to V.
2. V computes `B := H(k_{1,zero}, ..., k_{|G|,zero})` and checks `A = B`.

Where per gate `j`:
- P: `m_{j,zero} := g_{j,2}({m_{u_i} + d_i}) - g_{j,2}({m_{u_i}}) + m_{y_j} - m_{w_upsilon_j}`
- V: `k_{j,zero} := g_{j,2}({k_{u_i} + d_i}) - g_{j,2}({k_{u_i}}) + k_{y_j} - k_{w_upsilon_j} + f_j({d_i}) * x`

The `g_{j,2}({m + d}) - g_{j,2}({m})` form avoids multiplying two extension-field elements; for a single multiplication gate `w_alpha * w_rho`, this is `(m_{u_alpha} + d_alpha) * (m_{u_rho} + d_rho) - m_{u_alpha} * m_{u_rho}`, but the second term is precomputed. On a large field (`r = 1`), this reduces to **1 multiplication** for the term instead of 2.

Concrete advantage: BLAKE3 on 10M 61-bit elements is at least 2x faster than 10M 61-bit field multiplications. This makes hash-based batch checking cheaper than the algebraic alternative (which costs 3 mults over `F_{p^r}` per gate in QuickSilver).

Soundness: `(q_H + 1) / p^r + 1 / 2^kappa`, where `q_H` is the number of random oracle queries.

### Large-Field Optimization (JQv1)

When `r = 1` (large field, no extension needed), the degree-2 term computation can be further optimized. Instead of computing `sum c_i * (d_{alpha_i} * m_{u_{rho_i}} + d_{rho_i} * m_{u_{alpha_i}})` as two multiplications per term, compute:

```
g_{j,2}({d_i + m_{u_i}}) - g_{j,2}({m_{u_i}}) - g_{j,2}({d_i})
```

The second term `g_{j,2}({m_{u_i}})` is precomputed. The subtraction of `g_{j,2}({d_i})` can be omitted because it only shifts the constant term (absorbed by the hash check). Result: **1 multiplication per gate** instead of 2.

### JQv2: Half Communication for Layered Circuits

JQv2 targets layered circuits (each gate's inputs come from the layer immediately below). The observation (shared with LPZKv2) is that for even-layer gates:

```
g(X) = d_rho * p_{u_alpha}(X) + d_alpha * p_{u_rho}(X) + p_y(X) + d_rho * d_alpha * X
```

already authenticates `w_alpha * w_rho` without P sending any `d_upsilon`. The MAC and key of the output wire are computed directly:

- P: `m_{w_upsilon} := g_{j,2}({m_{u_i} + d_i}) - g_{j,2}({m_{u_i}}) + m_{y_j}`
- V: `k_{w_upsilon} := g_{j,2}({k_{u_i} + d_i}) - g_{j,2}({k_{u_i}}) + k_{y_j} + f_j({d_i}) * x`

For odd-layer gates, P sends `d_upsilon` and a QuickSilver-style degree-2 batch check (with random `chi`) is used. Assuming even layers have >= half the gates, the amortized communication is **1/2 element per gate**. For random circuits, roughly 38% communication reduction vs JQv1.

Soundness: `(3 + |G_1|) / p^r`, where `|G_1|` is the number of odd-layer gates.

### JQv1 Protocol (Figure 4)

**Preprocessing:**
1. Initialize `F_qsVOLE`, V gets `x in F_{p^r}`.
2. Generate `n + |G|` sVOLE correlations: `{[u_{tau_i}]}` for input wires and gate outputs.
3. Invoke qsVOLE Quadratic for all gates: get `{[y_j = f_j({u_i}) - g_{j,0}]}`.

**Online:**
1. **Input commit:** P sends `d_{tau_i} := w_{tau_i} - u_{tau_i}` for each input. Both compute `[w_{tau_i}] := [u_{tau_i}] + d_{tau_i}`.
2. **Gate output commit:** For gate `j` (j'-th in G), P sends `d_{upsilon_j} := f_j({w_i}) - u_{tau_{n+j'}}`. Both set `[w_{upsilon_j}] := [u_{tau_{n+j'}}] + d_{upsilon_j}`. Cost: **1 element in `F_p`**.
3. **Per-gate check values:** For each gate, P computes `m_{j,zero}`, V computes `k_{j,zero}` (formulas above).
4. **Batch check:** P sends `A = H(m_{1,zero}, ..., m_{|G|,zero})`. V checks `A = H(k_{1,zero}, ..., k_{|G|,zero})`.
5. **Output check:** P sends `m_h` for output wire. V checks `k_h = m_h + x`.

Online rounds: 3 (or non-interactive via Fiat-Shamir).

### JQv2 Protocol (Figure 5)

**Preprocessing:**
1. Initialize `F_qsVOLE`, V gets `x`.
2. Generate `n + |G_1|` sVOLE correlations (input wires + odd-layer gate outputs only).
3. One VOPE correlation of degree 1: P gets `(M_0, M_1)`, V gets `K = M_0 + M_1 * x`.
4. Invoke qsVOLE Quadratic for even-layer gates only: get `{[y_j]}_{j in G_0}`.

**Online:**
1. **Input commit:** Same as JQv1.
2. **Even-layer gates (`j in G_0`):** Both parties directly compute `[w_{upsilon_j}]` from input wire MACs/keys (no communication).
3. **Odd-layer gates (`j in G_1`):** P sends `d_{upsilon_j}`. Both compute `[w_{upsilon_j}] := [u_{tau_{n+j'}}] + d_{upsilon_j}`. P also computes `a_{0,j}, a_{1,j}` and V computes `b_j` for the degree-2 check.
4. **Batch check (odd layers):** V sends random `chi`. P sends `(u_0, u_1)` masked with VOPE. V checks `w = u_0 + u_1 * x`.
5. **Output check:** Same as JQv1.

### Batchman Integration

JesseQ plugs directly into the Batchman framework for batched disjunctions. Batchman uses an underlying VOLE-ZK protocol as a black box for multiplication gates. Replacing QuickSilver with JQv1 or JQv2 yields 1.1x to 2.2x improvement. The substitution is seamless because JesseQ preserves the same IT-MAC commitment structure and 1-element-per-gate communication.

## Cost Model

### Per-Gate Online Computation (Any Field)

**JQv1 vs QuickSilver:**

| | F_p mult | F_p add | F_{p^r} mult | F_{p^r} add | Scalar mult | Hash |
|---|---|---|---|---|---|---|
| **QS Prover** | 1 | 1 | 4 | 4 | 2 | 0 |
| **JQv1 Prover** | 1 | 1 | 0 | 2 | 2 | <1 |
| **QS Verifier** | 0 | 0 | 4 | 3 | 1 | 0 |
| **JQv1 Verifier** | 1 | 0 | 0 | 5 | 3 | <1 |

Key: JQv1 eliminates all 4 extension-field multiplications on the prover and all 4 on the verifier, replacing them with scalar multiplications and hash amortization.

**JQv2 vs QuickSilver (amortized, layered):**

| | F_p mult | F_p add | F_{p^r} mult | F_{p^r} add | Scalar mult | Hash |
|---|---|---|---|---|---|---|
| **QS Prover** | 1 | 1 | 4 | 4 | 2 | 0 |
| **JQv2 Prover** | 1 | 0.5 | 2 | 3 | 2 | 0 |
| **QS Verifier** | 0 | 0 | 4 | 3 | 1 | 0 |
| **JQv2 Verifier** | 0.5 | 0 | 2 | 3 | 2 | 0 |

### Per-Gate Online Computation (Large Field, r=1 optimization)

**JQv1 vs IT-LPZKv2:**

| | Prover mult | Prover add | Prover hash | Verifier mult | Verifier add | Verifier hash |
|---|---|---|---|---|---|---|
| **IT-LPZKv2** | 3 | 5 | 0 | 3 | 2 | 0 |
| **JQv1** | 2 | 2 | <1 | 4 | 4 | <1 |

**JQv2 vs ROM-LPZKv2:**

| | Prover mult | Prover add | Prover hash | Verifier mult | Verifier add | Verifier hash |
|---|---|---|---|---|---|---|
| **ROM-LPZKv2** | 8.5 | 8.5 | <1 | 8.5 | 8.5 | <1 |
| **JQv2** | 4 | 3.5 | 0 | 4 | 3.5 | 0 |

### Communication per Multiplication Gate

| Protocol | Elements per gate | Field |
|---|---|---|
| Wolverine | 7 (Boolean) / 4 (large) | any / large |
| QuickSilver | 1 | any |
| IT-LPZKv2 | 1 | large |
| ROM-LPZKv2 | 1/2 | large (layered) |
| **JQv1** | **1** | **any** |
| **JQv2** | **1/2** | **any (layered)** |

### Preprocessing Requirements

| | sVOLE correlations | qsVOLE invocations | VOPE |
|---|---|---|---|
| JQv1 | `n + |G|` | 1 (all gates) | 0 |
| JQv2 | `n + |G_1|` | 1 (even-layer gates) | 1 (degree 1) |

### Application-Specific Communication (Online Phase)

| Application | JQv1 | JQv2 |
|---|---|---|
| Inner product (length n) | `(n+1) log p + kappa` bits | `n log p + kappa` bits |
| Matrix mult (n x n) | `3n^2 log p + kappa` bits | `2n^2 log p + kappa` bits |
| SIS (binary s, length m) | `(2m+n) log p + kappa` bits | `m log p + kappa` bits |
| SIS (s in [-B,B], length m) | `(m+n+2mB) log p + kappa` bits | `(m+mB) log p + 3*kappa` bits |

## Concrete Performance

### Online Throughput (single thread, 3 x 10^8 gates)

**Boolean circuit (AND gates):**

| Protocol | 20 Mbps | 30 Mbps | 50 Mbps | Local |
|---|---|---|---|---|
| JQv1 | 19.5 M/s | 40.6 M/s | 64.1 M/s | 64.1 M/s |
| JQv2 | 34 M/s | 34 M/s | 34 M/s | 34 M/s |
| QuickSilver | -- | -- | -- | 8.6 M/s |

**Arithmetic circuit (F_{2^61-1}, mult gates):**

| Protocol | 500 Mbps | 1 Gbps | 2 Gbps | Local |
|---|---|---|---|---|
| JQv1 | 7.4 M/s | 14.2 M/s | 23.3 M/s | 23.3 M/s |
| JQv2 | 12.5 M/s | 12.5 M/s | 12.5 M/s | 12.5 M/s |
| QuickSilver | -- | -- | -- | 7.8 M/s |

JQv1 saturates at ~50 Mbps (Boolean) and ~2 Gbps (arithmetic). JQv2 saturates at ~10 Mbps (Boolean) and ~400 Mbps (arithmetic) due to lower communication.

### Speedup vs State-of-the-Art (from Table 1)

| Protocol | Boolean speed | Arith speed |
|---|---|---|
| Wolverine | 1.25 M/s | 0.96 M/s |
| IT-LPZKv2 | -- | 21.8 M/s |
| ROM-LPZKv2 | -- | 9.8 M/s |
| QuickSilver | 8.6 M/s | 7.8 M/s |
| **JQv1** | **64.1 M/s** (7.5x QS) | **23.3 M/s** (3.0x QS) |
| **JQv2** | **34.2 M/s** (4.0x QS) | **13.7 M/s** (1.8x QS) |

### Detailed Time Breakdown (3 x 10^8 gates, single thread)

**Boolean (50 Mbps / Local):**

| Phase | JQv1 | JQv2 | QuickSilver |
|---|---|---|---|
| Cir-Ind preprocessing | 3 s | 2 s | 3 s |
| Cir-Dep preprocessing | 32 s | 20 s | -- |
| Online | 4.5 s | 9 s | 32 s |

**Arithmetic (2 Gbps / Local):**

| Phase | JQv1 | JQv2 | QuickSilver |
|---|---|---|---|
| Cir-Ind preprocessing | 3 s | 2 s | 3 s |
| Cir-Dep preprocessing | 32 s | 20 s | -- |
| Online | 13 s | 24 s | 36 s |

JesseQ trades a more expensive preprocessing (qsVOLE) for a much faster online phase. Total time (Model 2, preprocessing + online) is state-of-the-art.

### Dollar Cost (AWS, cheapest instances, online phase)

| Protocol | Instance | CPU | Boolean gates/cent | Arith gates/cent | Boolean gates/$ |
|---|---|---|---|---|---|
| JQv1 | t3a.small ($0.018/hr) | AMD | 92 B | 58 B | **9.2 T** |
| JQv2 | t3a.small ($0.018/hr) | AMD | 64 B | 18 B | 6.4 T |
| JQv1 | t3.small ($0.02/hr) | Intel | 96 B | 25 B | 9.6 T |
| JQv2 | t3.small ($0.02/hr) | Intel | 54 B | 14 B | 5.4 T |

### Applications

**Inner product (online phase, ms):**

| Length | 10^6 (F_2) | 10^7 (F_2) | 10^8 (F_2) | 10^6 (F_{61}) | 10^7 (F_{61}) | 10^8 (F_{61}) |
|---|---|---|---|---|---|---|
| QuickSilver | 36 | 69 | 423 | 42 | 100 | 703 |
| JQv2 | 3.2 | 38 | 400 | 5.9 | 60.6 | 648 |

**Matrix multiplication (1024 x 1024, F_{2^61-1}, 500 Mbps):**

| Protocol | Time | Communication |
|---|---|---|
| Spartan | >= 5000 s | <= 100 KB |
| Virgo | 357 s | 221 KB |
| QuickSilver | 10 s | 25.2 MB |
| **JQv2** | **7 s** | **16.7 MB** |

**SIS proof (n=2048, m=1024, log q=61):**

| Protocol | Time | Communication |
|---|---|---|
| QuickSilver | 22 ms | 8.2 KB |
| JQv2 | 7 ms | 8.2 KB |

### Batchman Integration (batched disjunctions, 2^21 gates per repetition)

| Protocol | 100 Mbps | 500 Mbps | 1 Gbps |
|---|---|---|---|
| AntMan (16 threads) | 15.86 M/s | 17.51 M/s | 17.74 M/s |
| QS-Batchman | 104.91 M/s | 335.02 M/s | 461.82 M/s |
| **JQv1-Batchman** | **122.26 M/s** | **569.44 M/s** | **1051.01 M/s** |
| **JQv2-Batchman** | **144.76 M/s** | **666.47 M/s** | **1190.22 M/s** |

JQv2-Batchman at 1 Gbps: 2.6x over QS-Batchman.

### Hash vs Multiplication Benchmarks (m5.2xlarge, single thread)

| Operation | 10^6 (61-bit, ms) | 10^7 (61-bit, ms) | 10^8 (61-bit, ms) |
|---|---|---|---|
| Field mult | 3.87 | 38.5 | 386 |
| SHA256 | 20.3 | 203 | 2029 |
| BLAKE3 | 1.83 | 17.6 | 174 |

BLAKE3 is **2.2x faster** than field multiplication at 10^8 elements over 61-bit field, confirming the concrete benefit of hash-based batch checking.

## Key Definitions

- **`[u]`** -- IT-MAC authenticated value: P holds `(u, m)`, V holds `(k, x)`, satisfying `m = k - u * x` where `x in F_{p^r}` is V's global key. Same as QuickSilver/Wolverine.
- **sVOLE** -- Subfield VOLE: generates `[u]` for random `u in F_p` with tags/keys in `F_{p^r}`, where `r >= kappa / log p`.
- **qsVOLE (Quadratic subfield VOLE)** -- Extends sVOLE with a `Quadratic` command: given stored `{[u_i]}` and circuit `{(f_j, I_j)}`, generates `[y_j = f_j({u_i}) - g_{j,0}]`. Built from `F_{ext-sVOLE}` using QuickSilver's degree-2 extension method plus internal batch verification.
- **`F_{ext-sVOLE}`** -- Extended sVOLE functionality: combines standard sVOLE (Initialize, Extension) with VOPE (degree-d random polynomial evaluation). Provides unified global key `x` across all operations.
- **VOPE** -- Vector Oblivious Polynomial Evaluation: P gets random coefficients `{M_i}_{i in [0,d]}` over `F_{p^r}`, V gets `K = sum M_i * x^i`.
- **Scalar multiplication** -- Product `a * b` where `a in F_p, b in F_{p^r}`. Costs `O(kappa)`-bit operation. For Boolean (`a in F_2`), this is a conditional copy.
- **Extension-field multiplication** -- Product `a * b` where `a, b in F_{p^r}`. Costs `O(kappa log kappa)`-bit operation (via Karatsuba/NTT over the extension).
- **Degree-1 checking polynomial** -- `f(X) = a_0 + a_1 * X` where `a_1 = w_alpha * w_rho - w_upsilon`. Correct gate implies `f(X)` is constant (a_1 = 0). This is the core innovation vs QuickSilver's degree-2 polynomial.
- **Layered circuit** -- Circuit where each gate at layer `k` takes inputs only from gates at layer `k-1`. Required for JQv2's half-communication optimization.
- **`g_{j,2}`** -- The homogeneous degree-2 part of gate polynomial `f_j = g_{j,2} + g_{j,1} + g_{j,0}`. For a simple multiplication gate, `g_{j,2} = w_alpha * w_rho`.

## Relevance to VOLE-Based zkVM

1. **Halves the computational cost of the base execution layer.** Every Wasm instruction that compiles to multiplication gates benefits from JQv1's 2 scalar multiplications instead of QuickSilver's 4 extension-field multiplications. For a Boolean circuit (the natural choice for bit-level Wasm operations), this yields 7.5x online speedup.

2. **9.2 trillion AND gates per dollar sets a new cost floor.** On the cheapest AWS instances, JQv1 proves 92 billion AND gates per cent. This establishes the economic baseline for any VOLE-based zkVM: the raw gate-proving cost is negligible compared to memory and lookup overheads.

3. **JQv2's half-communication matters for layered VM circuits.** A typical step-circuit for a VM (fetch-decode-execute-writeback) is naturally layered. JQv2 halves communication to 1/2 element per gate for these circuits, reducing bandwidth requirements to saturate at ~10 Mbps (Boolean) instead of ~50 Mbps.

4. **Seamless Batchman integration for instruction dispatch.** A zkVM's instruction dispatch is a batched disjunction (one of N instruction circuits is active per step). JQv2-Batchman achieves 1.19 billion gates/sec at 1 Gbps -- 2.6x over QS-Batchman -- directly accelerating the most communication-heavy part of VM proving.

5. **Hash-based batch checking eliminates the batching bottleneck.** QuickSilver's algebraic batch check costs 3 extension-field multiplications per gate. JQv1 replaces this with BLAKE3 hashing, which is 2x faster per element. For a zkVM proving millions of gates per step, this removes a significant computational bottleneck.

6. **Preprocessing/online split aligns with zkVM architecture.** JesseQ's more expensive circuit-dependent preprocessing (qsVOLE generation) can run during idle/off-peak time, while the online phase (when witness is known) is 3-7x faster than QuickSilver. AWS dynamic pricing further incentivizes this split. The online phase is streamable with `O(M)` memory.

7. **Drop-in replacement for QuickSilver.** JesseQ preserves the same IT-MAC commitment structure, the same 1-element-per-gate communication format, and the same `F_{ext-sVOLE}` preprocessing interface. Any system built on QuickSilver (including Batchman) can substitute JesseQ with minimal integration effort.

8. **Application-level speedups compound with VM gadgets.** Inner product (2x), matrix multiplication (1.4x), and SIS proofs (3x) all improve. These directly correspond to zkVM host functions or gadgets (e.g., hash verification via lattice problems, memory consistency via inner products).
