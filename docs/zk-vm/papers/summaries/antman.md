# AntMan: Interactive Zero-Knowledge Proofs with Sublinear Communication

**Authors**: Chenkai Weng, Kang Yang, Zhaomin Yang, Xiang Xie, Xiao Wang
**Venue**: ACM CCS 2022 | **ePrint**: [2022/566](https://eprint.iacr.org/2022/566)

## Role in the VOLE-ZK Design Space

AntMan is the first VOLE-based ZK protocol to achieve **sublinear communication** relative to circuit size. Prior VOLE-ZK protocols (Wolverine, QuickSilver, Mac'n'Cheese, LPZK) all require communication linear in the circuit: at least 1 field element per multiplication gate. AntMan introduces two modes:

1. **SIMD (batched) mode**: For `B` executions of a circuit `C`, communication is `O(B + |C|)` instead of `O(B*|C|)`. Addition gates are free.
2. **Generic (single-execution) mode**: For a single execution of an arbitrary circuit `C`, communication is `O(|C|^{3/4})`.

The protocol integrates seamlessly with existing VOLE-ZK protocols: use AntMan for SIMD sub-circuits, use standard VOLE-ZK for the rest.

## Key Ideas

### Information-Theoretic Polynomial Authentication Code (IT-PAC)

The core primitive. An IT-PAC commits to a degree-`k` polynomial `f(.)` using two secret keys held by the verifier:

- **Polynomial key** `Lambda`: a uniform field element, reused across IT-PACs in one batch
- **Global key** `Delta`: the standard IT-MAC global key from VOLE

The commitment `[f(.)]` satisfies: P holds `(f(.), M)`, V holds `(K, Delta, Lambda)`, with `M = K + f(Lambda) * Delta`.

Key properties:
- **Hiding**: No party knows `f(Lambda)` (P doesn't know `Lambda`, V doesn't know `f(.)`).
- **Binding**: Opening to a different polynomial requires guessing `Lambda` or `Delta`. Soundness error `<= (k+1)/|F|`.
- **Additively homomorphic**: Given `[f(.)]` and `[g(.)]` under the same `Lambda`, both parties locally compute `[f(.) + g(.)]`.
- **Generalizes IT-MACs**: When `f(.)` is a constant, IT-PAC reduces to a standard IT-MAC.

An IT-PAC on a degree-`k` polynomial commits to `k+1` values simultaneously (the evaluations at `k+1` fixed points), but costs only `O(1)` communication to authenticate, versus `O(k)` for `k+1` separate IT-MACs.

### Sublinear ZK for SIMD Circuits

For `B` executions of circuit `C`, encode the `B` wire values on each wire into a degree-`(B-1)` polynomial via Lagrange interpolation at fixed points `alpha_1, ..., alpha_B`. Commit each wire polynomial with an IT-PAC. The protocol has three parts:

**1. Addition gates**: Free. IT-PACs are additively homomorphic, so `[h(.)] := [f(.)] + [g(.)]` is computed locally.

**2. Multiplication gates** (two sub-checks):

- *Polynomial multiplication check*: For input polynomials `f(.), g(.)` (degree `B-1`), P computes `h_tilde(.) = f(.) * g(.)` (degree `2B-2`) and commits it via IT-PAC. Then V reveals `Lambda` to P (after all openings are committed), and both parties use a standard DVZK check (QuickSilver-style) to verify `h_tilde(Lambda) = f(Lambda) * g(Lambda)`. By Schwartz-Zippel, this suffices. Cost: `O(1)` per gate group.

- *Degree reduction check (BatchCheck)*: P also computes a degree-`(B-1)` polynomial `h(.)` that agrees with `h_tilde(.)` at the `B` evaluation points. The check `h(alpha_i) = h_tilde(alpha_i)` for all `i` is "sacrificable": generate random IT-PACed polynomials `(r(.), s(.))` satisfying the same property, take a random linear combination across all gate groups, open the result. Communication: `O(B)` total, independent of number of gates.

**3. Input/output consistency**: IT-MACs on individual input values are linked to IT-PACs via the Lagrange basis: `[z_j] := sum_i delta_i(Lambda) * [w_{i,j}]`, then CheckZero on `[u_j(Lambda)] - [z_j]`.

### Sublinear ZK for Generic Circuits

Decompose the circuit into individual gates. Group every `B` same-type gates and apply the SIMD technique (each gate has only 1 "execution", but there are `B` gates per group). The main new challenge: **wire consistency** between groups.

If the output of gate `i` in group `A` feeds the input of gate `j` in group `B`, we need `f_hat(alpha_i) = g_hat(alpha_j)`. These checks are organized by `(i,j)` index pair. For each of the `B^2` possible `(i,j)` pairs, run a BatchCheck across all wire tuples with that index pair. Each BatchCheck costs `O(B)`, so total wire-consistency cost is `O(B^3)`.

Total communication: `O(|C|/B + B^3)`. Setting `B = |C|^{1/4}` gives `O(|C|^{3/4})`.

### IT-PAC Generation Protocol

Uses additively homomorphic encryption (BGV with one level) to distribute IT-PACs efficiently:

1. V encrypts powers `Enc(Lambda^1), ..., Enc(Lambda^k)` and sends to P (one-time setup cost `O(k)`).
2. For each polynomial `f(.)`, P homomorphically computes `Enc(f(Lambda) - u)` where `u` is from a random VOLE correlation, and sends one ciphertext to V.
3. V decrypts to get `b = f(Lambda) - u`, sets `K = v - b*Delta`.

Cost: `O(k + ell)` for `ell` polynomials of degree `k`. Security against malicious V uses "commit-then-open": V commits the seed for `Lambda` and ciphertext randomness; P commits its ciphertexts. V opens seed first; if ciphertexts were wrong, P aborts before opening its messages.

## Cost Model

### SIMD Mode: `B` executions of circuit `C`

| Component | Communication | Computation |
|-----------|--------------|-------------|
| IT-PAC generation (setup) | `O(B)` (AHE ciphertexts) | BGV operations |
| Commit wire polynomials | `O(|C|)` (one ciphertext per polynomial) | `O(|C|)` |
| BatchCheck (degree reduction) | `O(B)` | `O(|C| * B)` NTT |
| DVZK (polynomial mult check) | `O(1)` per gate group | `O(|C|)` |
| **Total** | **`O(B + |C|)`** | **`O(B * |C| * log B)`** |

When `B >= |C|`, group `k` circuits into one bigger circuit: communication becomes `O(sqrt(B * |C|))`.

### Generic Mode: single execution of circuit `C`

| Component | Communication |
|-----------|--------------|
| IT-PAC commit + SIMD checks | `O(|C|/B + B)` |
| Wire consistency (B^2 BatchChecks) | `O(B^3)` |
| **Total (B = |C|^{1/4})** | **`O(|C|^{3/4})`** |

Computation: `O(|C| * log |C|)`.

### Comparison to QuickSilver

| Metric | QuickSilver | AntMan (SIMD, B=2048) |
|--------|-------------|----------------------|
| Communication per mult gate | 1 field element | 0.0064 field elements |
| Improvement factor | -- | **156x** |

## Concrete Performance

Field: `F_p` with `p = 2^59 - 2^28 + 1`. Security: `lambda = 128`, `rho > 40`. Testbed: two EC2 m5.8xlarge instances, same region.

### Table 1: Communication and running time vs. batch size (|C| = 2^20, 4 threads, 1 Gbps)

| B | Setup (ms) | Per gate (us) | Per gate (field elements) |
|---|-----------|--------------|--------------------------|
| 16 | 138 | 0.241 | 0.82 |
| 64 | 263 | 0.156 | 0.205 |
| 256 | 761 | 0.142 | 0.051 |
| 1024 | 2743 | 0.141 | 0.0127 |
| 2048 | 5445 | 0.141 | 0.0064 |
| QuickSilver | 0 | 0.107 | 1 |

AntMan per-gate time stabilizes at ~0.141 us for `B >= 128` (vs. QuickSilver's 0.107 us). The 30% computation overhead is offset by 156x communication savings at `B=2048`. Setup cost (5.1 MB, independent of `B` up to 2048) is amortized over the circuit.

### Table 2: Throughput (mgps) vs. bandwidth and threads (B=1024, |C|=2^21)

| Protocol | 10 Mbps | 50 Mbps | 100 Mbps | 1 Gbps |
|----------|---------|---------|----------|--------|
| AntMan-1 | 1.79 | 2.00 | 2.05 | 2.09 |
| AntMan-4 | 4.86 | 6.88 | 6.69 | 7.01 |
| AntMan-16 | 7.56 | 14.07 | 15.86 | 17.74 |
| QuickSilver-inf | 0.17 | 0.85 | 1.7 | 16.95 |

Key observation: AntMan throughput is nearly **bandwidth-independent** above 50 Mbps (computation-bound). QuickSilver is **bandwidth-bound** (linear in bandwidth). At 50 Mbps / 16 threads: AntMan achieves **14 mgps vs. 0.85 mgps** (16.5x throughput improvement).

### Table 3: Microbenchmark (amortized ns per mult gate, B=256, |C|=2^22)

| Operation | Time (ns) | Share |
|-----------|----------|-------|
| BGV homomorphic eval | 84.53 | 60% |
| Polynomial multiplication (NTT) | 24.84 | 18% |
| Others (comm, hashing, etc.) | 32.69 | 22% |
| **Total** | **142.06** | |

BGV ciphertext operations dominate. The setup (BGV encryption + rotation) is one-time and amortizable.

## Key Definitions

- **`[f(.)]`** (IT-PAC) -- Polynomial authentication code: P holds `(f(.), M)`, V holds `(K, Delta, Lambda)`, with `M = K + f(Lambda) * Delta`. Generalizes IT-MAC to polynomials.
- **`BatchCheck_{k,m,t}`** -- Batch procedure to verify `f_j(alpha_i) = g_j(beta_i)` for all `i in [1,t], j in [1,ell]`, where `f_j` has degree `k` and `g_j` has degree `m`. Communication: `O(k + m)`, independent of `ell`.
- **`Pi^{SIMD}_{ZK}`** -- The SIMD-circuit ZK protocol. Proves `B` executions of circuit `C` in `O(B + |C|)` communication. Constant rounds.
- **`Pi^{generic}_{ZK}`** -- The generic-circuit ZK protocol. Proves one execution of any circuit `C` in `O(|C|^{3/4})` communication. Sets `B = |C|^{1/4}`, decomposes into gates, applies SIMD technique plus wire-consistency checks.
- **IT-PAC generation (`Pi^k_{IT-PAC}`)** -- Uses BGV AHE. V sends encryptions of powers of `Lambda`; P homomorphically evaluates each polynomial; VOLE correlation converts the result into an IT-PAC. Cost: `O(k + ell)` for `ell` degree-`k` polynomials.

## Relevance to VOLE-Based zkVM

1. **Batched execution mode is directly applicable if the VM proves many executions of the same sub-program.** RAM consistency checks, repeated hash/cipher invocations, and activation functions in ML inference are all SIMD-structured. For `B` iterations of a sub-circuit with `|C|` mult gates, AntMan reduces communication from `B*|C|` to `B + |C|` field elements.

2. **Single-execution sublinear mode reduces proof size for large programs.** For a circuit with `|C|` mult gates, communication drops from `|C|` to `|C|^{3/4}` field elements. However, this mode has `O(|C| log |C|)` computation (vs. `O(|C|)` for QuickSilver) and the generic-circuit protocol is not yet implemented/benchmarked -- the paper only benchmarks the SIMD protocol.

3. **Seamless integration with standard VOLE-ZK.** AntMan provides conversion procedures between IT-PACs and IT-MACs, so SIMD sub-circuits can be proved with AntMan while the remaining circuit uses QuickSilver/Wolverine. This composability is essential for a zkVM where some instruction patterns repeat (SIMD) and others don't.

4. **Communication efficiency matters more than raw throughput for a zkVM proof system.** AntMan's throughput is comparable to QuickSilver at high bandwidth, but its 156x communication reduction at `B=2048` translates to much lower bandwidth requirements. For a proof system that must transmit proofs over WAN, this is a significant practical advantage.

5. **The IT-PAC primitive is a reusable building block.** Any future VOLE-ZK protocol that needs to commit to polynomial-structured data (lookup arguments, memory consistency checks) can use IT-PACs to achieve sublinear communication for that component.
