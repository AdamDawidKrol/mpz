# Batchman and Robin: Batched and Non-batched Branching for Interactive ZK

**Authors:** Yibin Yang, David Heath, Carmit Hazay, Vladimir Kolesnikov, Muthuramakrishnan Venkitasubramaniam
**Venue:** ACM CCS 2023
**ePrint:** [2023/1257](https://eprint.iacr.org/2023/1257)
**Code:** <https://github.com/gconeice/stacking-vole-zk>

## Role in the VOLE-ZK Design Space

This paper addresses the core inefficiency of proving disjunctive statements (1-of-B branch selection) in VOLE-based ZK, which is the fundamental operation in CPU-step emulation. Prior VOLE-based ZK systems (QuickSilver, Mac'n'Cheese, AntMan) pay computation proportional to R·B·|C| for R repetitions of a B-branch disjunction with branch-circuit size |C|. Batchman breaks through this barrier with O(RB + R|C| + B|C|) total computation — the cost does *not* scale in the product RB|C|. This is achieved purely in the VOLE-hybrid model with information-theoretic security, no random oracle (except for challenge compression), and black-box use of VOLE. The companion protocol Robin handles the single-disjunction case with communication-efficient O(B + |C|) field elements. Both operate in the designated-verifier interactive setting.

## Key Ideas

### Core Abstraction: Topology Matrices

Each branch circuit C_i is represented by a **topology matrix** M_i of dimension (2n_× + 1) × (n_in + 3n_× + 1), where n_× is the number of multiplication gates and n_in is the number of inputs. The matrix encodes the linear constraints of the circuit after removing multiplication gates (i.e., the "skeleton" of addition/scale/offset gates).

An extended witness `w_ext = glue(w, ℓ, r, o, 1)` is valid for branch C_i iff:
1. `ℓ ◦ r = o` (multiplication gates are well-formed)
2. `M_i × w_ext = 0` (topology/addition-gate constraints hold)

**MULLEFT algorithm:** Left-multiplying `s^T × M_i` is computed in O(|C|) time by evaluating the circuit's linear gates *backwards* (reverse topological order), avoiding ever materializing the potentially-dense matrix M_i. This is the key to keeping computation linear in circuit size.

### Robin: Single Disjunction Protocol

**Goal:** Prove C_1(w) = 0 ∨ ... ∨ C_B(w) = 0 with communication O(B + |C|), not O(B|C|).

**Protocol steps (Π^{p,q}_Single):**
1. P commits to extended witness `w_ext` using IT-MACs from subfield VOLE (n_in + 3n_× commitments).
2. P and V run LPZK to check multiplication gates: `ℓ_k · r_k = o_k` for all k ∈ [n_×].
3. V sends random challenge vector `s ∈ F_{p^q}^{2n_× + 1}`.
4. Both parties locally compute **compressed topology vectors** `cv_i = s^T × M_i` for each i ∈ [B] using MULLEFT, then homomorphically derive IT-MACs `[v_i] = cv_i^T × [w_ext]` — these are **compressed topology tokens**.
5. P proves `∏_{i∈[B]} v_i = 0` via a small B-product circuit using LPZK.

**Why it works:** If P is honest with active branch a, then `s^T × M_a × w_ext = 0` always, so `v_a = 0` and the product is 0. If P cheats (no valid witness), by Lemma 1 / Corollary 1, `Pr[∃i: s^T × M_i × w_ext = 0] ≤ B/p^q`, so with overwhelming probability no token is 0 and the product is nonzero.

**Soundness error:** (n_× + 2B + 4) / p^q.

### Batchman: Batched Disjunctions Protocol

**Goal:** Prove R repetitions of the same B-branch disjunction with computation O(RB + R|C| + B|C|), not O(RB|C|).

**Key insights enabling batching:**
1. P knows which branch is active per repetition — she can *directly commit* to the compressed topology vector of her active branch.
2. The challenge vector s can be **reused across all R repetitions** safely.
3. Compressed topology vectors have length O(|C|), not O(B|C|).

**Protocol steps (Π^{p,q}_Batch):**
1. For each j ∈ [R], P commits to extended witness `w_ext^{(j)}` using IT-MACs.
2. P and V run LPZK on all Rn_× multiplication gates.
3. V sends random challenge `s ∈ F_{p^q}^{2n_× + 1}`.
4. Both compute B compressed topology vectors: `cv_i = (s^T × M_i)^T` for i ∈ [B] — cost O(B|C|), done **once**.
5. For each j ∈ [R], P commits to `cv^{(j)} = cv_{a^{(j)}}` (the compressed topology of her active branch). Cost: R · O(|C|) field elements over the extension field.
6. **Topology check:** P and V run QuickSilver (QS) batched inner-product proof to verify `(cv^{(j)})^T × w_ext^{(j)} = 0` for all j ∈ [R]. Cost: O(R|C|) computation, O(1) communication.
7. V sends second random challenge `t ∈ F_{p^q}^{n_in + 3n_× + 1}`.
8. Both compute B **compressed topology tokens** `ct_i = cv_i^T × t` for i ∈ [B] — cost O(B|C|), done **once**. Each ct_i is a single field element.
9. For each j ∈ [R], both compute `[ct^{(j)}] = [cv^{(j)}]^T × t` homomorphically.
10. **Topology validation:** For each j ∈ [R], P proves `∏_{i∈[B]} (ct^{(j)} - ct_i) = 0` via a B-product circuit using LPZK, demonstrating that P's committed compressed topology matches some valid branch.

**Why topology validation works:** If `cv^{(j)} ∉ {cv_1, ..., cv_B}`, then by Corollary 2, `Pr[cv^{(j)T} × t ∈ {ct_1, ..., ct_B}] ≤ B/p^q`. The second challenge t catches any cheating P who committed to a topology vector not corresponding to any real branch.

**Soundness error:** (Rn_× + R + 3B + 6) / p^q.

### Challenge Compression

Both protocols require V to send large challenge vectors. Three methods:
- **Powers of χ:** V sends one random χ, defines s = (1, χ, χ², ...). O(1) communication, slightly worse soundness (factor n_× increase via Lemma 2).
- **Random oracle:** V sends a λ-bit seed, expand via RO. O(1) communication, RO assumption. This is what the implementation uses.
- **Vandermonde:** Two random field elements generate a two-row Vandermonde matrix for s and t.

### Constraining Batch Witnesses

Batched disjunctions are only useful if witnesses across repetitions are related. Two methods:
- **Per-repetition public parameters:** Shared public inputs per repetition; P opens portions of committed witness.
- **Connecting repetitions:** Prove equality of committed wires across repetitions by subtracting IT-MACs and proving the difference is zero. RO compression reduces overhead to O(1) total.

## Cost Model

### Asymptotic Comparison (Batched Disjunctions)

| Protocol | Prover Comp. | Communication | Verifier Comp. |
|---|---|---|---|
| QuickSilver [YSWW21] | O(RB\|C\|) | O(RB\|C\|) | O(RB\|C\|) |
| AntMan [WYY+22] | O(RB\|C\| log R) | O(B\|C\| + R) | O(RB\|C\| log R) |
| Mac'n'Cheese [BMRS21] | O(RB\|C\|) | O(R log B + R\|C\|) | O(RB\|C\|) |
| **Robin** | O(RB\|C\|) | O(RB + R\|C\|) | O(RB\|C\|) |
| **Batchman** | **O(RB + R\|C\| + B\|C\|)** | **O(RB + R\|C\|)** | **O(RB + R\|C\| + B\|C\|)** |

### Concrete Cost Breakdown — Robin (Single Disjunction)

| Resource | Count |
|---|---|
| Communication | n_in + (2q+3)n_× + q(B+7) field elements; optimized to O(\|C\| + qB) |
| VOLE correlations | n_in + 3n_× + q(B+1) = O(\|C\| + qB) subfield VOLE |
| Rounds | 5 (online phase) |
| Computation (each party) | O(B\|C\|) field ops |

### Concrete Cost Breakdown — Batchman (Batched Disjunctions)

| Resource | Count |
|---|---|
| Communication | (Rq+R)n_in + (3Rq+3R)n_× + qR(B+1) + 2q = O(qRB + qR\|C\|) field elements |
| VOLE correlations | (Rq+R)n_in + (3Rq+3R)n_× + qR(B+1) + 2q = O(qRB + qR\|C\|) subfield VOLE |
| Rounds | 7 (online phase) |
| Computation (each party) | O(RB + R\|C\| + B\|C\|) field ops |

### Computation Breakdown per Step (Batchman)

| Step | Cost |
|---|---|
| Generate R extended witnesses | O(R\|C\|) |
| Combine subfield VOLE → VOLE | O(R\|C\| + RB) |
| Commit R extended witnesses | O(R\|C\|) |
| LPZK (mult check) | O(R\|C\|) |
| MULLEFT × B (compute cv_i) | O(B\|C\|) — **done once** |
| Commit R compressed topology vectors | O(R\|C\|) |
| QS (inner-product check) | O(R\|C\|) |
| Compute B compressed topology tokens | O(B\|C\|) — **done once** |
| R × LPZK (B-product circuits) | O(RB) |

## Concrete Performance

### Robin vs Mac'n'Cheese (Boolean, ~1B AND gates, 30 Mbps / 100ms latency)

| B (branches) | Mac'n'Cheese (50 threads, λ≥40, no VOLE) | Robin (1 thread, λ≥100, with VOLE) |
|---|---|---|
| 2 | 307s | 468s |
| 4 | 568s | 520s |
| 8 | 1254s | 615s |
| 16 | — | 812s |
| 32 | — | 1209s |
| 64 | — | 2004s |

Robin pays ~25s/extra branch vs Mac'n'Cheese ~150s/extra branch. Communication: Robin 628 MB vs Mac'n'Cheese 124 MB (but Robin stays nearly constant as B grows, ~200 MB total for arithmetic).

### Robin vs QuickSilver (F_{2^61-1}, 8M mult gates per branch)

| B | QS 100Mbps | QS 1Gbps | Robin 100Mbps | Robin 1Gbps | QS Comm. | Robin Comm. |
|---|---|---|---|---|---|---|
| 1 | 7.0s | 2.3s | 18.1s | 3.9s | 73.7MB | 197MB |
| 10 | 60.3s | 13.0s | 19.8s | 5.9s | 659MB | 199MB |
| 50 | 294.1s | 59.3s | 27.5s | 13.3s | 3.17GB | 199MB |
| 100 | 582.0s | 114.5s | 37.1s | 23.0s | 6.33GB | 199MB |

**Up to 16× speedup** over QuickSilver at B=100, 100Mbps.

### Batchman vs AntMan (F_{2^61-1}, 2^21 total mult gates per repetition, R=1024)

| Protocol | 50 Mbps | 100 Mbps | 500 Mbps | 1 Gbps |
|---|---|---|---|---|
| AntMan-1 (1 thread) | 2.00 Mgps | 2.05 Mgps | 2.08 Mgps | 2.09 Mgps |
| AntMan-4 (4 threads) | 6.88 Mgps | 6.69 Mgps | 6.99 Mgps | 7.01 Mgps |
| Batchman-(8, 2^18) | 0.96 Mgps | 1.83 Mgps | 6.60 Mgps | 9.60 Mgps |
| Batchman-(64, 2^15) | 7.55 Mgps | 14.43 Mgps | 51.28 Mgps | 75.40 Mgps |
| Batchman-(512, 2^12) | 56.20 Mgps | 104.91 Mgps | 335.02 Mgps | 461.82 Mgps |

**Up to 36× over AntMan** (B=64, R=1024). Up to 221× with B=512 at 1 Gbps.

### Batchman vs QuickSilver (F_{2^61-1}, B=R, 1.25×10^5 mult gates per branch)

| B=R | QS 100Mbps | QS 1Gbps | Batchman 100Mbps | Batchman 1Gbps |
|---|---|---|---|---|
| 50 | 231.6s | 46.5s | 28.6s | 6.2s |
| 100 | 926.1s | 186.8s | 55.9s | 11.9s |
| 200 | 3694.3s | 735.9s | 111.4s | 21.5s |
| 400 | 14747.3s | 2983.6s | 221.2s | 42.6s |

**Up to 70× over QuickSilver** at B=R=400, 1 Gbps.

### Batchman Runtime Breakdown (seconds, F_{2^61-1}, 1.25×10^5 mult gates/branch)

| BW | B | R | Mult Check | MULLEFT | Commit Topo | Inner-Prod | Topo Valid |
|---|---|---|---|---|---|---|---|
| 1 Gbps | 50 | 50 | 3.4 | 0.1 | 2.4 | 0.1 | 0.2 |
| 1 Gbps | 100 | 100 | 6.2 | 0.2 | 4.9 | 0.2 | 0.4 |
| 1 Gbps | 400 | 400 | 20.3 | 0.8 | 18.6 | 0.7 | 2.2 |

MULLEFT is negligible. Dominated by commitment + multiplication check (VOLE-bound).

### CPU Emulation Benchmark (B=50 instructions, 125 mult gates each, F_{2^61-1})

| Protocol | 100 Mbps | 500 Mbps | 1 Gbps |
|---|---|---|---|
| QuickSilver | 181 Hz | 625 Hz | 902 Hz |
| Batchman | 1525 Hz | 5375 Hz | 7891 Hz |

**~9× improvement.** Note: each "CPU step" executes 125 multiplications, vs single-mult steps in prior ZK CPUs (ZEE).

## Key Definitions

- **Batched disjunction:** R repetitions of the same B-way disjunction. P holds R witnesses w^{(1)}, ..., w^{(R)} and R branch selectors a^{(1)}, ..., a^{(R)} ∈ [B], proving C_{a^{(j)}}(w^{(j)}) = 0 for all j ∈ [R].
- **Extended witness (w_ext):** `glue(w, ℓ, r, o, 1)` — the input witness concatenated with left-inputs, right-inputs, and outputs of every multiplication gate in the active branch, plus a trailing 1 (for offset gates). Length: n_in + 3n_× + 1.
- **Topology matrix (M_i):** A (2n_× + 1) × (n_in + 3n_× + 1) matrix encoding the linear constraints (addition/scale/offset gates) of branch C_i. Rows correspond to: left-inputs of mult gates (n_× rows), right-inputs of mult gates (n_× rows), circuit output (1 row). `M_i × w_ext = 0` iff w_ext respects C_i's linear structure.
- **Compressed topology vector (cv_i):** `cv_i = (s^T × M_i)^T`, a length-(n_in + 3n_× + 1) vector computed via MULLEFT in O(|C|) time. Compresses a full topology matrix into a single vector using V's random challenge s.
- **Compressed topology token (ct_i):** `ct_i = cv_i^T × t`, a single field element. Further compresses a topology vector using V's second challenge t. Used for efficient set-membership checks in the batched protocol.
- **MULLEFT:** Algorithm computing `s^T × M` in O(|C|) time by evaluating the circuit's linear gates in reverse topological order, without materializing M. The key to O(|C|)-per-branch computation.
- **IT-MAC:** Information-theoretic MAC commitment. P holds (m_x, x), V holds (k_x, Δ), where m_x = k_x − x·Δ. Linearly homomorphic, binding (guessing Δ required to forge), hiding.
- **LPZK (Line-Point Zero Knowledge):** Sub-protocol proving n IT-MAC tuples satisfy multiplication relations. Cost: O(n) computation, O(1) communication (amortized 1 field element per gate), one random challenge from V.
- **QS (QuickSilver inner-product proof):** Sub-protocol proving inner-product of two committed vectors is zero. Cost: O(m) computation, O(1) communication. k batched inner-product proofs can be compressed to O(1) communication.
- **Subfield VOLE:** VOLE variant generating IT-MACs where committed values are from base field F_p but MAC/key values live in extension field F_{p^q}. Essential for small-field (e.g., Boolean) security where Δ must be hard to guess.

## Relevance to VOLE-Based zkVM

1. **Direct CPU-step emulation model.** Each CPU step is exactly a 1-of-B disjunction (which instruction to execute), repeated R times (R = number of steps). Batchman's O(RB + R|C| + B|C|) cost means the VM's per-step cost is O(B + |C|) amortized, *independent of the product B·|C|*. This enables large instruction sets (hundreds of opcodes) without proportional blowup.

2. **Concrete throughput for zkVM design targets.** At B=50, |C|=125, Batchman achieves 7891 Hz (1 Gbps) for CPU steps. For a real ISA with ~200 opcodes and ~1000 gates per instruction, the batched approach remains feasible. The O(B|C|) one-time setup cost is amortized over R steps.

3. **The O(B|C|) one-time cost is the critical planning constraint.** MULLEFT runs once to compute B compressed topology vectors. For B=256 opcodes, |C|=4096 gates each, this is ~1M field ops — negligible. But for very large |C| or B, this setup dominates short traces. The design should target R ≥ B for amortization.

4. **Communication model fits the interactive zkVM setting.** Communication is O(qRB + qR|C|) = O(R(qB + q|C|)). For F_{2^61-1} (q=1), this is ~R·(B + |C|) field elements per proof. At 8 bytes per element, a 1M-step proof with B=64, |C|=1024 uses ~64·10^6 × 8 ≈ 0.5 GB. Batchman's communication advantage over QuickSilver grows linearly with B.

5. **Robin handles non-uniform disjunctions.** Not all VM operations fit the batched model (e.g., syscalls, initialization, finalization). Robin provides single-disjunction handling with O(B + |C|) communication, covering irregular steps at the cost of O(B|C|) computation per instance.

6. **Topology matrix formalism enables modular instruction-set design.** Each instruction is an independent circuit with its own topology matrix. Adding/removing instructions changes only the set of M_i matrices — the protocol structure is unchanged. This maps directly to a zkVM ISA: each opcode → one branch circuit → one topology matrix.

7. **Witness-connection mechanism enables state threading.** The batch witness constraining techniques (Appendix E) — proving wire equality across repetitions via IT-MAC subtraction + zero-check — directly implement CPU register/memory state passing between steps. Cost is O(1) per constraint via RO batching.

8. **Extension-field overhead matters for small fields.** Batchman's communication includes a factor q (extension degree). For Boolean circuits (q=128), communication blows up by 128×. The paper notes repeating Robin is better for Boolean batched disjunctions. For a zkVM over F_{2^61-1} (q=1), this is not an issue. **Design implication: prefer large arithmetic fields for the VM's native field.**

9. **Round complexity is low.** Robin: 5 rounds online. Batchman: 7 rounds online. Both assume VOLE correlations are preprocessed. For a streaming/pipelined zkVM, the 7-round structure of Batchman can be overlapped with trace generation.

10. **Composability with ZK-RAM.** The paper explicitly notes Batchman needs ZK-RAM (e.g., [FKL+21]) for full CPU emulation. The topology-matrix approach handles control flow (instruction selection); RAM handles data-dependent memory access. These compose in the VOLE-hybrid model. The paper's CPU benchmark (B=50, 125 mults/step, no RAM) achieves 7.9 KHz — adding RAM will reduce this but the branching component's cost is well-characterized.

11. **Optimization via ZK-ROM [YH24].** A noted optimization replaces the topology-token membership check with a ZK Read-Only Memory lookup, reducing Batchman's communication to O(B + R|C|) — removing the RB term entirely. This is significant for large B (many opcodes) with many steps.
