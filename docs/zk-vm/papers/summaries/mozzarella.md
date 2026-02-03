# Mozzarella: Efficient Vector-OLE and Zero-Knowledge Proofs Over Z_{2^k}

**Authors**: Carsten Baum, Lennart Braun, Alexander Munch-Hansen, Peter Scholl
**Venue**: CRYPTO 2022 | **ePrint**: [2022/819](https://eprint.iacr.org/2022/819)

## Role in the VOLE-ZK Design Space

This is arguably the most important paper for a Wasm zkVM. All prior VOLE-ZK works (Wolverine, QuickSilver, Mac'n'Cheese, LPZK) operate over fields — F_2 or F_p for prime p. But Wasm operates on Z_{2^32} and Z_{2^64}, which are rings, not fields. Emulating ring arithmetic in a prime field requires (1) a prime p > 2^{2k} to avoid wraparound during multiplication, (2) explicit modular reduction circuits to prove each operation correctly wraps mod 2^k, and (3) range proofs or truncation protocols per multiplication. Mozzarella eliminates all of this by providing both a VOLE extension protocol and a ZK proof system (QuarkSilver) that work natively over Z_{2^k}.

## Key Ideas

### The Core Problem: Non-Invertibility in Rings

Over a field, the IT-MAC `M[x] = K[x] + Delta * x` is unforgeable because any forged value `x' != x` yields `M[x'] - M[x] = Delta * (x' - x)`, and since `x' - x` is invertible the prover recovers `Delta`, which happens with probability `1/|F|`. Over Z_{2^k} this breaks: if `x' - x = 2^{k-1}`, the forger only needs 1 bit of `Delta`, succeeding with probability 1/2.

The SPDZ2k fix (Cramer et al. 2018) extends the modulus: work in Z_{2^l} with `l = k + 2s` for statistical security parameter s. The lower s bits of `Delta` still protect the lower k bits of the committed value. Both the VOLE extension and QuarkSilver adopt this strategy.

### VOLE Extension Over Z_{2^k}

The paper builds a maliciously secure VOLE extension protocol that turns a short seed-VOLE over Z_{2^l} into a much longer pseudorandom VOLE, following the Wolverine/Boyle et al. blueprint:

1. **Single-point VOLE**: Generate a VOLE correlation `w = Delta * u + v` where u has exactly one non-zero (invertible) entry. Built from a GGM-tree puncturable PRF: the receiver expands a PRF key into a binary tree with 2n leaves, and via log(n) OTs the sender learns all leaves except the one at position alpha. Two consistency checks ensure active security:
   - **Hash check**: Receiver sends a universal-hash of the "right child" leaves. Sender verifies these fix a unique tree structure. This works over rings identically to fields.
   - **Binary linear combination check**: Instead of random coefficients from the large ring (which fail due to zero divisors), coefficients are drawn from {0,1}. This only has soundness 1/2, but the functionality is relaxed to allow the receiver to learn the hidden index alpha with probability 1/2 — modeled as leakage in the ideal functionality.

2. **VOLE extension via LPN**: Run t single-point VOLEs of length n/t, concatenate into a weight-t VOLE of length n, then apply the primal LPN assumption (public 10-local linear code matrix A over Z_{2^l}) to obtain pseudorandom VOLE. First m outputs are reserved as seed for the next iteration (Wolverine-style bootstrapping).

**Key design choice**: The paper uses primal LPN with regular noise (one non-zero entry per block of size n/t), not dual LPN. Dual LPN achieves lower communication but relies on structured codes that don't readily adapt to rings.

**Ring-specific LPN hardening**: Non-zero entries in both the error vector and the sparse matrix A are sampled from Z*_{2^l} (odd integers) to prevent a reduction attack where reducing mod 2 halves the effective noise weight.

### QuarkSilver: QuickSilver Generalized to Z_{2^k}

QuarkSilver is a commit-and-prove ZK protocol over Z_{2^k}. Like QuickSilver, it uses VOLE-based IT-MACs as commitments and verifies multiplications via a quadratic check on the MAC key Delta.

**Commitments**: A commitment `[x]` to `x in Z_{2^k}` consists of:
- Prover holds: `x_tilde in Z_{2^l}` (with `x_tilde = x mod 2^k`) and `M[x] in Z_{2^l}`
- Verifier holds: `Delta in Z_{2^s}` and `K[x] in Z_{2^l}`
- Relation: `K[x] = M[x] + x_tilde * Delta (mod 2^l)`

These are linearly homomorphic: `[a*x + b]` from `[x]` requires no interaction.

**Multiplication check**: For committed values `[w_alpha], [w_beta], [w_gamma]` with `w_gamma = w_alpha * w_beta mod 2^k`, the parties locally compute:
- Prover: `A_{0,i} = M[w_alpha] * M[w_beta]` and `A_{1,i} = w_tilde_alpha * M[w_beta] + w_tilde_beta * M[w_alpha] - M[w_gamma]`
- Verifier: `B_i = K[w_alpha] * K[w_beta] - Delta * K[w_gamma]`
- Relation: `B_i = A_{0,i} + A_{1,i} * Delta` iff the multiplication is correct.

**Batched check**: All t multiplication checks are aggregated: verifier samples `chi in Z_{2^s}^t`, prover sends `U = sum(chi_i * A_{0,i}) + A*_0` and `V = sum(chi_i * A_{1,i}) + A*_1`, verifier checks `W = U + V * Delta`. A cheating prover must find a root of a quadratic polynomial in Delta mod 2^l.

**The ring-specific difficulty**: Over a field, `a*X^2 + b*X + c = 0` has at most 2 roots, giving soundness `2/|F|`. Over Z_{2^k}, there can be up to `2^{3k/4}` roots (e.g., when `a = 2^{k/2}`, every multiple of `2^{k/4}` is a root). The paper proves (Lemma 11) that for `l - r > s'`, the number of roots in `{0, ..., 2^s - 1}` is at most `2^{max((2s-s')/2, 1)}`. For the batched setting (Theorem 12, Corollary 13), choosing `s = sigma + log(sigma) + 3` and `l = k + 2s` achieves `2^{-sigma}` soundness.

**Protocol flow** (Figure 7):
1. **Preprocessing** (input-independent): Init VOLE, generate `n + t + 2` random authenticated values.
2. **Online — Inputs**: For each input `w_i`, prover sends `delta_i = w_i - mu_tilde_i`. Both compute `[w_i] = [mu_i] + delta_i`.
3. **Online — Circuit evaluation**: Addition gates are free (linear homomorphism). For each multiplication gate, prover sends `d_i = w_alpha * w_beta - nu_tilde_i`, both compute `[w_gamma] = [nu_i] + d_i`.
4. **Online — Batch check**: Verifier samples random chi, prover sends aggregated (U, V), verifier checks `W = U + V * Delta mod 2^l`.
5. **Output verification**: Blinded opening of the output wire commitment.

### Ring Size Requirements

For a circuit over Z_{2^k} with statistical security sigma:

| sigma | k  | s  | l (ring size) |
|-------|----|----|---------------|
| 40    | 32 | 49 | 130           |
| 40    | 64 | 49 | 162           |
| 80    | 32 | 90 | 212           |
| 80    | 64 | 90 | 244           |

The overhead factor `l/k` ranges from ~2.5x (k=64, sigma=40) to ~6.6x (k=32, sigma=80). This is the cost of working over a ring: the extended modulus protects integrity of the lower k bits.

## Cost Model

| Resource | Cost |
|----------|------|
| VOLE communication | ~1 bit/VOLE (amortized) |
| Commitment to input wire | 1 VOLE + l bits (prover sends delta_i) |
| Addition gate | Free (no communication) |
| Multiplication gate | 1 VOLE + l bits (prover sends d_i) |
| Batch multiplication check | 2 * l bits total (U, V) + t * s bits (chi) |
| Output verification | 2 * l bits (blinded opening) |

**Amortized cost per multiplication**: ~l bits communication (dominated by the `d_i` value). For k=64, sigma=40: 162 bits/mult. For comparison, QuickSilver over F_{2^61-1} costs 61 bits/mult — about 2.7x less communication, but QuickSilver cannot do ring arithmetic natively.

## Concrete Performance

All benchmarks: Intel Core i9-7960X, 10 Gbps LAN (0.25ms RTT), kappa=128.

### VOLE Generation (Table 1)

| Ring size l | LAN (ns/VOLE) | WAN (ns/VOLE) | Comm (bits/VOLE) |
|-------------|---------------|---------------|------------------|
| 64          | 20 - 28       | 46 - 191      | 0.95 - 1.39      |
| 104         | 33 - 41       | 59 - 187      | 1.00 - 1.46      |
| 144         | 47 - 55       | 75 - 213      | 1.05 - 1.53      |
| 244         | 77 - 81       | 103 - 255     | 1.10 - 1.60      |

Ranges correspond to batch sizes 10^8 (faster) to 10^7 (slower). Rate: **13-50M VOLEs/sec** in LAN depending on ring size.

**vs. Wolverine** (Table 2, at 50+ Mbps, ~10^7 VOLEs): Mozzarella at l=64 achieves ~50-55 ns/VOLE vs. Wolverine's ~85 ns/VOLE over F_{2^61-1}. For l <= 128, Mozzarella matches or beats Wolverine. Computation becomes the bottleneck above 100 Mbps.

### QuarkSilver ZK (Table 3)

| sigma | l   | LAN (ns/mult) | WAN (ns/mult) | Comm (bits/mult) |
|-------|-----|---------------|---------------|------------------|
| 40    | 162 | 770           | 2,405          | 193.5            |
| 80    | 244 | 848           | 3,165          | 257.5            |

Rate: **1.3M 64-bit multiplications/sec** (LAN, sigma=40), or **1.2M/sec** (sigma=80). Cost breakdown: ~86% of time spent in the `mult` step (committing outputs), ~10% VOLE generation, ~4% batch check.

**vs. QuickSilver**: Single-threaded QuickSilver does 4.8M mults/sec over F_{2^61-1}, a 5.3x gap. This comes from: (1) Z_{2^162} elements require three 64-bit limbs vs. one for F_{2^61-1} — 3x more communication and 2.1-2.5x slower arithmetic; (2) F_{2^61-1} benefits from Mersenne-prime-specific optimizations and AVX vectorization. However, this is a misleading comparison: emulating Z_{2^64} arithmetic in a prime field would require p > 2^{128}, eliminating Mersenne prime advantages and adding reduction/range-proof overhead per operation.

## Key Definitions

- **Ring-VOLE correlation**: Sender holds `(u, w) in Z_{2^l}^n x Z_{2^l}^n`, Receiver holds `(Delta, v) in Z_{2^s} x Z_{2^l}^n`, satisfying `w = Delta * u + v mod 2^l`. The key `Delta` is sampled from the smaller range `Z_{2^s}` (not `Z_{2^l}`).
- **`[x]`** — Commitment over Z_{2^k}: Prover holds `(x_tilde, M[x])`, Verifier holds `(Delta, K[x])`, with `K[x] = M[x] + x_tilde * Delta mod 2^l` and `x_tilde = x mod 2^k`. The upper `l - k` bits of `x_tilde` are not meaningful (masked during opening).
- **Blinded opening**: To open `[x]` without leaking the upper bits, compute `[z] = [x] + 2^k * [pi]` for random `[pi]`, then open z. The verifier checks `z = 1 mod 2^k` for the output wire.
- **Single-point VOLE**: VOLE where `u` has exactly one non-zero (invertible mod 2^l) entry. The leaky variant allows the receiver to learn the hidden index with probability 1/2.
- **Leaky regular LPN**: Primal LPN with regular noise (one non-zero per block), where up to sigma noise positions may be leaked to the adversary. Parameters are chosen so the reduced LPN instance (after removing leaked coordinates) remains hard.
- **Statistical security parameter sigma vs. s and l**: `s = sigma + log(sigma) + 3` (size of Delta's range), `l = k + 2s` (total ring size). The factor 2 in `2s` comes from the quadratic check in QuarkSilver.

## Relevance to VOLE-Based zkVM

**CRITICAL**. Wasm's `i32` and `i64` types are Z_{2^32} and Z_{2^64}. With Mozzarella:

1. **Native ring arithmetic**: `i32.add`, `i32.mul`, `i32.sub`, `i64.add`, `i64.mul`, `i64.sub` all correspond directly to addition and multiplication in Z_{2^32} or Z_{2^64}. No field embedding, no reduction circuits, no range proofs per operation. Addition is free; multiplication costs 1 VOLE + l bits.

2. **Elimination of field-embedding overhead**: In a prime field approach, each 64-bit multiplication requires: (a) a prime p >= 2^{128} to hold the product, (b) proving correct reduction mod 2^{64}, (c) range proofs to ensure values remain in [0, 2^{64}). Mozzarella avoids all three — the wraparound semantics of Z_{2^k} are baked into the algebraic structure.

3. **Concrete cost for Wasm operations**: At sigma=40 and k=64, each multiplication gate costs 162 bits of communication and ~770ns. The VOLE-generation amortized overhead is ~1 bit/VOLE. These are the direct costs of a single Wasm `i64.mul` in ZK.

4. **The paper operates in the same algebraic domain as Wasm**: This is the fundamental point. A zkVM for Wasm needs to prove statements about computations that happen in Z_{2^32} and Z_{2^64}. Mozzarella's ZK system proves circuit satisfiability over exactly these rings. There is no impedance mismatch.

5. **Mixed-width support**: Different ring sizes can coexist. Z_{2^32} operations (i32) and Z_{2^64} operations (i64) can each use their natural ring, with the VOLE infrastructure parameterized accordingly. The same VOLE extension protocol works for any l.
