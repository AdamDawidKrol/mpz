# Line-Point Zero Knowledge and Its Applications

**Authors**: Samuel Dittmer, Yuval Ishai, Rafail Ostrovsky
**Venue**: ITC 2021 | **ePrint**: [2020/1446](https://eprint.iacr.org/2020/1446)

## Role in the VOLE-ZK Design Space

LPZK provides the cleanest conceptual abstraction of VOLE-based ZK: the prover encodes the witness as an affine line `v(t) = at + b` in a vector space `F^n`, and the verifier queries the line at a single random point `t = alpha`. This is the theoretical foundation that Wolverine, QuickSilver, and Mac'n'Cheese all optimize in various directions. Where those papers work with the IT-MAC relation `M[x] = K[x] + Delta * x` as an operational primitive, LPZK shows why that relation works — it is the evaluation of an affine line at a secret point.

## Key Ideas

### The Line-Point Abstraction

An LPZK proof system is defined by a pair of algorithms `(Prove, Verify)`:

- **Prove(F, C, w)**: given circuit `C` and witness `w`, outputs vectors `a, b in F^n` defining the affine line `v(t) := a*t + b`.
- **Verify(F, C, alpha, v_alpha)**: given the evaluation `v_alpha = a*alpha + b` at a random point `alpha in F`, outputs `acc` or `rej`.

The key property: for any fixed `alpha`, each entry `v_i(alpha) = a_i * alpha + b_i` is **information-theoretically encrypted** when `b_i` is uniformly random, because `v_i(alpha)` is uniformly distributed regardless of `a_i`. This is exactly the VOLE correlation — the prover holds `(a, b)` and the verifier holds `v = a*alpha + b` for secret `alpha`.

### Mapping LPZK to VOLE

An LPZK proof compiles directly to an NIZK in the rVOLE-hybrid model:

1. **Setup**: Random VOLE gives the prover random `(a', b')` and the verifier `v' = a'*alpha + b'`.
2. **Online**: The prover sends corrections `(a - a')` and `(b - b')` to the verifier, who computes `v = v' + (a - a')*alpha + (b - b')`.

Two optimizations reduce cost below the naive `2n` field elements:
- If an entry of `a` or `b` is chosen uniformly at random (independent of witness), the prover sets the correction to zero — **no communication needed** for that entry.
- If an entry of `a` is always zero, the prover sends the corresponding `b` entry in the clear — **saves one VOLE entry**.

These are captured by the `(n, n', n'')` complexity measure: total dimension `n`, entries depending on witness `n'`, entries with `a_i = 0` always `n''`. The compiler uses VOLE of length `n - n''` and communication of `n' + n''` field elements.

### Addition Gates (Free)

Given encrypted wires `v_1(t) = a_1*t + b_1` and `v_2(t) = a_2*t + b_2`:

- Prover locally computes `(a_1 + a_2)*t + (b_1 + b_2)`.
- Verifier locally computes `v_1(alpha) + v_2(alpha)`.

**No communication or VOLE consumption.** Same as in Wolverine/QuickSilver.

### Multiplication Gates (Core Construction)

To prove `a_1 * a_2 = a_3` (the product of two wire values), where each is encrypted as `v_i(t) = a_i*t + b_i`:

1. The prover computes `v_1(t) * v_2(t)` — this is a **quadratic** `a_1*a_2*t^2 + (a_1*b_2 + a_2*b_1)*t + b_1*b_2`.
2. By introducing a random mask `b_3*t`, the prover decomposes:
   - `v_3(t) = a_1*a_2*t + (a_1*b_2 + a_2*b_1 - b_3)` — the encrypted product wire.
   - `v_4(t) = b_3*t + b_1*b_2` — a masking auxiliary wire.
   - Relation: `v_1(t)*v_2(t) = t*v_3(t) + v_4(t)`.
3. The verifier checks: `v_3(alpha) = (v_1(alpha)*v_2(alpha) - v_4(alpha)) / alpha`.

**Soundness**: if `a_3 != a_1*a_2`, the verification polynomial is a nontrivial quadratic in `alpha`, so the check passes with probability at most `2/|F|`.

**Zero knowledge**: the verifier sees `v_i(alpha)` for `i = 1..4`. Since `b_1, b_2, b_3, b_4` are uniform and independent, `v_1, v_2, v_3` are uniform, and `v_4` is determined by the check equation. A simulator produces indistinguishable output.

### Single Gate Example (Commit-and-Prove)

For the relation `R(x, y, z) := xy - z`, this is realized as a `(5, 4, 1)`-LPZK:

```
a = (x, y, z, x*b_2 + y*b_1 - b_3, 0)
b = (b_1, b_2, b_3, b_4, b_1*b_2 - b_4)    // b_1..b_4 random
```

Verifier checks: `v_1*v_2 - alpha*v_3 - v_4 - v_5 = 0`.

### Full Circuit Satisfiability Protocol

For a circuit `C` with `k` inputs, `k'` outputs, and `m` multiplication gates:

1. The prover sets `a_0 = 1, b_0 = 0`. Entries `a_1, ..., a_k` are the witness; `b_1, ..., b_k` are random.
2. For each multiplication gate `i`, the prover propagates inputs via the circuit's linear relations `R_C` and computes:
   - `a_{k+4i-1} := a_{k+4i-3} * a_{k+4i-2}` (the product).
   - `a_{k+4i} := a_{k+4i-3}*b_{k+4i-2} + a_{k+4i-2}*b_{k+4i-3} - b_{k+4i-1}` (the cross term).
3. For output wires, `a_{k+4m+i} = 0` and `b_{k+4m+i} := r_{2m+i} . b`.
4. **Batching**: multiplication checks are batched in blocks of `t` gates. For each block, the prover computes a product of residuals `c_i` and encodes it as a single LPZK entry. This reduces communication from 3 to `2 + 1/t` field elements per multiplication gate, at the cost of increasing soundness error to `2t/|F|`.

The shortened vector `(a_hat, b_hat)` has length `k + k' + (2 + 1/t)*m + 1` — the two input wires per multiplication gate are reconstructed by the verifier from `R_C`.

### ROM Variant (Section 5)

In the random oracle model, all multiplication gates are batched into a single block:

1. The prover commits a vector `w` of per-gate correction values and evaluates `H(w)` (an `r x m` matrix from `F^m -> F^{mr}`).
2. The residual check becomes `M*s = M*y*alpha + M*z` where `s, y, z` are derived from the gate evaluations.
3. Communication drops to `k + k' + m + 2r` field elements — roughly **1 element per multiplication gate** plus `2r` overhead.
4. Soundness: `2/|F| + ell/|F|^r` where `ell` is the number of oracle queries by a malicious prover.

### Contrast with Wolverine/Mac'n'Cheese

A subtle but important difference: in LPZK, the **prover** holds the full line `v(t) = a*t + b` and the verifier gets a single point `v(alpha)`. In Wolverine and Mac'n'Cheese, the prover holds `a` (values), the verifier holds `b` (keys) and `alpha` (global key Delta), and the prover learns `v(alpha)` (the MAC tags). LPZK's formulation allows the verifier to locally compute the multiplication check `v_3 = (v_1*v_2 - v_4)/alpha` without any communication for the auxiliary wire, reducing per-gate cost.

## Cost Model

| Setting | Communication per mult gate | Prover computation | Verifier computation |
|---------|---------------------------|-------------------|---------------------|
| Information-theoretic | `2 + 1/t` field elements | 3 mults/gate (< 4x cleartext) | 4 mults/gate (< 5x cleartext) |
| Random oracle model | `1 + 2r/m` field elements | `3 + 2r` mults/gate + 1 hash | `3 + r` mults/gate + 1 hash |

- Addition gates are **free** (no communication, no VOLE).
- rVOLE length: `k + 2m` (IT) or `k + m + r` (ROM).
- For large fields, set `t = log|F|` (IT) or `r = 1` (ROM).
- Memory: **streaming-friendly**. Beyond what's needed for circuit evaluation in the clear, the verifier stores only O(1) field elements per batch (the running product of check residuals). The prover stores double (values from both `a` and `b` in scope).

## Concrete Performance

The paper is primarily theoretical and does not include implementation benchmarks. The comparison with concurrent work gives relative figures:

| Metric | LPZK (IT) | LPZK (ROM) | Wolverine (ROM) | Mac'n'Cheese |
|--------|-----------|------------|-----------------|--------------|
| Communication/gate | `2 + 1/t` elts | `1 + o(1)` elts | `2 + o(1)` elts | `3 + o(1)` elts |
| Prover mults/gate | 3 | `3 + 2r` | `4r + 6` | 13 |
| Verifier mults/gate | 4 | `3 + r` | `7r` | 10 |
| Hash calls/gate | 0 | 0 (1 total) | 2/gate + 1 total | 3/gate |

At `r = 1` (sufficient for large fields), LPZK ROM achieves 5 prover mults/gate and 4 verifier mults/gate with a single hash over the entire proof, vs. Wolverine's 10 prover mults/gate and 7 verifier mults/gate plus per-gate hashing.

## Key Definitions

- **LPZK** — Line-Point Zero Knowledge: proof system where P encodes witness as affine line `v(t) = a*t + b in F^n`, V queries `v(alpha)` for random `alpha`.
- **`(n, n', n'')`-LPZK** — Refined complexity measure: `n` = total dimension, `n'` = entries of `a, b` depending on witness, `n''` = entries where `a_i = 0` always. Communication = `n' + n''`; VOLE length = `n - n''`.
- **rVOLE (random VOLE)** — Trusted setup giving P random `(a', b')` and V random `alpha, v' = a'*alpha + b'`. The LPZK-to-NIZK compiler uses the self-reduction `v = v' + (a - a')*alpha + (b - b')`.
- **Batching parameter `t`** — Multiplication checks are grouped in blocks of `t`. Amortized communication: `2 + 1/t` per gate. Soundness error: `2t/|F|`. Trade-off: larger `t` = less communication, worse soundness.
- **Wire encryption** — Entry `v_i(alpha) = a_i*alpha + b_i` with `b_i` uniform is information-theoretically independent of `a_i`. This is the mechanism that provides zero knowledge.

## Relevance to VOLE-Based zkVM

1. **Conceptual foundation**: LPZK is the simplest explanation of why VOLE correlations give you zero knowledge — the VOLE output `a*alpha + b` is an evaluation of the prover's affine line at the verifier's secret point. Understanding this abstraction is the key to understanding all VOLE-ZK protocols (Wolverine, QuickSilver, Mac'n'Cheese).
2. **The multiplication gate construction** — decomposing a quadratic into `t*v_3(t) + v_4(t)` and checking at a random point — is the template that all subsequent protocols follow, with varying approaches to batching and soundness amplification.
3. **Free additions, expensive multiplications**: the same cost structure as all VOLE-ZK protocols. For a zkVM circuit, this means minimizing multiplication gates is the primary optimization target.
4. **Streaming model**: LPZK is explicitly designed for streaming/space-efficient evaluation — the prover and verifier process gates left-to-right with memory proportional to the circuit width, not size. This maps directly to instruction-by-instruction VM execution.
5. **QuickSilver is LPZK + subfield VOLE**: Yang et al.'s QuickSilver protocol applies Wolverine's subfield VOLE techniques to the LPZK framework, achieving efficient soundness amplification over small fields. Understanding LPZK is prerequisite to understanding QuickSilver.
