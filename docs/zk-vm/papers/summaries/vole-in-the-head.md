# Publicly Verifiable Zero-Knowledge and Post-Quantum Signatures From VOLE-in-the-Head

**Authors**: Carsten Baum, Lennart Braun, Cyprien Delpech de Saint Guilhem, Michael Klooß, Emmanuela Orsini, Lawrence Roy, Peter Scholl
**Venue**: CRYPTO 2023 | **ePrint**: [2023/996](https://eprint.iacr.org/2023/996)

## Role in the VOLE-ZK Design Space

Standard VOLE-based ZK protocols (Wolverine, QuickSilver, Mac'n'Cheese) are **designated-verifier only**: the verifier must hold a secret `Delta` to ensure soundness. This paper presents a compiler that transforms these designated-verifier protocols into **publicly verifiable, non-interactive** proofs using a technique called *VOLE-in-the-head*.

The core idea: the prover simulates both sides of a VOLE correlation internally (in its "head"), commits to the VOLE messages via a seed tree, and the verifier opens a subset to check consistency. This is analogous to MPC-in-the-head but uses the simpler VOLE-based multiplication checks instead of full MPC circuit evaluation, yielding smaller and faster proofs. The resulting NIZK is applied to build FAEST, a post-quantum signature scheme from AES.

## Key Ideas

### The O2C Compiler: From OT-Based ZK to Public Verifiability

The transformation works on any ZK protocol in the `F_{OT}`-hybrid model where:
1. The prover plays the OT **sender** role.
2. The verifier is **OT-admissible**: its actions don't depend on intermediate OT outputs (all OT queries are deferred to a final verification step).

**Mechanism**: Replace each (N-1)-out-of-N random OT instance with a **vector commitment** (VC). The prover commits to N pseudorandom strings via a GGM seed tree. At verification time, the verifier reveals its choice index and the prover opens all-but-one of the committed values (opening cost: `O(log N)` communication via the punctured tree path).

Any public-coin verifier is automatically OT-admissible, so the compiler preserves the public-coin property. The compiled protocol `O2C[Pi]` inherits:
- **Knowledge soundness** from the original protocol (with negligible loss from VC extractability).
- **SHVZK** from the original simulator.

### VOLE-in-the-Head: Simulating VOLE from All-But-One OT

A single (N-1)-out-of-N OT with `N = p^k` can produce a VOLE correlation. Let `t_0, ..., t_{N-1}` be the prover's OT messages and `Delta` be the verifier's choice (an element of `F_{p^k}`). The verifier learns `t_x` for all `x != Delta`, and both parties can compute:

```
q = sum_{x != Delta} t_x * (Delta - x) = u * Delta + v
```

where `u = sum t_x` and `v = -sum t_x * x`. The prover knows `(u, v)`, the verifier knows `(Delta, q)` — this is exactly a VOLE correlation.

**Key limitation**: This is only efficient when `p^k` is small (the OT has `p^k` branches). For large extension fields, `Delta` is restricted to a small subset `S_Delta` with each coordinate independently sampled from a polynomial-sized set.

### Handling Large Fields: Subspace VOLE and Code-Switching

For large fields `F_p` (where `p >= 2^kappa`), sampling `Delta` from a small subset gives poor soundness. The fix is **subspace VOLE**: the prover's input `u` is encoded under a linear code `C`, so the VOLE correlation becomes:

```
q = C(u) * Delta + v    (component-wise product)
```

If `C` has minimum distance `d_C` and the `n_C` entries of `Delta` are independent, a malicious prover must guess `d_C` entries of `Delta` to cheat — amplifying soundness from `|S_Delta|^{-1}` to `|S_Delta|^{-d_C}`.

Since subspace VOLE is incompatible with the standard VOLE used in QuickSilver-style checks, the paper introduces a **code-switching** technique: commit the witness under subspace VOLE, then translate to standard VOLE-in-the-head for the constraint check. This adds a ~2x communication overhead compared to designated-verifier VOLE-ZK.

### Fiat-Shamir via Round-by-Round Soundness

The compiled protocol is made non-interactive via Fiat-Shamir. Soundness is analyzed by reinterpreting the protocol as an IOP (the prover's OT inputs are "PCP oracles", the verifier's OT choices are "queries") and applying **round-by-round (RBR) knowledge soundness**. The knowledge error of the resulting NIZK is:

```
mu * (Q_FS + Q_Verify) * kappa  +  M * Q_Verify * AdvEB^{VC}  +  AdvDist^{Setup}
```

where `mu` is the number of rounds, `Q_FS` is the number of random oracle queries, `kappa` is the per-round knowledge error, and `M` bounds the number of VC commitments.

## Cost Model

### Communication per Multiplication Gate

| Protocol | Field | Model | Comm/gate (kappa=128) | Comm/gate (kappa=40) | Assumption |
|----------|-------|-------|-----------------------|----------------------|------------|
| VOLE-ZK [YSWW21] | `F_2` | Designated-verifier | 1 bit | 1 bit | LPN |
| VOLE-ZK [DIO21] | `F_p` | Designated-verifier | 1 elem | 1 elem | LPN |
| Limbo [dOT21] | `F_2` | Public-coin | 42 bits | 11 bits | Hash |
| Limbo [dOT21] | `F_p` | Public-coin | 40 elems | 11 elems | Hash |
| **VitH (this work)** | **`F_2`** | **Public-coin** | **16 bits** | **5 bits** | **Hash** |
| **VitH (this work)** | **`F_p`** | **Public-coin** | **3 elems** | **2 elems** | **Hash** |

For `2^20` mult gates at 128-bit security: VitH over `F_2` costs 16 bits/gate; over large `F_p` costs ~3 field elements/gate (vs. 1 for designated-verifier VOLE-ZK).

### Proof Size Breakdown

For the large-field protocol (`Pi_{2D-LC}`), total communication beyond sVOLE setup is:

```
(2*ell + 2) * k_C * log(p)  +  n_C * log(p)
```

This is roughly **2x** the cost of QuickSilver in designated-verifier mode. The sVOLE setup contributes `ell * (n_C - k_C)` field elements, which is sublinear in witness length `ell * k_C` if the code `C` has good rate.

For the small-field protocol (`Pi_{2D-Rep}` with repetition code `[tau, 1, tau]`):

```
ell * log(p)  +  2 * r * tau * log(p)  +  CommCost_{sVOLE}
```

## Concrete Performance

### FAEST Signature Sizes (AES-128, lambda=128)

| Field size `q` | Sign (ms) | Verify (ms) | Signature (bytes) |
|---------------|-----------|-------------|-------------------|
| `2^7` | 2.63 | 2.43 | 7,506 |
| `2^8` | 2.28 | 2.11 | 6,583 |
| `2^9` | 4.30 | 3.95 | 6,435 |
| `2^10` | 6.45 | 5.94 | 5,803 |
| `2^11` | 11.05 | 10.18 | 5,559 |

### Comparison to Other PQ Schemes (128-bit security)

| Scheme | Sign (ms) | Verify (ms) | Size (B) | Assumption |
|--------|-----------|-------------|----------|------------|
| SPHINCS+-SHA2 (fast) | 4.40 | 0.40 | 17,088 | Hash |
| SPHINCS+-SHA2 (short) | 88.21 | 0.15 | 7,856 | Hash |
| Helium+AES (fast) | 9.87 | 9.60 | 11,420 | Hash/AES |
| Limbo (fast) | 2.61 | 2.25 | 23,264 | Hash/AES |
| BN++Rain4 (short) | 4.79 | 4.53 | 4,992 | Rain4 |
| **FAEST (fast, q=2^8)** | **2.28** | **2.11** | **6,583** | **Hash/AES** |
| **FAEST (short, q=2^11)** | **11.05** | **10.18** | **5,559** | **Hash/AES** |

FAEST is the first AES-based signature smaller than SPHINCS+ (short). Signing is 8-40x faster than SPHINCS+ (short), but verification is slower (2.1 ms vs 0.15 ms).

## Key Definitions

- **VOLE-in-the-head** — Technique where the prover internally simulates both sides of a VOLE correlation, commits to the sender's messages via a seed tree, and the verifier opens all-but-one to reconstruct its view. Replaces the interactive VOLE setup with a commitment-based non-interactive one.
- **O2C compiler** (`O2C[Pi]`) — Generic compiler that replaces `F_{OT}`-hybrid calls with vector commitments. Input: OT-admissible ZK protocol. Output: publicly verifiable protocol in the CRS+RO model.
- **OT-admissible** — Property of a ZK protocol where the verifier's actions split into two phases: (1) interact without reading OT outputs, (2) read all OT outputs and decide. Allows deferring all OT choice queries to the end.
- **Subspace VOLE** — VOLE variant where the prover's input `u` is encoded as `C(u)` under a linear code `C` before the VOLE correlation: `q = C(u) * Delta + v`. Amplifies soundness from `|S_Delta|^{-1}` to `|S_Delta|^{-d_C}` where `d_C` is the code's minimum distance.
- **Code-switching** — Technique to translate a commitment under subspace VOLE (with a general code `C`) into a standard VOLE-in-the-head commitment. The prover sends `S = R + U * Delta'` and the verifier checks this against the original subspace VOLE output. Adds ~2x overhead.
- **Round-by-round (RBR) knowledge soundness** — Soundness notion for IOPs where a "bad challenge" function tracks whether the transcript has been corrupted. If no bad challenge occurs and the verifier accepts, a witness can be extracted. Used to analyze Fiat-Shamir security with tight bounds.
- **Tree-PRG vector commitment** (`VC_{GGM}`) — Commitment to N random seeds using a GGM tree of PRGs. Supports all-but-one openings with `O(log N)` communication (reveal the co-path). Provides extractable-binding (via hash extraction) and selective hiding (via PRG security).

## Relevance to VOLE-Based zkVM

1. **Public verifiability path**: If the VC spec ever requires non-interactive, publicly verifiable proofs (e.g., for on-chain verification or multi-verifier settings), this paper provides the transformation. The core VOLE-ZK protocols (QuickSilver-style constraint checks, IT-MACs) remain unchanged — VitH is applied *on top* as a compiler.
2. **Same protocol internals**: The underlying ZK protocol is essentially QuickSilver with degree-2 constraint checks. The only change is that VOLE correlations come from seed-tree commitments instead of LPN-based VOLE extension. This means the same circuit/constraint representation works in both settings.
3. **Cost of public verifiability**: The overhead is concrete and well-characterized — roughly 2x communication over designated-verifier for large fields, 5-16x for binary (depending on security parameter). This is much better than prior MPCitH approaches (Limbo: 11-42 elements/gate vs VitH: 2-5).
4. **Not needed for two-party setting**: In the current VC design (designated prover and verifier), interactive VOLE-ZK is strictly more efficient. VitH is relevant only if the trust model changes to require public verifiability or non-interactivity beyond Fiat-Shamir on the interactive protocol.
5. **Computational overhead is low**: The prover runtime is comparable to designated-verifier VOLE-ZK (the seed-tree expansion replaces LPN-based VOLE, which is computationally *cheaper*). The main cost is proof size, not prover time.
