# Improving Line-Point Zero Knowledge: Two Multiplications for the Price of One

**Authors**: Samuel Dittmer, Yuval Ishai, Steve Lu, Rafail Ostrovsky
**Venue**: ACM CCS 2022 | **ePrint**: [2022/552](https://eprint.iacr.org/2022/552)

## Role in the VOLE-ZK Design Space

This paper halves the online proof size of LPZK. The original LPZK (LPZKv1) required ~2 field elements per multiplication gate in the IT setting, or ~1 in ROM. This paper's IT-LPZKv2 and ROM-LPZKv2 reduce those to ~1 and ~1/2, respectively. ROM-LPZKv2 is the first practical VOLE-based NIZK to break the 1-element-per-multiplication barrier. The improvement comes from introducing *additional structure* in the correlated randomness (quadratically certified VOLE), not from changing the verification approach.

## Key Ideas

### From VOLE to qVOLE

In standard VOLE, P holds random `(a', b')` and V holds `v' = a'*alpha + b'`. The paper defines **quadratically certified VOLE (qVOLE)**: a VOLE instance where certain entries of `a'` are constrained to satisfy quadratic relations `f_i(a') = a'_i` (the degree-2 parts of the circuit's gate polynomials). This moves the computation of products `a_i * a_j` out of the online phase and into preprocessing.

Two realizations of qVOLE are given:
- **From plain VOLE**: bootstraps off an existing VOLE instance with linear additional offline communication (effectively shifts ~50% of LPZKv1's communication to offline).
- **From ring-LPN**: sublinear offline communication via pseudorandom correlation generators. Efficient in the SIMD setting (many instances of the same circuit) or circuits with repeated subcircuits (e.g., hash trees).

### Swapping the Roles of a and b

In LPZKv1, the witness is encoded in vector `a` (the slope) and `b` serves as a random mask. LPZKv2 reverses this: **`b` holds the witness and wire values, `a` holds the masks**. From the prover's perspective, the two choices are identical, but the verifier saves one multiplication by `alpha` per gate because it no longer needs to extract the witness component from the slope.

### Half-Free Multiplication (ROM Variant)

The ROM protocol divides wires into two types via a **red/blue coloring**:
- **Red wires**: `a_i` is determined purely by the correlated randomness (independent of witness).
- **Blue wires**: `a_i` depends on both the correlated randomness and the prover's input.

The key observation: the product of two red-wire polynomials, combined with an appropriately constructed qVOLE entry, yields the output wire **with zero communication**. The output wire becomes blue, so the technique doesn't apply to blue*blue or blue*red products. For **layered circuits**, alternating layers can be colored red and blue, so exactly half the multiplication gates are "free" (no communication). For general circuits, a greedy coloring still achieves ~38% reduction on random circuits.

The only communication required is:
1. Adjusting constant terms of red wires (the `b^1 - b'^1` corrections).
2. A batched proof of polynomial degeneracy (`F_PoPD`) to verify consistency.

### Batched Proofs of Polynomial Degeneracy

Both protocols rely on batched proof-of-polynomial-degeneracy functionalities:
- **`F^{k,F}_PoPD`** (ROM): P and V hold shares of degree-k polynomials; P proves the leading coefficient is zero. Realized with `k*r` field elements of communication, where `r` controls soundness.
- **`F^{1,F}_PoLD`** (IT): Special case for linear polynomials. Realized with 1 field element per batch of `t` entries (a simple product check), giving information-theoretic security.

## Cost Model

### Communication per Multiplication Gate

| Protocol | Comm. (field elts) | Prover mults | Prover adds | Hash calls | Prover speed |
|---|---|---|---|---|---|
| Plain circuit eval. | -- | 1 | 0 | 0 | 49.6 M/sec |
| Mac'n'Cheese | 3 | 23 | 15 | 3 | 3.6 M/sec |
| Wolverine | 2 | 13 | 16 | 3 | 0.96 M/sec |
| IT-LPZKv1 | 2 + 1/t | 4 | 7 | 0 | 19.6 M/sec |
| QuickSilver | 1 | 15 | 13 | <1 | 7.8 M/sec |
| **IT-LPZKv2** | **1 + 1/t** | **3** | **5** | **0** | **21.8 M/sec** |
| **ROM-LPZKv2** | **1/2** | **8.5** | **8.5** | **1** | **9.8 M/sec** |

All figures use `F_{2^61 - 1}`, `kappa = 128`, single-threaded. Parameter `t` in IT-LPZK gives soundness error `2t/(|F| - 1)`.

Key observations:
- IT-LPZKv2 is the fastest prover among all VOLE-ZK protocols (21.8 M gates/sec), within 2-3x of plain circuit evaluation.
- ROM-LPZKv2 breaks the 1-element barrier at 1/2 element per gate for layered circuits.
- Addition gates remain free (2 additions only, no communication).

### Comparison with Non-VOLE Protocols (1M gates, `F_{2^61-1}`)

| Protocol | Comm. | Prover time | Verifier time | Bottleneck network |
|---|---|---|---|---|
| Groth16 | 192 B | 21 s | <2 ms | 0.009 kBps |
| Virgo | 271 kB | 478 ms | 12.4 ms | 567 kBps |
| Cerberus | 2.8 MB | 2.17 s | 148 ms | 1.29 MBps |
| QuickSilver | 8 MB | 128 ms | <128 ms | 62.5 MBps |
| **IT-LPZKv2** | **8 MB** | **45.8 ms** | **27.4 ms** | **174.7 MBps** |
| **ROM-LPZKv2** | **4 MB** | **102.5 ms** | **88.1 ms** | **39.0 MBps** |

"Bottleneck network" = minimum network speed at which prover computation becomes the bottleneck rather than communication. For networks >= 200 MBps, IT-LPZKv2 gives the fastest online ZK.

## Concrete Performance

### Prover and Verifier Online Runtimes (ms / million mult gates)

| Protocol | Field | Prover | Verifier | V/P ratio |
|---|---|---|---|---|
| IT-LPZKv1 | `F_{2^61-1}` | 51.1 | 30.3 | 0.59 |
| IT-LPZKv2 | `F_{2^61-1}` | 45.8 | 27.4 | 0.60 |
| IT-LPZKv1 | P-384 | 3050 | 1813 | 0.59 |
| IT-LPZKv2 | P-384 | 2556 | 1511 | 0.59 |
| ROM-LPZKv2 | `F_{2^61-1}` | 102.5 | 88.1 | 0.86 |

- IT-LPZKv2 gives ~10% prover speedup over IT-LPZKv1 on Mersenne primes (expected ~30% as field multiplication cost dominates; ~16% already observed on P-384).
- Verifier is consistently ~60% of prover time for IT variants (dominated by a single circuit evaluation vs. two for the prover). ROM ratio is ~85% due to shared hashing cost.
- All benchmarks are single-threaded on AWS m5.2xlarge, in the preprocessing model (qVOLE already generated).

## Key Definitions

- **qVOLE (Quadratically Certified VOLE)** -- A VOLE instance `(a', b', v' = a'*alpha + b')` where certain entries of `a'` are constrained by quadratic relations `f_i(a') = a'_i` derived from the circuit's degree-2 gates. `alpha != 0` is required (a revised fix from the original publication, addressing an attack from [Oechsner, Pereira, Scholl 2025]).
- **Certified LPZK** -- An LPZK proof system where the prover outputs `(a, b)` with `a` satisfying a set of quadratic relations, and the verifier gets a guarantee (certification) that these hold. The compiler from certified LPZK to NIZK over qVOLE requires communication of `n - n'` field elements.
- **Red/blue wire coloring** -- A classification of circuit wires for the ROM protocol. Red wires have `a_i` determined purely by correlated randomness; blue wires have `a_i` depending on the witness. A gate with all-red inputs can produce its output with zero communication (output becomes blue). This is what enables the 1/2-element-per-gate rate.
- **`F^{k,F}_PoPD` (Proof of Polynomial Degeneracy)** -- Functionality where P proves that the leading coefficient of batched degree-k polynomials is zero. ROM-based, `k*r` elements communication.
- **`F^{1,F}_PoLD` (Proof of Linear Degeneracy)** -- IT variant of the above for degree-1 polynomials. 1 element communication per batch of `t` entries, using a product check.

## Relevance to VOLE-Based zkVM

1. **Directly halves per-instruction communication**: The zkVM circuit is dominated by multiplication gates for constraint checks. Cutting from ~1 to ~1/2 field element per gate (ROM) or from ~2 to ~1 (IT) translates directly to 2x bandwidth savings.
2. **Fastest known prover for VOLE-ZK**: IT-LPZKv2 at 21.8M gates/sec is within 2.3x of plain circuit evaluation, making it the best choice when the network is fast and prover computation is the bottleneck.
3. **Online/offline split aligns with preprocessing**: qVOLE is generated offline from VOLE. The zkVM already assumes a preprocessing model with VOLE correlations; adding the quadratic certification step to preprocessing is a natural extension.
4. **Layered circuit structure is exploitable**: The ROM variant's half-free multiplication applies to layered circuits. If the zkVM's constraint circuit can be organized into layers (which is natural for instruction-by-instruction execution), the full 50% communication savings applies.
5. **SIMD-friendly**: The ring-LPN qVOLE realization is efficient when proving many instances of the same subcircuit, which matches the zkVM's execution model (the same instruction constraint circuit is applied to each step).
