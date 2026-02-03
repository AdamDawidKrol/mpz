# LogRobin++: Optimizing Proofs of Disjunctive Statements in VOLE-Based ZK

**Authors:** Carmit Hazay, David Heath, Vladimir Kolesnikov, Muthuramakrishnan Venkitasubramaniam, Yibin Yang
**Venue:** ASIACRYPT 2024
**ePrint:** [2024/1427](https://eprint.iacr.org/2024/1427)
**Code:** <https://github.com/gconeice/logrobinplus>

---

## Role in the VOLE-ZK Design Space

LogRobin++ is the state-of-the-art protocol for proving disjunctive statements (1-out-of-B branch selection) in VOLE-based ZK. It directly targets the zkVM use-case where a CPU step is a disjunction over the instruction set: the prover knows which instruction was executed but must hide the branch identity. LogRobin++ achieves `(n_in + n_x)|F| + O(rho * log B)` bits of communication in O(1) rounds in the VOLE-hybrid model, simultaneously beating Robin's O(B) additive overhead and Mac'n'Cheese's O(log B) round count. It is information-theoretically secure (UC) against static unbounded adversaries in the VOLE-hybrid model.

---

## Key Ideas

### Problem Setup

P and V agree on B fan-in-2 circuits `C_0, ..., C_{B-1}` over field F_p, each with `n_in` inputs, `n_x` multiplications, 1 output. P holds witness `(id, w)` such that `C_id(w) = 0`; neither `id` nor `w` is revealed. Let `rho = max{log|F|, lambda}`.

### Background: Single-Circuit VOLE-based ZK (LPZK)

For a single circuit C, P commits to inputs `w` and multiplication outputs `o` via IT-MACs over F_{p^q} (generated from VOLE correlations). P and V evaluate C gate-by-gate over IT-MACs: addition gates are local (linear homomorphism), multiplication gates consume a committed output. For each multiplication triple `[x], [y], [z]`, the LPZK check exploits:

```
k_x * k_y - k_z * Delta = (xy - z) * Delta^2 + (x*m_y + y*m_x - m_z) * Delta + m_x * m_y
```

If `xy = z`, the leading coefficient vanishes and V sees a degree-1 polynomial in Delta. Otherwise V sees degree-2, and P cannot forge because Delta is secret. Batched LPZK: V issues random challenges gamma to linearly combine all triples; P sends one randomized pair of coefficients. Cost: `(n_in + n_x) log p + O(q log p)` bits, O(1) rounds.

### Background: Robin (Prior SOTA)

Robin has P commit to `w, l, r, o` (inputs + all three wires per multiplication gate on the active branch). Multiplication correctness is checked via batched LPZK on `(l, r, o)`. Then for each branch `C_i`, P and V evaluate `C_i` over committed `[w]` and `[o]`, reusing the committed values. For the active branch `id`, the difference vectors `[l - l^(id)]`, `[r - r^(id)]`, `[res^(id)]` are all zero. V issues a random challenge vector gamma of length `2n_x + 1`, parties compute an inner product per branch, then P proves one of B inner products is zero by showing their product is zero. Cost: `(n_in + 3n_x) log p + O(rho * B)` bits, O(1) rounds. Communication is O(B) in the branching overhead and 3n_x in per-gate cost.

### Stepping Stone 1: LogRobin -- O(log B) Zero-Membership Proof

**Goal:** Given B IT-MAC commitments `[t_0], ..., [t_{B-1}]`, prove one is zero with O(log B) communication instead of Robin's O(B).

**Bit-decomposition of branch id.** Let `B = 2^b`. P commits to bits `id_0, ..., id_{b-1}` via IT-MACs and proves each is binary (via batched LPZK: `id_i * (id_i - 1) = 0`). Cost: b IT-MACs in F_p + O(1) extension field elements.

**Path matrix construction.** P prepares b random IT-MACs `[delta_0], ..., [delta_{b-1}]` over F_{p^q}. V issues challenge `Lambda in F_{p^q}`. Parties construct the 2 x b matrix of IT-MACs:

```
       col 0                          col b-1
row 0: [Lambda*(1 - id_0) + delta_0]  ...  [Lambda*(1 - id_{b-1}) + delta_{b-1}]
row 1: [Lambda*id_0 - delta_0]        ...  [Lambda*id_{b-1} - delta_{b-1}]
```

Properties:
- Each column sums to Lambda (public).
- Each element is revealable because delta_i is uniform.
- In column i, Lambda appears only on row `id_i`.

P opens the second row. For each `a in [B]`, bit-decompose `a = (a_0, ..., a_{b-1})` and compute:

```
C_a = prod_{i=0}^{b-1} M_{a_i, i}(Lambda)
```

`C_id` is a degree-b polynomial in Lambda (all b factors contain Lambda). For `a != id`, at least one factor lacks Lambda, so `C_a` has degree <= b-1 in Lambda.

**Proving zero membership.** Parties locally compute:

```
[S] = sum_{a=0}^{B-1} C_a * [t_a]
```

Since `t_id = 0`, the `C_id * [t_id]` term vanishes. So `S = s(Lambda)` where `s(X)` is a degree-(b-1) polynomial whose coefficients `s_0, ..., s_{b-1}` are known to P (independent of Lambda). P commits to these b coefficients before Lambda is issued. After Lambda is public, parties open `[S] - [s_0] - Lambda*[s_1] - ... - Lambda^{b-1}*[s_{b-1}]` and check it is zero. Soundness follows from SZDL: if all t_i are nonzero, S would be degree-b in Lambda, which a degree-(b-1) polynomial cannot match except with negligible probability.

**LogRobin result:** `(n_in + 3n_x) log p + O(rho * log B)` bits, O(1) rounds. P computation O(B|C| + B log B), V computation O(B|C|).

### Stepping Stone 2: Robin++ -- Commit Only to w and o

**Key insight:** P commits only to `(w, o)` instead of `(w, l, r, o)`. P and V execute single-circuit VOLE-based ZK evaluation on each branch `C_i` using committed `[w]` and `[o]`, then run the Acc sub-procedure with V-issued challenge gamma:

```
For each branch i in [B]:
  t^(i) = Eval-IT-MAC(C_i, [w], [o])     // returns n_x+1 IT-MAC triples
  P computes: M_2^(i), M_1^(i), M_0^(i) = AccP(t^(i), gamma)
  V computes: K^(i) = AccV(t^(i), gamma)
```

The correlation holds: `M_2^(i) * Delta^2 + M_1^(i) * Delta + M_0^(i) = K^(i)` for all i, and `M_2^(id) = 0` (because on the active branch all multiplication triples hold).

**Affine-polynomial-correlation problem:** P holds B quadratic polynomials `p^(i)(X) = M_2^(i)*X^2 + M_1^(i)*X + M_0^(i)` and V holds `K^(i) = p^(i)(Delta)`. P must prove one polynomial is actually affine (leading coefficient = 0).

**Robin++ solution (sub-optimal, O(B)):** P commits all `M_2^(i)` via IT-MACs and proves their product is zero. Then V issues random chi, parties batch-check that the committed M_2 values are consistent with the K^(i) values using two fresh VOLE correlations `[r_2], [r_1]` as masks:

```
V checks: k_{r_2}*Delta + k_{r_1} + sum_i chi_i * K^(i) = M_2*Delta^2 + M_1*Delta + M_0
```

where `M_2, M_1, M_0` are the batch-aggregated (and randomized) coefficients.

**Robin++ result:** `(n_in + n_x) log p + O(rho * B)` bits, O(1) rounds. When B=1, degenerates to QuickSilver.

### Final Protocol: LogRobin++

**Non-trivial combination** of LogRobin's path-matrix technique with Robin++'s affine-polynomial-correlation reduction. The K^(i) values serve as "conceptual commitments" to M_2^(i). The path matrix induces C_a values, and V computes:

```
S = sum_{a=0}^{B-1} C_a * K^(a)
```

This is a bivariate polynomial `s(Lambda, Delta)` of degree at most b in Lambda and 2 in Delta:

```
s(Lambda, Delta) = sum_{j=0}^{b} sum_{k=0}^{2} s_{j,k} * Lambda^j * Delta^k
```

The coefficient `s_{b,2}` (of `Lambda^b * Delta^2`) can only be induced by `C_id * K^(id)`, and since `M_2^(id) = 0` it vanishes. So if P is honest, `s` has degree < b+2. There are 3(b+1) coefficients, all known to P.

**Randomization with VOLE correlations.** P cannot evaluate at Delta (must stay secret). Instead:

- For the `Lambda^b` terms: consume one VOLE `[r_b]`. P sends `s_{b,1} + r_b` (one-time padded). P commits `s_{b,0} + m_{r_b}` as IT-MAC.
- For each `Lambda^j` (j in [b]): consume two VOLEs `[r_{j,2}], [r_{j,1}]`. P sends `s_{j,2} + r_{j,2}` and `s_{j,1} + m_{r_{j,2}} + r_{j,1}` (both one-time padded). P commits `s_{j,0} + m_{r_{j,1}}` as IT-MAC.

The b+1 committed (IT-MAC) values are all coefficients of `Delta^0` terms. After Lambda is issued, parties open:

```
[S'] = sum_{j=0}^{b} Lambda^j * [s_{j,0}]
```

V reconstructs `S'` by adding in the disclosed coefficients times appropriate powers of `Lambda * Delta`. V checks `S = S'`.

**Why some coefficients stay in IT-MACs:** If `s_{b,0} + m_{r_b}` were disclosed, a malicious V setting `Delta=0` could learn `s_{b,0} = M_0^(id)`, then compare against known `M_0^(i)` values to identify id.

**Protocol steps summary (Figures 5-6):**
1. P evaluates `C_id(w)` to get extended witness `o`.
2. Initialize VOLE, generate IT-MACs for witness commitment, bit-decomposition, randomization.
3. P commits `w, o` by sending differences from VOLE random masks (n_in + n_x elements in F_p).
4. P commits `id` bit-by-bit (b elements in F_p), proves each bit is binary via batched LPZK.
5. For each branch i: Eval-IT-MAC + AccP/AccV to get the quadratic correlation.
6. P constructs bivariate polynomial `s(X,Y)`, randomizes it, declares 2b+1 padded coefficients (in F_{p^q}) and commits b+1 coefficients as IT-MACs.
7. V issues Lambda. Parties open path matrix, V computes S, adds randomization. Parties open `[S']`. V checks `S = S'`.

**Soundness error:** `(B + b + 7) / p^q`. UC-secure against static unbounded adversary.

### Sub-procedures

**Eval-IT-MAC(C, [in], [o]):** Evaluates circuit C gate-by-gate over IT-MACs. Input gates: place `[in_j]` on wire. Addition gates: `[x] + [y]` locally. Multiplication gates: place `[o_j]` on output, record triple `([x_j], [y_j], [o_j])`. Append output check `([res], [res], [0])`. Returns vector of n_x+1 triples. Cost: O(|C|).

**AccP(t, gamma):** P linearly combines triples with challenge gamma:
```
M^(2) = sum_j gamma_j * (x_j * y_j - z_j)
M^(1) = sum_j gamma_j * (x_j * m_{y_j} + y_j * m_{x_j} - m_{z_j})
M^(0) = sum_j gamma_j * m_{x_j} * m_{y_j}
```

**AccV(t, gamma):** V computes:
```
K = sum_j gamma_j * (k_{x_j} * k_{y_j} - k_{z_j} * Delta)
```

Invariant: `M^(2) * Delta^2 + M^(1) * Delta + M^(0) = K`. If all triples are valid multiplications, `M^(2) = 0`.

---

## Cost Model

All costs in the **VOLE-hybrid model** (exclude VOLE generation cost).

### Communication Comparison (VOLE-hybrid)

| Protocol | Field | P->V Communication (bits) | Rounds |
|---|---|---|---|
| Mac'n'Cheese | Boolean F_2 | `n_in + n_x + 2*lambda*n_x + O(lambda * log B)` | O(log B) |
| Mac'n'Cheese | Arithmetic F | `(n_in + 3n_x) log|F| + O(log B * log|F|)` | O(log B) |
| Robin | Boolean F_2 | `n_in + 3n_x + O(lambda * B)` | O(1) |
| Robin | Arithmetic F | `(n_in + 3n_x) log|F| + O(B * log|F|)` | O(1) |
| **LogRobin** | Boolean F_2 | `n_in + 3n_x + O(lambda * log B)` | O(1) |
| **LogRobin** | Arithmetic F | `(n_in + 3n_x) log|F| + O(log B * log|F|)` | O(1) |
| **Robin++** | Boolean F_2 | `n_in + n_x + O(lambda * B)` | O(1) |
| **Robin++** | Arithmetic F | `(n_in + n_x) log|F| + O(B * log|F|)` | O(1) |
| **LogRobin++** | Boolean F_2 | `n_in + n_x + O(lambda * log B)` | O(1) |
| **LogRobin++** | Arithmetic F | `(n_in + n_x) log|F| + O(log B * log|F|)` | O(1) |

### Computation (VOLE-hybrid, extension field operations)

| Protocol | Prover | Verifier |
|---|---|---|
| Robin | O(B\|C\|) | O(B\|C\|) |
| LogRobin | O(B\|C\| + **B log B**) | O(B\|C\|) |
| Robin++ | O(B\|C\|) | O(B\|C\|) |
| LogRobin++ | O(B\|C\| + **B log B**) | O(B\|C\|) |

The B log B term (gray-boxed in paper) is P computing the bivariate polynomial coefficients. V's path-matrix evaluation is O(B).

### LogRobin++ Detailed P->V Breakdown

| Step | What | Count | Field |
|---|---|---|---|
| Steps 4-5 | Witness commit | n_in + n_x | F_p |
| Step 8 | Bit-decomposed id | b | F_p |
| Step 9 | Batched LPZK (bit checks) | 2 | F_{p^q} |
| Step 13 | Randomized coefficients | 2b + 1 | F_{p^q} |
| Step 14 | Committed coefficients | b + 1 | F_{p^q} |
| Step 16 | Path matrix openings | 2b | F_{p^q} |
| Step 19 | Final IT-MAC opening | 2 | F_{p^q} |
| **Total** | | n_in + n_x + b in F_p, 5b + 6 in F_{p^q} | |

### VOLE Correlations Consumed

| Type | Count | Purpose |
|---|---|---|
| Subfield VOLEs (F_p values) | n_in + n_x + b | Commit witness + id bits |
| Full VOLEs (F_{p^q} values) | 2 + 4b (combined into b+1 delta, 1 r_b, b r_{j,2}, b r_{j,1}, b+1 tau) | Randomization + coefficient commitment |

### Challenge Generation Variants

| Variant | V->P comm | Assumption | Soundness error |
|---|---|---|---|
| Independent | Omega(n_x + b) elements in F_{p^q} | IT | (B+b+7)/p^q |
| RO (PRG from kappa-bit seeds) | O(kappa) bits | Random Oracle | (B+b+7)/p^q + 2Q/2^kappa |
| IT (power of single element) | O(lambda) bits | None (IT) | (B*n_x + 2b + 4)/p^q |

---

## Concrete Performance

All benchmarks: AWS EC2 m5.xlarge (Intel Xeon Platinum 8175 @ 3.1GHz, 4 vCPUs, 16GiB), single-threaded. Costs **include** VOLE generation. RO variant used.

### Benchmark "Many": B = 2^22, n_in = 10, n_x = 100

| Field | Protocol | P->V | V->P | Total Comm | Comm Impr | LAN 1Gbps (s) | LAN Impr | WAN 10Mbps (s) | WAN Impr |
|---|---|---|---|---|---|---|---|---|---|
| F_2 | Robin | 64 MB | 28 MB | 92 MB | baseline | 51.2 | baseline | 114.1 | baseline |
| F_2 | LogRobin | 9 KB | 540 KB | 549 KB | 172x | 15.1 | 3.4x | 14.8 | 7.7x |
| F_2 | Robin++ | 128 MB | 56 MB | 184 MB | 0.5x | 94.6 | 0.5x | 212.2 | 0.5x |
| F_2 | LogRobin++ | 10 KB | 540 KB | 550 KB | 172x | 16.4 | 3.1x | 16.1 | 7.1x |
| F_{2^61-1} | Robin | 32 MB | 2 MB | 34 MB | baseline | 25.8 | baseline | 54.3 | baseline |
| F_{2^61-1} | LogRobin | 0.8 MB | 1.7 MB | 2.5 MB | 13.6x | 27.0 | 1.0x | 28.6 | 1.9x |
| F_{2^61-1} | Robin++ | 64 MB | 2 MB | 66 MB | 0.5x | 13.8 | 1.9x | 68.7 | 0.8x |
| F_{2^61-1} | LogRobin++ | 0.8 MB | 1.7 MB | 2.5 MB | 13.6x | 15.3 | 1.7x | 17.3 | 3.1x |

### Benchmark "Large": B = 2, n_in = 10, n_x = 10^7

| Field | Protocol | P->V | V->P | Total Comm | Comm Impr | LAN 1Gbps (s) | LAN Impr | WAN 10Mbps (s) | WAN Impr |
|---|---|---|---|---|---|---|---|---|---|
| F_2 | Robin | 3.6 MB | 1.0 MB | 4.6 MB | baseline | 8.1 | baseline | 10.1 | baseline |
| F_2 | Robin++ | 1.2 MB | 0.5 MB | 1.7 MB | 2.7x | 5.3 | 1.5x | 5.9 | 1.7x |
| F_2 | LogRobin++ | 1.2 MB | 0.5 MB | 1.7 MB | 2.7x | 5.4 | 1.5x | 6.1 | 1.7x |
| F_{2^61-1} | Robin | 230 MB | 3 MB | 233 MB | baseline | 11.7 | baseline | 205.8 | baseline |
| F_{2^61-1} | Robin++ | 77 MB | 1 MB | 78 MB | 3.0x | 6.5 | 1.8x | 71.7 | 2.9x |
| F_{2^61-1} | LogRobin++ | 77 MB | 1 MB | 78 MB | 3.0x | 6.4 | 1.8x | 71.7 | 2.9x |

### Scaling Trends (VOLE-hybrid model only, excluding VOLE gen)

- **Comm vs B** (n_in=10, n_x=100, log B from 4 to 16): Robin grows exponentially in b; LogRobin++ grows linearly in b.
- **Comm vs |C|** (B=2, n_in=10, n_x from 10^6 to 10^7): Both grow linearly; Robin ~3x LogRobin++.
- **B identical vs B different circuits:** Negligible performance difference.
- **RO vs IT variant:** Negligible wall-clock difference.

---

## Key Definitions

- **Disjunctive statement:** P and V agree on B circuits `C_0, ..., C_{B-1}`. P proves knowledge of `(id, w)` s.t. `C_id(w) = 0`, hiding both id and w.
- **Active branch:** The branch id for which P holds a valid witness. All other branches are inactive.
- **IT-MAC:** `[x]_Delta = <(x, m_x), k_x>` where `m_x = k_x - x*Delta`. P holds `(x, m_x)`, V holds `k_x` and global key Delta. Linearly homomorphic, binding with probability `1/|F|`.
- **VOLE correlation:** Random IT-MAC instances generated via Vector OLE. P gets `(u, m_u)`, V gets `k_u`, with `k_u = m_u + u*Delta`. Subfield VOLE: u in F_p, keys/MACs in F_{p^q}.
- **LPZK (Line-Point Zero Knowledge):** Technique to verify a multiplication triple inside IT-MACs. The relation `k_x*k_y - k_z*Delta` is degree-2 in Delta if `xy != z`, degree-1 if `xy = z`. P sends one masked evaluation; V checks.
- **Batched LPZK:** V issues random challenges to linearly aggregate all multiplication checks into a single degree test on Delta, then verifies with one masked opening.
- **Path matrix:** A 2 x b matrix of opened IT-MACs constructed from bit-decomposed id and random masks delta_i. Induces B scalars `C_a` where `C_id` is uniquely degree-b in Lambda while all others are degree < b.
- **Affine-polynomial-correlation problem:** P holds B quadratic polynomials `p^(i)(X)`, V holds their evaluations at secret Delta. P must prove one polynomial is affine (degree <= 1). Arises when P commits only to `(w, o)` and runs LPZK per branch.
- **rho:** `rho = max{log|F|, lambda}`. Governs the extension field overhead for statistical security.
- **VOLE-hybrid model:** Ideal model where VOLE correlations are provided by a trusted functionality. Protocol costs exclude VOLE generation.
- **Soundness error (LogRobin++):** `(B + b + 7) / p^q` where `B = 2^b`, in IT variant `(B*n_x + 2b + 4) / p^q`.

---

## Relevance to VOLE-Based zkVM

1. **Direct instruction-set disjunction primitive.** A zkVM executing a CPU step performs a disjunction over the instruction set. LogRobin++ makes the communication overhead of hiding the executed instruction O(log B) instead of O(B), where B is the number of opcodes. For a 256-opcode ISA, this is an 8-bit overhead instead of 256-element overhead per step.

2. **Commit only to inputs + multiplication outputs.** Robin++ / LogRobin++ show that P need not commit to all three wires per multiplication gate on the active branch -- only inputs `w` and outputs `o`. This saves ~2n_x field elements per disjunction, a concrete ~3x reduction in per-gate communication. For a zkVM with large per-instruction circuits, this dominates.

3. **Constant-round in VOLE-hybrid model.** Unlike Mac'n'Cheese's O(log B) rounds, LogRobin++ is O(1) rounds, making it suitable for WAN deployment where round-trips are expensive. The 7x WAN speedup over Robin confirms this matters in practice.

4. **Boolean and arithmetic field support.** LogRobin++ works over F_2 (via subfield VOLE, lambda >= 100) and F_{2^61-1} (lambda >= 40). A zkVM can use Boolean circuits for bitwise operations and arithmetic circuits for field arithmetic, applying LogRobin++ to both.

5. **Path matrix is reusable across batched disjunctions.** The bit-decomposition of `id` and path matrix construction can potentially be amortized if the same branch is taken across multiple steps, or shared across sub-circuits within one instruction.

6. **Computation cost is O(B|C|) for both parties.** The dominant cost is evaluating all B branches over IT-MACs (to hide which is active). The additional prover cost of O(B log B) for polynomial coefficient computation is modest. This means the verifier is not penalized by the log B technique.

7. **Near-optimal communication.** LogRobin++ adds only O(rho * log B) bits beyond the single-circuit cost of `(n_in + n_x) log|F|`. The paper notes this is comparable to the log B bits needed to non-privately identify the branch -- so the price of privacy is essentially the minimum information-theoretic cost.

8. **Integration with VOLE generation.** The protocol consumes `n_in + n_x + b` subfield VOLEs and `O(b)` full VOLEs per disjunction. These can be pre-generated via LPN-based protocols (e.g., Ferret) at sublinear amortized cost, fitting the standard VOLE-based zkVM pipeline.

9. **AccP/AccV as modular building block.** The Acc sub-procedures provide a clean interface for accumulating multiplication-gate correlations from any circuit evaluation into the quadratic/affine polynomial structure. A zkVM implementation can use this as the core "gate checking" module.

10. **VOLE-in-the-Head compatibility.** The paper notes that all VOLE-based ZK protocols (including LogRobin++) can be compiled to non-interactive, publicly verifiable proofs via the VOLE-in-the-Head technique [BBD+23], if post-hoc public verifiability is needed.
