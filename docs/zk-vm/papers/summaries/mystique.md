# Mystique: Efficient Conversions for Zero-Knowledge Proofs with Applications to Machine Learning

**Authors**: Chenkai Weng, Kang Yang, Xiang Xie, Jonathan Katz, Xiao Wang
**Venue**: USENIX Security 2021 | **ePrint**: [2021/730](https://eprint.iacr.org/2021/730)

## Role in the VOLE-ZK Design Space

Mystique solves the critical problem of **converting between arithmetic and boolean representations** in VOLE-ZK. Prior sVOLE-based ZK protocols (Wolverine, QuickSilver, Mac'n'Cheese, LPZK) efficiently handle either arithmetic circuits (over `F_p`) or boolean circuits (over `F_2`), but not mixed-mode computation. Real programs — and Wasm in particular — freely intermix arithmetic operations (add, mul) with bitwise operations (and, or, xor, shifts) and comparisons. Every transition between these representations requires a conversion, and the cost of these conversions dominates per-instruction overhead for many programs. Mystique provides the first efficient protocols for these conversions within the sVOLE-based ZK framework.

## Key Ideas

### The Core Abstraction: zk-edaBits

The central building block is the **ZK-friendly extended doubly authenticated bit** (zk-edaBit). A zk-edaBit is a tuple:

```
([r_0]_2, ..., [r_{m-1}]_2, [r]_p)
```

where `r_i in F_2` are authenticated bits (MAC'd over `F_{2^lambda}`), `[r]_p` is an authenticated field element (MAC'd over `F_{p^k}`), and the consistency invariant holds:

```
r = sum_{h=0}^{m-1} r_h * 2^h  mod p
```

Here `m = ceil(log p)` is the bit-width. The key insight (adapted from edaBits in the MPC setting [EGK+20]) is that this single object bridges the two domains: the boolean side `[r_i]_2` supports AND/XOR gates, the arithmetic side `[r]_p` supports ADD/MULT gates, and the consistency between them is checked once during generation rather than at every use.

**Generating zk-edaBits** (Protocol Pi_{zk-edaBits}):

1. Generate `ell = NB + c` *faulty* zk-edaBits: for each, sample random `r^i` in `F_p`, decompose to bits `(r^i_0, ..., r^i_{m-1})`, authenticate both sides via `F_{authZK}`. Cost: `O(log p)` bits per edaBit.
2. **Cut-and-bucket** to enforce consistency: place the first `N` into `N` buckets (one each). V picks a random permutation on the remaining `ell - N`. Open and check the last `c`. Distribute the remaining `N(B-1)` into the `N` buckets (each now has `B` entries).
3. For each bucket, **combine-and-open** check: for the first zk-edaBit `([r_h]_2, [r]_p)` and each other `([s_h]_2, [s]_p)`, compute `[t]_p := [r]_p + [s]_p` arithmetically and `([t_h]_2) := AdderModp([r_h]_2, [s_h]_2)` as a boolean circuit, then BatchCheck the bits and CheckZero on `[t]_p - t'` to verify consistency.
4. Output the first zk-edaBit from each bucket.

Soundness error: `C(N(B-1)+c, B-1)^{-1} + 1/p^k`. For `N = 10^6`, `B = 3`, `c = 2` gives >= 40-bit statistical security.

### A2B Conversion (Arithmetic to Boolean)

**Protocol Pi^{A2B}_{Convert}**: Given `[x]_p`, produce `([x_0]_2, ..., [x_{m-1}]_2)`.

1. Obtain a random zk-edaBit `([r_0]_2, ..., [r_{m-1}]_2, [r]_p)` from `F_{zk-edaBits}`.
2. Compute `[z]_p := [x]_p - [r]_p` and BatchCheck-open `z` (this reveals `z = x - r mod p`, which is uniformly random and leaks nothing about `x`).
3. Bit-decompose the *public* value `z` into `(z_0, ..., z_{m-1})`.
4. Evaluate the boolean circuit `AdderModp(z_0, ..., z_{m-1}, [r_0]_2, ..., [r_{m-1}]_2)` to obtain `([x_0]_2, ..., [x_{m-1}]_2)`, where the `z_i` are public constants (free in boolean circuits).

**Cost**: 1 zk-edaBit + 1 BatchCheck-open of one field element + 1 AdderModp boolean circuit evaluation.

### B2A Conversion (Boolean to Arithmetic)

**Protocol Pi^{B2A}_{Convert}**: Given `([x_0]_2, ..., [x_{m-1}]_2)`, produce `[x]_p`.

1. Obtain a random zk-edaBit `([r_0]_2, ..., [r_{m-1}]_2, [r]_p)`.
2. Evaluate the boolean circuit `AdderModp([x_0]_2, ..., [x_{m-1}]_2, [r_0]_2, ..., [r_{m-1}]_2)` to obtain `([z_0]_2, ..., [z_{m-1}]_2)` where `z = x + r mod p`.
3. BatchCheck-open `(z_0, ..., z_{m-1})`, recompose `z = sum z_h * 2^h mod p`.
4. Output `[x]_p := z - [r]_p` (local computation using additive homomorphism).

**Cost**: 1 zk-edaBit + 1 AdderModp circuit + 1 BatchCheck-open of `m` bits.

### C2A Conversion (Committed to Authenticated)

Converts from non-interactive public commitments to privately authenticated values usable in sVOLE-ZK.

**Hybrid commitment scheme**: Commit a set of messages `{x_i}_{i in [1,ell]}` by:
1. Sample key `sk`, randomness `r`. Publish `com_0 = H(sk, r)`.
2. Compute `c_i = PRF(sk, i) + x_i` for each message, build Merkle tree with root `com_1`.
3. Publish `(com_0, com_1)`.

**Conversion protocol** (Pi_{NICom -> [.]}): To convert committed `x_i` to `[x_i]`:
1. P sends `(c_i, path_i)` to V, who verifies against Merkle root `com_1`.
2. Parties authenticate `[sk]_2`, `[r]_2` via `F_{authZK}`, and P proves `com_0 = H([sk]_2, [r]_2)` in ZK (boolean circuit for hash).
3. Parties compute `[x_i] := c_i - PRF([sk]_2, i)` as a boolean circuit in `F_{authZK}`.

PRF is instantiated with LowMC (64-bit block size, 11 rounds), chosen for minimal AND-gate count. The key schedule is precomputed once and reused across all PRF evaluations.

### Matrix Multiplication Optimization

**Protocol Pi_{MatMul}**: Given authenticated `[A]`, `[B]`, `[C]` with `A in F_q^{n x m}`, `B in F_q^{m x ell}`, `C in F_q^{n x ell}`, prove `A * B = C`.

Uses a generalized **Freivalds check**:

1. V samples random vectors `u in (F_{q^k})^n`, `v in (F_{q^k})^ell`.
2. Both parties locally compute `[x]^T := u^T * [A]`, `[y] := [B] * v`, `[z] := u^T * [C] * v`.
3. Prove `[x]^T * [y] = [z]` — this is an inner product of dimension `m`, requiring `m` authenticated multiplications. Further optimized to `O(k log q)` bits using the QuickSilver polynomial check.
4. CheckZero on `[z] - [z']`.

**Reduction**: from `O(n^3)` private multiplications (naive) or `O(n^2)` (prior ZK [YSWW21]) down to **`n` private multiplications** for `n x n` matrices. Soundness error: `3/q^k`.

## Cost Model

All costs are for `p = 2^61 - 1` (Mersenne prime), `m = 61` bits, `lambda = 128`, `rho >= 40`.

### Per-Conversion Communication

| Conversion | Dominant cost |
|---|---|
| zk-edaBit generation | `O(log p)` bits per edaBit (sVOLE for `[r]_p` + `m` sVOLEs for `[r_i]_2`) |
| A2B | 1 zk-edaBit + open 1 field element + AdderModp circuit (`O(m)` AND gates) |
| B2A | 1 zk-edaBit + AdderModp circuit + open `m` bits |
| C2A | LowMC evaluation (~62 bits comm per 64-bit block via hybrid commitment) |
| MatMul (n x n) | `O(n^2)` field elements for inputs + `O(k log q)` bits for the Freivalds check |

### Cut-and-Bucket Overhead

For `N` zk-edaBits with bucket size `B` and check parameter `c`:
- Total faulty edaBits needed: `NB + c` (e.g., `B=3, c=2` means 3x overhead).
- Each bucket check costs: `(B-1)` AdderModp evaluations + `(B-1)` BatchChecks + `(B-1)` CheckZeros.

### AdderModp Circuit

A boolean circuit computing `(a + b) mod p` on two `m`-bit inputs. This is the dominant per-conversion boolean cost. For `m = 61`, this is a standard binary adder plus a conditional subtraction of `p`, requiring `O(m)` AND gates.

## Concrete Performance

All benchmarks: two EC2 m5.2xlarge (32 GB RAM), `p = 2^61 - 1`, `lambda = 128`, `rho >= 40`.

### Conversion Benchmarks (Table 2)

| Operation | 50 Mbps | 200 Mbps | 500 Mbps | 1 Gbps |
|---|---|---|---|---|
| A2B | 107 us | 45 us | 34 us | 29 us |
| B2A | 109 us | 49 us | 38 us | 33 us |
| C2A | 56 us | 55 us | 55 us | 55 us |
| Fix2Float | 50 us | 46 us | 46 us | 46 us |
| Float2Fix | 49 us | 46 us | 46 us | 46 us |

- **A2B/B2A** are bandwidth-sensitive (drop from ~110 us to ~30 us from 50 Mbps to 1 Gbps) because the dominant cost is sVOLE generation for the zk-edaBits, which is communication-bound.
- **C2A** is compute-bound (constant ~55 us) because it is dominated by the LowMC PRF evaluation in a boolean circuit.
- **Fix2Float/Float2Fix** are compute-bound (~46 us) — pure boolean circuit evaluations.

### zk-edaBit Preprocessing

Per zk-edaBit generation time: 95 us at 50 Mbps, decreasing to 19 us at 1 Gbps.

### Matrix Multiplication (Table 2)

| Dimension | 200 Mbps | 500 Mbps |
|---|---|---|
| 512 x 512 | 186 ms | 185 ms |
| 1024 x 1024 | 1.48 s | 1.39 s |
| 2048 x 2048 | 11.30 s | 10.63 s |

7x faster than prior best (QuickSilver [YSWW21]: ~10 s for 1024 x 1024 at 500 Mbps). Bottleneck is local matrix multiplication by the prover, not communication.

### Commitment Conversion (C2A)

55 us per 64-bit block. 18,000 blocks/sec (144 KB/sec) at >= 50 Mbps. Uses LowMC-64 (64-bit block, 11 rounds). Comparison: SHA-256 takes 395 us / 705 bits per commitment; LowMC-256 takes >= 1000 us / 49 bits. Mystique's hybrid scheme: 55 us / 62 bits — best balance of time and communication.

## Key Definitions

- **`[x]_p`** — Authenticated value over `F_p`: P holds `(x, M)`, V holds `K`, with `M = K + Gamma * x` in `F_{p^k}`.
- **`[x]_2`** — Authenticated bit over `F_2`: P holds `(x, M)`, V holds `K`, with `M = K + Delta * x` in `F_{2^lambda}`.
- **zk-edaBit** — Tuple `([r_0]_2, ..., [r_{m-1}]_2, [r]_p)` with `r = sum r_h * 2^h mod p`. Bridges boolean and arithmetic authenticated domains.
- **AdderModp** — Boolean circuit computing `(a + b) mod p` for `m`-bit inputs. Used in both A2B and B2A conversions, and in zk-edaBit consistency checks.
- **BatchCheck** — Open multiple authenticated values with amortized soundness. Cost: `ell * log q + lambda` bits for `ell` values.
- **CheckZero** — Special case of BatchCheck where all opened values should be 0 (P need not send values).
- **Cut-and-bucket** — Technique to verify consistency of `N` faulty zk-edaBits by generating `NB + c` total, opening `c` for direct check, then partitioning into buckets of size `B` for pairwise consistency checks.
- **A2B (convertA2B)** — Convert `[x]_p` to `([x_0]_2, ..., [x_{m-1}]_2)`: mask with zk-edaBit, open mask, add in boolean domain.
- **B2A (convertB2A)** — Convert `([x_0]_2, ..., [x_{m-1}]_2)` to `[x]_p`: add with zk-edaBit in boolean domain, open result, subtract in arithmetic domain.
- **C2A (convertC2A)** — Convert a non-interactive public commitment to authenticated values by proving the PRF relationship in ZK.

## Relevance to VOLE-Based zkVM

**Directly relevant.** Wasm i32/i64 instructions freely mix:
- **Arithmetic**: `i32.add`, `i32.mul`, `i32.sub` — natural in `F_p` (one authenticated multiplication per mul, additions free).
- **Bitwise**: `i32.and`, `i32.or`, `i32.xor`, `i32.shl`, `i32.shr_u`, `i32.rotl` — natural in `F_2` (boolean circuits on individual authenticated bits).
- **Comparisons**: `i32.lt_s`, `i32.lt_u`, `i32.eqz`, `i32.ge_s` — require bit decomposition (A2B), then boolean comparison circuits.

Each transition between arithmetic and boolean representation requires one A2B or B2A conversion. The costs:

1. **A2B conversion (~45 us at 200 Mbps)** is needed whenever an arithmetic value must be bit-decomposed: comparisons, shifts, bitwise ops after arithmetic.
2. **B2A conversion (~49 us at 200 Mbps)** is needed whenever a boolean result re-enters arithmetic: e.g., computing `i32.add` on a value produced by `i32.and`.
3. **The conversion cost dominates** many Wasm instruction sequences. A single `i32.add` costs ~1.6 us (one authenticated multiplication); a single `i32.lt_s` costs ~45 us (A2B) + comparison circuit + potentially B2A — roughly 25-50x more expensive. This cost ratio shapes the circuit design: minimizing domain crossings is essential.
4. **zk-edaBits can be preprocessed** independently of the program, fitting the VOLE preprocessing model. The online conversion (given a preprocessed edaBit) is cheap: one mask-and-open + one AdderModp circuit.
5. **The AdderModp circuit** for `m = 32` (i32) or `m = 64` (i64) is the per-conversion boolean cost. For `p = 2^61 - 1` with `m = 61`, this is already benchmarked. For a 32-bit field, the circuit would be smaller (~32 AND gates for the adder + conditional subtract).
6. **Matrix multiplication optimization** is less directly relevant to a general-purpose zkVM (Wasm doesn't have a matmul instruction), but the Freivalds-check technique generalizes: any time the zkVM can batch-verify a structured computation (e.g., a memory consistency check), the same random-linear-combination approach applies.
