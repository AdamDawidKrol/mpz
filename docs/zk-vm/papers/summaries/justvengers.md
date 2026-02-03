# Justvengers: Batched VOLE ZK Disjunctions in O(R+B+C) Communication

**Authors:** Yibin Yang (Georgia Institute of Technology / NTT Research)
**Venue:** TCC 2025
**ePrint:** [2025/936](https://eprint.iacr.org/2025/936)

## Role in the VOLE-ZK Design Space

Justvengers targets the **batched disjunctive statement** — R repetitions of a
1-of-B branch selection where each branch is a circuit of size C — which is the
core primitive for VOLE-based zkVM instruction dispatch. It achieves the
theoretically optimal O(R+B+C) communication by non-trivially combining
AntMan's AHE-based polynomial commitment (IT-PAC) technique with Batchman's
topology-vector membership framework. The improvement comes at the cost of
requiring additively homomorphic encryption with **linear targeted malleability**
(a non-falsifiable assumption, instantiated via BGV) and increased prover
computation versus Batchman. This makes it a theoretical advance that
establishes the communication floor for VOLE-based batched disjunctions, but
its practical advantage over Batchman depends heavily on the parameter regime.

## Key Ideas

### Problem Statement

P and V agree on B fan-in-2 circuits C_1,...,C_B over field F, each of size C
with n_in inputs and n_x multiplications. P holds R witnesses (id_j, w_j) for
j in [R] and must prove that C_{id_j}(w_j) = 0 for all j without revealing id_j
or w_j. Naive flattening yields circuit size O(RBC).

### Background: IT-MAC and IT-PAC

- **IT-MAC:** V holds global key Delta. Commitment to x is [x] = (x, m_x) on P
  side, k_x on V side, where m_x = k_x + x*Delta. Free linear homomorphism.
  Each commitment costs 1 VOLE correlation + O(1) communication.

- **IT-PAC (from AntMan):** V additionally holds secret evaluation point Lambda.
  Commitment to polynomial f(.) is [f(Lambda)], i.e., an IT-MAC of f evaluated
  at the unknown point. Binding holds because P doesn't know Lambda (until
  reveal phase). Degree-d polynomials require a one-time O(d) setup of encrypted
  powers of Lambda via AHE. After setup, each polynomial commitment costs O(1)
  communication + 1 VOLE correlation.

- **IT-PAC generation via AHE:** V sends encrypted powers
  Enc(Lambda), Enc(Lambda^2), ..., Enc(Lambda^{2R-2}) using BGV. P uses
  additive homomorphism to compute Enc(f(Lambda) - u) where [u] is a random
  VOLE correlation (one-time pad). V decrypts to learn f(Lambda) - u. Parties
  derive [f(Lambda)] = [u] + (f(Lambda) - u). The d encrypted powers can be
  reused across all polynomial commitments.

### Background: Batchman's 6-Step Framework

1. P commits to extended witness for each of R repetitions (IT-MACs)
2. P proves multiplication gates are well-formed (LPZK)
3. Parties compress each branch circuit into a topology vector of length O(C)
4. P commits to the active topology vector for each repetition
5. P proves inner product of witness and topology vector is zero per repetition
6. P proves each committed topology vector is one of B public vectors (universal
   hashing membership check)

Batchman cost: O(RC + B) communication (Steps 1,4 dominate at O(RC)).

### Justvengers: Replacing IT-MACs with IT-PACs

The core insight: Steps 1, 2, 4, 5 of Batchman have a SIMD structure — the
same circuit topology is applied R times. By encoding the R values on each wire
as a degree-(R-1) polynomial via Lagrange interpolation over fixed public points
alpha_1,...,alpha_R, P commits O(C) polynomials via IT-PACs instead of O(RC)
individual IT-MACs.

**Step 1 (Commit witnesses):** Arrange extended witnesses into a
(n_in + 3n_x) x R matrix W. Interpolate each row to get polynomials IN_k(.),
L_k(.), R_k(.), O_k(.). Commit via IT-PACs.
- Cost: **O(C)** communication + O(C) VOLE (was O(RC) in Batchman)

**Step 2 (Prove multiplications):** For each k in [n_x]: generate IT-PAC
[L_k(.)R_k(.)] via IT-PAC multiplication; prove [L_k(.)R_k(.) - O_k(.)] is a
vanishing polynomial at alpha_1,...,alpha_R.
- Cost: **O(C)** communication + O(R) batched vanishing proof + O(C) VOLE

**Step 3 (Topology vectors):** Same as Batchman. O(1) communication.
Produces B public vectors tv^{(1)},...,tv^{(B)} each of length n_in + 3n_x + 1.

**Step 4 (Commit active topology):** Arrange R active topology vectors into a
(n_in + 3n_x + 1) x R matrix TV. Interpolate rows, commit as IT-PACs
TV_k(.).
- Cost: **O(C)** communication + O(C) VOLE (was O(RC) in Batchman)

**Step 5 (Prove zero inner products):** Construct the polynomial:

    IP(.) = sum_k IN_k(.)TV_k(.) + sum_k L_k(.)TV_{n_in+k}(.)
           + sum_k R_k(.)TV_{n_in+n_x+k}(.) + sum_k O_k(.)TV_{n_in+2n_x+k}(.)
           + TV_{n_in+3n_x+1}(.)

This requires O(C) IT-PAC multiplications. Prove IP(.) is vanishing.
- Cost: **O(C)** communication + O(R) batched vanishing proof + O(C) VOLE

**Step 6 (Membership check — the non-trivial part):** P constructs a B x R
binary matrix MK where MK_{i,j} = 1 iff branch i is active in repetition j.
Interpolate rows to get MK_1(.),...,MK_B(.). Commit all via IT-PACs.

V samples challenge gamma. Parties compute public hashes:
h_i = <(1, gamma, gamma^2, ...), tv^{(i)}> for each branch i.

P proves vanishing of:
- sum_{k} gamma^{k-1} TV_k(.) - sum_i h_i MK_i(.) (topology hash matches)
- MK_i(.)(MK_i(.) - 1) for each i (binary constraint)
- sum_i MK_i(.) - 1 (exactly one branch selected per repetition)

All are free linear combinations of existing IT-PACs.
- Cost: **O(B)** communication + O(R) batched vanishing proof + O(B) VOLE

### Vanishing Polynomial Batching

All vanishing polynomial proofs across Steps 2, 5, 6 are aggregated: P samples
a random vanishing polynomial PAD(.) of degree 2R-2, V sends challenge zeta,
P reveals VP(.) = PAD(.) + sum_i zeta^i f_i(.) where f_i are the polynomials
to be proved vanishing. V checks VP(.) is vanishing. Total cost: O(R) for one
batched proof (sending 2R-1 coefficients).

### Commit-Disclose-Open Technique

Since AHE ciphertexts are generated by P before verifying V's encrypted
Lambda-powers, P commits all AHE ciphertexts via F_Com before sending.
After V reveals the AHE seed and Lambda (allowing P to verify correctness),
P opens all committed ciphertexts. This avoids requiring verifiable AHE.

### Polynomial Multiplication via IT-PAC

Given [f(.)], [g(.)], parties compute [f(.)g(.)] at O(1) communication cost by:
P computes f(X)g(X), homomorphically evaluates at Lambda using the encrypted
powers, and one-time-pads with a fresh VOLE correlation. Constraint: degree of
product <= 2R-2 (so input polynomials must each be degree < R).

### Soldering (Cross-Repetition Constraints)

To enforce e.g. IN_1(alpha_j) = O_1(alpha_{j-1}) (output of step j-1 feeds
input of step j), Justvengers uses the "sacrifice" technique: P commits random
polynomials r_1, r_2 satisfying the constraint; reveals phi*IN_1(.) + r_1(.)
and phi*O_1(.) + r_2(.) for random challenge phi; V checks the relation holds.

### Proof of Knowledge (Extractability)

IT-PAC commitments alone don't support extraction (P's polynomial is hidden
from the simulator). Fix: P additionally commits each input polynomial's R
coefficients as individual IT-MACs in Step 5 of the protocol. Since the
simulator controls VOLE, it can extract these coefficients and reconstruct
the witness. This adds O(R * n_in) overhead — negligible when n_in = O(1).

### Protocol Phases (Full UC Description)

1. **Initialization:** AHE setup (V sends 2R-2 encrypted Lambda-powers), VOLE
   pool of size R*n_in + 2B + 3n_in + 10n_x + 3
2. **Commitment (Steps 5-10):** All IT-PAC ciphertexts committed via F_Com
3. **Disclosure (Steps 11-13):** Batched vanishing proof; V reveals seed + Lambda
4. **Open (Steps 14-19):** P opens all AHE ciphertexts; IT-PACs become IT-MACs
5. **Verification (Steps 20-22):** Check vanishing polynomial at Lambda; LPZK
   for all multiplications; check input coefficients match polynomials

## Cost Model

### Asymptotic Comparison (field elements / field operations)

| Protocol      | Communication   | P Computation          | V Computation      | AHE? |
|---------------|-----------------|------------------------|--------------------| -----|
| QuickSilver   | O(RBC)          | O(RBC)                 | O(RBC)             | No   |
| Batchman      | O(RC + B)       | O(BC + RC)             | O(BC + RC)         | No   |
| AntMan        | O(BC + R)       | O(BCR log R)           | O(BC + R log R)    | Yes  |
| **Justvengers** | **O(R + B + C)** | **O(BC + (B+C)R log R)** | **O(BC + R log R)** | **Yes** |

Note: Batchman complexity incorporates the ZK SET optimization from [YH24].

### Detailed VOLE + Communication Costs

| Resource         | Count                                |
|------------------|--------------------------------------|
| VOLE correlations | R*n_in + 2B + 3n_in + 10n_x + 3    |
| AHE ciphertexts (V->P) | 2R - 2                        |
| AHE ciphertexts (P commits via F_Com) | 2B + 3n_in + 10n_x + 2 |
| F elements P->V (input coefficients)  | R * n_in              |
| F elements P->V (vanishing poly)      | 2R - 1                |
| F elements P->V (verification proofs) | n_in + 3              |
| F elements V->P (challenges)          | 5 (chi, gamma, zeta, LPZK, Lambda) |
| Rounds | O(1) constant                                         |

### Computation Breakdown

- **P's bottleneck:** NTT-based polynomial interpolation: O(B+C) polynomials
  each of degree O(R), costing O(R log R) per polynomial = O((B+C)R log R).
  Plus O(BC) for topology vector generation. Total: **O(BC + (B+C)R log R)**.

- **V's bottleneck:** O(BC) for topology vectors + O(R log R) for verifying
  the degree-O(R) vanishing polynomial VP(.) via NTT. Total: **O(BC + R log R)**.

### When Justvengers Beats Batchman (Communication)

Justvengers: O(R + B + C). Batchman: O(RC + B).

- **Justvengers wins when RC >> R + C**, i.e., when **both R and C are large**.
  Concretely: R*C >> R + B + C, which simplifies to R >> 1 and C >> 1 and
  R*C dominates.
- **Batchman wins when R is small** (few repetitions) or **C is small** (small
  branches), since Batchman avoids AHE overhead entirely.
- **Batchman always wins on prover computation:** O(BC + RC) vs
  O(BC + (B+C)R log R). The R log R factor from NTT is significant.

### Soundness Error

Union bound over all bad events:

    (6R + 2B + 2n_in + (2B+8)n_x) / |F| + eps_ro + eps_ltm + eps_cpa

where eps_ro is RO error, eps_ltm is linear targeted malleability error, and
eps_cpa is AHE CPA security error. Negligible for |F| = lambda^{omega(1)}.

## Concrete Performance

**No implementation or benchmarks are provided.** The paper is purely
theoretical. All costs are stated in field elements / field operations with no
concrete timings, wall-clock measurements, or implementation artifacts.

Relevant practical considerations:
- AHE (BGV) ciphertexts are much larger than field elements — the O(R) AHE
  ciphertexts from V and O(B+C) AHE ciphertexts from P carry substantial
  constant factors.
- NTT-friendly evaluation points (alpha chosen as roots of unity) accelerate
  interpolation in practice.
- The CheckZero optimization (via RO) compresses n zero IT-MAC proofs into
  O(lambda) bits total.
- Batchman + ZK SET is already implemented and benchmarked in prior work;
  Justvengers is not.

## Key Definitions

- **Batched disjunctive statement (F_ZK^{R,B}):** P proves knowledge of R
  witnesses (id_j, w_j) such that C_{id_j}(w_j) = 0 for all j in [R], without
  revealing id_j or w_j. B is the number of branches, C is the circuit size per
  branch, R is the number of repetitions.

- **IT-MAC ([x]_Delta):** Distributed commitment where P holds (x, m_x) and V
  holds k_x, with m_x = k_x + x*Delta. Global key Delta is uniform and unknown
  to P. Free linear homomorphism. Binding with probability 1/|F|.

- **IT-PAC ([f(.)]):** Commitment to polynomial f(.) via IT-MAC of f(Lambda),
  where Lambda is V's secret evaluation point. Binding: forging requires guessing
  Lambda (prob d/|F| for degree-d polys). Becomes IT-MAC once Lambda is revealed.

- **Topology matrix / vector:** Public matrix M encoding the linear constraints
  of circuit C_i. After V's random challenge chi, compressed to a length-(n_in +
  3n_x + 1) topology vector tv^{(i)}. The active branch satisfies
  <w_ext || 1, tv^{(id)}> = 0.

- **Vanishing polynomial:** f(X) vanishes at alpha_1,...,alpha_R iff
  f(alpha_j) = 0 for all j. If deg(f) < R and f vanishes at R points, f is
  identically zero.

- **Linear targeted malleability (AHE property):** Given Enc(m), an adversary
  can only produce ciphertexts of the form Enc(a*m + b) for known a, b — cannot
  produce ciphertexts encoding non-linear functions of m. Non-falsifiable
  assumption. Instantiated by BGV.

- **LPZK (Line-Point Zero-Knowledge):** Technique to verify n multiplication
  triples [x_k][y_k] = [z_k] with O(1) communication using the identity
  k_x*k_y + k_z*Delta = (xy - z)*Delta^2 + (...)*Delta + m_x*m_y. Batched via
  random linear combination.

- **Soldering:** Enforcing cross-repetition witness consistency (e.g., output of
  step j-1 equals input of step j). Done via the sacrifice technique with random
  polynomial padding.

## Relevance to VOLE-Based zkVM

1. **Communication floor for instruction dispatch:** Justvengers establishes
   that O(R+B+C) communication is achievable for batched disjunctions. For a
   zkVM with B instruction types, C gates per instruction, and R execution steps,
   this is the theoretical minimum (each parameter appears at most once). Any
   VOLE-based zkVM design should be aware of this bound even if it doesn't use
   Justvengers directly.

2. **AHE cost is the practical bottleneck:** The AHE requirement (BGV
   ciphertexts, linear targeted malleability assumption) adds significant constant
   factors and implementation complexity compared to Batchman's pure-VOLE
   approach. For a zkVM prioritizing engineering simplicity and concrete
   performance, Batchman's O(RC+B) with no AHE may be preferable until R*C
   becomes very large.

3. **Prover computation tradeoff:** Justvengers' prover cost
   O(BC + (B+C)R log R) is worse than Batchman's O(BC + RC) by a factor of
   ~(B/C + 1) log R. For a typical zkVM where B ~ 50-200 instructions and
   C ~ 100-1000 gates, this overhead is non-trivial. The O(R log R) NTT per
   polynomial (of which there are O(B+C)) is the dominant cost.

4. **Batchman is strictly better when R is moderate:** If R (number of CPU
   steps) is not dramatically larger than C (instruction circuit size), the RC
   term in Batchman's communication is manageable and its simpler prover wins.
   Justvengers' advantage materializes when R and C are both large — e.g.,
   R = 10^6 steps with C = 10^3 gates.

5. **Soldering mechanism is essential for zkVM:** The cross-repetition witness
   consistency (Section 4.3) via the sacrifice technique is required for any zkVM
   — the output of instruction j-1 must feed into instruction j. Justvengers
   supports this natively and the technique is general enough to apply to other
   VOLE-based constructions.

6. **IT-PAC technique is reusable:** Even without adopting Justvengers
   wholesale, the IT-PAC approach (polynomial commitments via AHE + VOLE) from
   AntMan that Justvengers builds on could be selectively applied to specific
   high-repetition subprotocols within a zkVM to reduce communication for those
   components.

7. **Topology vector framework carries over:** Justvengers inherits and
   validates Batchman's topology matrix/vector abstraction for representing
   circuit linear constraints. This is the right abstraction for instruction
   dispatch in a VOLE-based zkVM regardless of whether IT-MACs or IT-PACs are
   used for commitment.

8. **Non-falsifiable assumption is a real concern:** Linear targeted
   malleability of BGV is non-falsifiable — it cannot be disproved by an
   efficient adversary even if false. This is a strictly weaker security
   guarantee than Batchman's LPN-only assumption. A zkVM design should weigh
   this carefully.

9. **No implementation exists:** Unlike Batchman (which has been implemented
   and benchmarked), Justvengers is purely theoretical. The concrete constant
   factors in AHE ciphertext sizes, NTT overhead, and the commit-disclose-open
   protocol flow are unknown. Any adoption would require significant
   implementation effort and benchmarking.

10. **Hybrid approach possible:** A practical zkVM could use Batchman for
    instruction dispatch (where R*C is manageable) and selectively apply
    IT-PAC/AntMan techniques for specific high-repetition SIMD subcircuits within
    the VM (e.g., memory consistency checks) where the communication savings
    justify the AHE overhead.
