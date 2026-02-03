# Tight ZK CPU: Batched ZK Branching with Cost Proportional to Evaluated Instruction

**Authors**: Yibin Yang, David Heath, Carmit Hazay, Vladimir Kolesnikov, Muthuramakrishnan Venkitasubramaniam
**Venue**: ACM CCS 2024 | **ePrint**: [2024/456](https://eprint.iacr.org/2024/456)

## Role in the VOLE-ZK Design Space

Tight ZK CPU is the first ZK CPU protocol where the cost of executing each instruction is proportional to **only the size of the taken instruction**, rather than the largest instruction in the instruction set. Prior VOLE-based ZK CPUs (Batchman, CCS'23) pay for the largest branch on every step: all instructions are padded to the maximum size, and the prover/verifier process that many gates regardless of which instruction actually executes. Tight ZK CPU eliminates this waste by introducing a novel ZK Unbalanced Read-Only Memory (UROM) primitive that handles variable-length topology vectors without leaking instruction boundaries. The protocol is formalized in the commit-and-prove paradigm (FCPZK-hybrid), is public-coin and constant-round, and is instantiated concretely with VOLE-based ZK over F_{2^61 - 1}. For instruction sets with heterogeneous sizes -- the natural case for CFG blocks of a compiled program or Wasm opcodes -- this yields 5-18x improvement over Batchman, with cost only ~6-7x above the insecure baseline where the execution path is public.

## Key Ideas

### Background: Topology Matrices and Batchman

A circuit C with n_in inputs and n_x multiplication gates can be represented by separating multiplicative and linear constraints. P commits to an extended witness `(1, in, l, r, o)` where `l, r, o` are the left/right/output wires of all multiplications. Multiplicative constraints are checked via `l . r = o`. Linear constraints are captured by a **topology matrix** M such that:

```
M x (1, in, l, r, o)^T = 0
```

V issues a random challenge chi, and both parties compress M into a **topology vector** `c = (1, chi, ..., chi^{2n_x}) x M`. The proof reduces to checking that `<c, extended_witness> = 0`.

Batchman (CCS'23) extends this to batched disjunctions: B circuits C_1,...,C_B of **the same size** are each encoded as topology vectors. P commits to the topology vector of the taken branch per step and proves it was loaded from a ZK ROM of valid instruction hashes. The critical limitation: **all instructions are padded to the size of the largest**, so V knows the instruction boundaries.

### Tight ZK CPU: Variable-Size Instructions

The key insight: make instruction boundaries private by having P commit a **boundary string** p and an **index vector** id.

**Boundary string** p: a binary vector of length n (total execution gates) with 1s at instruction endpoints, last element always 1:
```
p = (0,...,0, 1, 0,...,0, 1, ..., 0,...,0, 1)
         n^(1)       n^(2)           n^(tau)
```

**Index vector** id: at each boundary position (p_i = 1), id_i gives the instruction index; other positions are filled with arbitrary valid indices (used as dummy ROM queries):
```
id = (i1,...,i1, i2,...,i2, ..., i_tau,...,i_tau)
```

Functions `Partition(p, v)` splits vector v into subvectors at p's boundaries; `Filter(p, id)` extracts the boundary values, yielding the sequence of executed instruction indices.

### Refined Topology Matrices

The paper introduces a ~2x optimization over Batchman's topology matrices. Since multiplication wire ordering (l_1, l_2, r_1, r_2, ...) is fixed, those routing constraints need not be in the matrix. The refined form:

```
M x (in_1, o_1, ..., in_n, o_n)^T = (l_1, r_1, ..., l_n, r_n)^T
```

Each instruction's topology matrix M^(i) is a 2n^(i) x 2n^(i) public matrix. The composed circuit's topology matrix is block-diagonal:

```
M = diag(M^(i1), M^(i2), ..., M^(i_tau))
```

This block-diagonal structure is the foundation: the composed topology vector c is the concatenation of per-instruction topology vectors, and constructing/validating c without revealing the block boundaries is the core technical challenge.

### Handling Constant 1 and Registers

Each instruction needs access to the constant 1. Since instructions cannot reference absolute wire positions (that would change the topology matrix hash), the solution is **push-forward**: each instruction's topology matrix includes rows that define the *next* instruction's first MULT as `1 * 1 = 1`. This way, each instruction's M^(i) is position-independent.

Registers work identically: each instruction's topology matrix includes rows that forward register values to the next instruction as MULTs of the form `1 * reg_k = reg_k`. For m registers, each instruction C_i has total size:

```
n^(i) = n_x^(i) + m + 2
```

where n_x^(i) is the instruction's real multiplication count, m accounts for register-forwarding MULTs, +1 for the `1*1=1` MULT, and +1 for the checking output `1*0=0` MULT.

### Constructing the Topology Vector Commitment

P must commit to `c = (1, chi, ..., chi^{2n-1}) x M` without revealing M. Using the block-diagonal structure:

```
c = a . b

where:
  a = Expand1(p, chi) = (1,...,1, chi^{2n^(1)}, ..., chi^{2n^(1)}, chi^{2n^(1)+2n^(2)}, ...)
  b = v^(i1) || v^(i2) || ... || v^(i_tau)
  v^(i) = (1, chi, ..., chi^{2n^(i)-1}) x M^(i)   [public, computed by both parties]
```

Vector a is constructed from chi and p (via n-1 MULTs). Vector b is the concatenation of variable-length topology vectors -- and constructing/validating b is exactly the ZK UROM problem.

### Expand Functions for Challenge Composition

Two challenge-expansion procedures compose V's challenges with P's boundary string:

**Expand1(p, chi)**: produces a vector where each subvector (partitioned by p) is a constant block at the appropriate power of chi. Used to construct the topology vector scaling factor a.

**Expand2(p, gamma)**: produces a vector where each subvector is `(1, gamma, gamma^2, ...)` resetting at each boundary. Used to compute per-subvector hash checksums in the UROM protocol.

Both are computed from com(p) via circuits with n-1 MULTs.

### ZK Unbalanced Read-Only Memory (UROM)

The novel building block. Standard ZK ROM (YH24) stores B entries of equal length. ZK UROM stores B entries `v^(1), ..., v^(B)` of **different lengths** and allows P to read a concatenation `d = v^(i1) || ... || v^(i_tau)` where V learns only `|d|`.

**Restriction**: all stored vectors must be **non-zero-end** (last element != 0). This enables soundness via Schwartz-Zippel on polynomial hashing of vectors with unknown (to V) lengths (Corollary 1 in the paper).

**Single-read reduction**: To check `w = v^(t)`:
1. V issues challenge gamma. Both compute `mac^(i) = <(1, gamma, gamma^2, ...), v^(i)>` for each i in [B].
2. Load `mac^(t)` from a standard (balanced) ZK ROM using com(t).
3. P proves `last(w) != 0` by committing `inv = last(w)^{-1}` and checking `last(w) * inv = 1`.
4. P proves `<(1, gamma, gamma^2, ...), w> = mac^(t)` by opening the difference.

**Batch-read generalization**: For full execution, P and V use the committed p and id to perform all checks in a single linear pass:

1. V issues gamma. Both compute B public MACs: `mac^(i) = <(1, gamma, ...), v^(i)>`.
2. P loads **selected MACs** `smac` via balanced ZK ROM at positions id: `smac_i = mac^(id_i)`.
3. Both compute `s = Expand2(p, gamma)`, the per-subvector hash basis.
4. P proves each subvector ends non-zero: commit `inv` where `inv_i = d_i^{-1}` if `p_i = 1`, else 0. Check `inv . d - p = 0`.
5. P proves hash consistency via a **running accumulator** (not per-position, which would be quadratic):
   ```
   for each i in [n]:  p_i * (sum_{j=1}^{i} d_j * s_j  -  sum_{j=1}^{i} p_j * smac_j) = 0
   ```
   At non-boundary positions (p_i = 0), the constraint is trivially satisfied. At boundaries, it checks that the accumulated hash matches the accumulated MACs.

**Topology vector optimization**: Each topology vector v^(i) ends with 0 (last column of M^(i) corresponds to the checking output, which isn't input to any wire). To satisfy the non-zero-end requirement, the checking output is folded into the first left wire. This sets the last element to 1, and the inverse vector inv becomes exactly p -- eliminating the need for a separate inverse commitment.

### Full Protocol Flow (ΠZKCPU)

1. **P claims n** (total execution size) to V.
2. **P commits** 4n values: `in, l, r, o` -- the extended witness for all instructions concatenated.
3. **P commits** 2n values: boundary string p and index vector id.
4. **V sends chi** (topology matrix compression challenge).
5. **P and V initialize UROM** with B public topology vectors `v^(i) = (1, chi, ...) x M^(i)`. Free since vectors are public and known to V.
6. **P reads UROM**: calls SetProg(p, id), then ReadUROM to get d = v^(i1) || ... || v^(i_tau), then CheckUROM.
7. **V checks**:
   - (7a) `l . r = o` -- multiplicative constraints (n MULTs).
   - (7b) `o . p = 0` -- checking outputs are 0 at instruction boundaries (n MULTs).
   - (7c) Linear constraints: `<s x M, (1, st^(0), in, o)> = <s, (l, r, ..., st^(final))>` where s is the chi-power vector and d encodes M (4n MULTs).

### Support for RAM (LOAD/STORE)

The protocol supports poly-size read-write memory with only **5 extra registers** and **2 extra rounds**:
- Uses the permutation-based ZK RAM of (YH24): all writes form vector a, all reads form vector b; prove a and b are permutations via `prod(a_i - beta) = prod(b_i - beta)` for random beta.
- Two registers accumulate the two products; one register tracks a global clock; two more for a second permutation check (timing).
- The extra rounds: (1) P commits inputs, (2) V sends beta, (3) P commits l, r, o (which now depend on beta).
- Fiat-Shamir can eliminate the extra rounds.
- The combination hides the number of RAM accesses from V.

### Equality Gates

Equality constraints (forcing two wires equal) are supported efficiently: V sends a random linear combination vector (in the same round as chi), which is embedded as a row of the topology matrix constraining the checking output. Each instruction can use an arbitrary number of equality gates at no asymptotic cost.

### Privacy Leakage

V learns the total number of multiplication gates n on the execution path. This is more granular than prior work (which reveals the number of steps tau): n can help V infer which instructions were executed, especially with heterogeneous sizes. Standard padding can mitigate this. The total number of executed instructions tau is hidden.

### Rounding Optimization

If every instruction size is a multiple of epsilon = gcd(n^(1), ..., n^(B)), the boundary string p has a known structure (every epsilon-th position can be 0), reducing the vectors p, id, inv, smac, s by factor epsilon. This gives ~2x speedup when epsilon >= 16.

## Cost Model

### Asymptotic Cost (VOLE-ZK instantiation, n = total execution gates, B = # instructions)

| Component | Communication | Computation |
|-----------|--------------|-------------|
| Extended witness (in, l, r, o) | 4n field elements (Commit) | O(n) |
| Boundary string + index (p, id) | 2n field elements (Commit) | O(n) |
| UROM read vector d | 2n field elements (Commit) | O(n) |
| Challenge expansion (Expand1, Expand2) | 2n + 2n/epsilon field elements (Commit) | O(n) MULTs each |
| UROM inverse + selected MACs | 2n/epsilon field elements (Commit) | O(n) |
| Topology vector computation | 0 (public) | O(sum n^(i)) per party |
| Balanced ROM (inside UROM) | O(n/epsilon + B) (via YH24) | O(n/epsilon + B) MULTs |
| Check circuits (C1-C6) | ~9n + 2n/epsilon MULTs total | O(n) |
| **Total** | **6n + 6n/epsilon + B + o(n) field elements** | **O(n + sum n^(i))** |

Where epsilon = gcd(n^(1), ..., n^(B)).

### Soundness Error

```
O((m + max{n, n^(1), ..., n^(B)}) / |F|)
```

Over F_{2^61 - 1}, this is negligible for practical parameters.

### Cost Comparison: Tight vs. Non-Tight ZK CPU

| Scenario | Batchman (non-tight) | Tight ZK CPU |
|----------|---------------------|--------------|
| All instructions size s, tau steps | O(tau * s) | O(tau * s) |
| 1 large (size L), rest small (size s), mostly small executed | O(tau * L) | O(tau * s) |
| Varied sizes, average s_avg | O(tau * s_max) | O(tau * s_avg) |

### Overhead vs. Insecure Baseline (Public Execution Path)

| Metric | QuickSilver (public path) | Tight ZK CPU |
|--------|--------------------------|--------------|
| Communication | 1x | ~6-7x |
| Computation | 1x | ~6-7x |

## Concrete Performance

**Field**: F_{2^61 - 1}. **Hardware**: AWS EC2 m5.2xlarge (Intel Xeon 8175 @ 3.10GHz, 8 vCPU, 32 GiB). Single-threaded. **Benchmarks**: randomly generated circuits as instructions.

### Table 1: MGPS and CPM of Tight ZK CPU

| B (instructions) | m (registers) | Distribution | 100 Mbps | 500 Mbps | 1 Gbps | Bytes/MULT |
|---|---|---|---|---|---|---|
| 10 | 5 | Balanced | 111K | 330K | 442K | 102 |
| 50 | 1 | Balanced | 109K | 334K | 438K | 102 |
| 50 | 10 | Balanced | 107K | 323K | 432K | 102 |
| 50 | 20 | Balanced | 108K | 342K | 459K | 102 |
| 100 | 20 | Balanced | 109K | 346K | 458K | 102 |
| 50 | 20 | Unbalanced | 110K | 337K | 467K | 102 |
| 50 | 20 | Varied | 109K | 340K | 460K | 102 |

Key: MGPS and CPM are **independent of B, m, and distribution** -- cost depends only on total multiplication gates.

### Table 2: Hertz Rate Comparison with Batchman (50 instructions, 500K steps, 125 MULT/instr max)

| Protocol | 100 Mbps | 500 Mbps | 1 Gbps | Comm/Step |
|----------|----------|----------|--------|-----------|
| Batchman | 1.5 KHz | 5.4 KHz | 8.0 KHz | 7.3 KB |
| Tight (balanced) | 0.6 KHz (0.56x) | 2.7 KHz (0.51x) | 3.7 KHz (0.46x) | 12.7 KB |
| Tight (balanced + rounding opt.) | 1.7 KHz (1.13x) | 5.9 KHz (1.11x) | 8.5 KHz (1.05x) | 6.3 KB |
| Tight (unbalanced: 1x125 + 49x5) | 10.6 KHz (6.90x) | 32.5 KHz (6.07x) | 43.8 KHz (5.45x) | 1.0 KB |

### Table 3: More Biased Unbalanced (50 instructions, 500K steps, 1x1000 + 49x5 MULT)

| Protocol | 100 Mbps | 500 Mbps | 1 Gbps | Comm/Step |
|----------|----------|----------|--------|-----------|
| Batchman | 0.2 KHz | 0.8 KHz | 1.1 KHz | 52.1 KB |
| Tight (unbalanced) | 4.0 KHz (18.58x) | 12.6 KHz (16.33x) | 17.1 KHz (14.96x) | 2.8 KB |

### Table 4: Overhead vs. Public Execution Path (50 instructions, 50K steps, varied sizes)

| Protocol | 100 Mbps | 500 Mbps | 1 Gbps | Total Comm |
|----------|----------|----------|--------|------------|
| QuickSilver (public path) | 21.2 s | 6.6 s | 5.1 s | 226 MB |
| Tight ZK CPU | 139.1 s (6.56x) | 44.4 s (6.72x) | 31.5 s (6.22x) | 1484 MB (6.56x) |

### Microbenchmarks (50 instructions, 20 registers, 500K steps, varied sizes)

| Protocol Step | 1 Gbps (s) | 500 Mbps (s) | 100 Mbps (s) |
|---------------|-----------|-------------|-------------|
| Total ΠZKCPU | 30.1 | 38.3 | 123.7 |
| Commit witness (Step 2 + 7a) | 7.1 | 10.4 | 39.1 |
| Commit p, id (Step 3) | 3.2 | 4.7 | 19.1 |
| Init UROM (Step 5) | ~0 | ~0 | ~0 |
| SetProg (Step 6a) | 0.2 | 0.2 | 0.2 |
| **UROM Check (Step 6b)** | **15.0** | **17.7** | **53.1** |
| Check o*p (Step 7b) | 0.1 | 0.1 | 0.1 |
| Linear constraints (Step 7c) | 3.9 | 4.5 | 11.3 |

The UROM check (Step 6b) dominates: ~50% of total time. Within it, the balanced ROM read (Sub-step 8(b)ii) takes ~40% of UROM time.

## Key Definitions

- **m-instruction** -- A circuit C: F^{n_in} -> F^{m+1} where the first m outputs are updated registers and the last output is a checking output (must be 0 for valid execution). Size n^(i) = n_x^(i) + m + 2.
- **FZKCPU** -- Ideal functionality for a tight ZK CPU. P proves execution of a sequence of m-instructions from initial to final state. V learns only the total multiplication count n, not the number or identity of instructions.
- **Boundary string p** -- A binary vector in {0,1}^{n-1} || 1 that marks instruction endpoints. Determines how the execution's wire vectors are partitioned into per-instruction subvectors.
- **Topology matrix M^(i)** -- A 2n^(i) x 2n^(i) public matrix encoding instruction C_i's linear constraints in the form `M^(i) x (in, o)^T = (l, r)^T`. Compressed to a topology vector via V's challenge chi.
- **Topology vector v^(i)** -- The public vector `(1, chi, ..., chi^{2n^(i)-1}) x M^(i)`. Stored in the UROM; P reads a concatenation of these.
- **ZK UROM (FCPZK-UROM)** -- Unbalanced read-only memory that stores B non-zero-end vectors of different lengths. P reads a concatenation of entries; V learns only the total read length. Built from standard (balanced) ZK ROM via polynomial hashing with Schwartz-Zippel.
- **Expand1(p, chi)** -- Produces a vector where each partition (by p) is a constant block at the cumulative chi-power. Used for topology vector scaling.
- **Expand2(p, gamma)** -- Produces a vector where each partition resets to `(1, gamma, gamma^2, ...)`. Used for per-subvector hash computation inside UROM.
- **Filter(p, v)** -- Extracts elements of v at positions where p = 1. Gives the sequence of instruction indices.
- **Partition(p, v)** -- Splits v into subvectors at positions where p = 1. Each subvector corresponds to one instruction's data.
- **Rounding optimization** -- When epsilon = gcd(n^(1),...,n^(B)) > 1, vectors p, id, smac, s, inv are shortened by factor epsilon, reducing communication and computation by ~epsilon for boundary-related costs.

## Relevance to VOLE-Based zkVM

1. **Wasm instruction heterogeneity is the ideal use case.** Wasm opcodes vary dramatically in complexity: `i32.add` is a single addition, while `memory.copy` or `call_indirect` compile to large constraint subcircuits. A non-tight ZK CPU pads every step to the cost of the most expensive opcode. Tight ZK CPU pays only for the instruction actually executed, turning this heterogeneity from a liability into a non-issue.

2. **CFG-block-level granularity amplifies the benefit.** Rather than treating each Wasm opcode as a separate instruction, the VM can group straight-line basic blocks as single "instructions." Block sizes can differ by orders of magnitude (a tight loop body vs. a function prologue with many locals). Tight ZK CPU handles this directly; a non-tight CPU would either split large blocks (incurring register-width overhead) or pad small blocks to the maximum.

3. **The 6-7x overhead over public-path execution is the privacy cost.** This is the concrete price of hiding the execution path. For a zkVM, this means the VOLE-based tight ZK CPU adds a moderate constant factor on top of the underlying VOLE-ZK proof for the computation itself. The constant is independent of instruction set size.

4. **The UROM primitive is reusable beyond instruction dispatch.** Any zkVM component that needs to load variable-length data from a public table (e.g., function signatures, type metadata, memory segment descriptors) can use ZK UROM. The cost is proportional to the data read, not the table's maximum entry size.

5. **Native RAM support with 5 registers eliminates a separate RAM argument.** The permutation-based ZK RAM integration requires only constant additional state (5 registers + 2 extra protocol rounds). This is directly applicable to Wasm linear memory: LOAD/STORE compile to a constant number of MULT gates per access, and the RAM consistency proof is folded into the CPU protocol. The number of memory accesses is hidden from V.

6. **Communication budget is well-characterized: ~102 bytes per multiplication gate.** For a zkVM proving N multiplication gates along the execution path, total communication is ~102N bytes, independent of instruction set size, register count, or instruction distribution. At 100 Mbps WAN, throughput is ~100K MULT/s; at 1 Gbps LAN, ~450K MULT/s.

7. **The rounding optimization suggests padding instructions to a common GCD.** If the VM rounds each instruction's MULT count to a multiple of epsilon (e.g., 16 or 32), boundary-related costs drop by ~epsilon, yielding ~2x overall speedup. This is a practical design knob: slightly over-provision small instructions to reduce per-step overhead.

8. **Fiat-Shamir compatibility enables non-interactive proofs.** The protocol is public-coin and constant-round in the FCPZK-hybrid model, so it supports Fiat-Shamir transformation. This is essential for a zkVM that needs to produce proofs consumable by a non-interactive verifier (e.g., on-chain verification or proof aggregation).

9. **Leakage profile differs from prior work and must be managed.** V learns total MULT count n rather than step count tau. For a Wasm zkVM, n reveals more about the execution path than tau would (since instruction sizes are public). The VM design must decide whether to pad n to a fixed upper bound, pad per-instruction to equalize sizes for sensitive code paths, or accept the leakage. The paper notes that standard padding techniques apply.

10. **Composition with AntMan's SIMD batching is an open opportunity.** For repeated sub-circuits (hash rounds, signature verification), AntMan's SIMD mode gives sublinear communication. Tight ZK CPU handles the heterogeneous dispatch layer. A zkVM could use tight ZK CPU for instruction-level branching and AntMan for batched sub-circuit execution within individual instructions.
