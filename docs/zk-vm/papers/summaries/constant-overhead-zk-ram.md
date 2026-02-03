# Constant-Overhead Zero-Knowledge for RAM Programs

**Authors:** Nicholas Franzese, Jonathan Katz, Steve Lu, Rafail Ostrovsky, Xiao Wang, Chenkai Weng
**Venue:** ACM CCS 2021
**ePrint:** [2021/979](https://eprint.iacr.org/2021/979)

## Role in the VOLE-ZK Design Space

This paper is the first to achieve constant-overhead (O(N+T)) interactive ZK proofs for RAM programs, eliminating the O(log N) per-access overhead of all prior work (BubbleRAM, BubbleCache, ORAM-based, routing-network-based). The core technique replaces sorting networks with a polynomial equality check over F_{2^κ} to verify that an access list and its sorted copy are permutations of each other. Built on VOLE-based IT-MACs (Wolverine/QuickSilver), it is the direct predecessor to the "Two Shuffles" line of work, which improves on these results by 2–20×. The paper also introduces a lightweight RISC CPU (13-instruction ISA) that achieves ~320 bytes/cycle and 6.6 KHz at 100 Mbps with 64 MB memory, establishing the baseline architecture for VOLE-based zkVMs.

## Key Ideas

### IT-MAC Authentication Scheme

The verifier holds a global key Δ ∈ F_{2^κ}. A bit x known to the prover is authenticated as:
- Prover holds (x, M_x) where M_x ∈ F_{2^κ} is uniform
- Verifier holds K_x ∈ F_{2^κ} such that K_x = M_x ⊕ x·Δ
- Notation: [x] for Boolean-authenticated values, [[x]] for F_{2^κ}-authenticated values
- XOR-homomorphic: parties holding [x] and [y] can locally compute [x ⊕ y]
- **Free bit-packing:** [x_0], ..., [x_{κ-1}] can be locally converted to [[x]] where x = Σ x_i · X^i in F_2[X]/(f(X)), by having prover compute M_x = Σ M_{x_i} · X^i and verifier compute K_x = Σ K_{x_i} · X^i. No communication needed.

### Read-Only Array Access (Protocol Π_{RO-ZKarray})

**Parameters:** Array size N, word size W bits, constraint W + ⌈log N⌉ ≤ κ.

1. **Initialization:** Prover commits to list L = {([0], [M_0]), ([1], [M_1]), ..., ([N-1], [M_{N-1}])} using IT-MACs. Indices are public constants authenticated via F_ZK.

2. **Read:** For address handle [ℓ], prover inputs d = M_ℓ to F_ZK, gets [d]. Both parties append ([ℓ], [d]) to L. No correctness check yet — a cheating prover can use wrong d.

3. **Check (after all T reads):** Let k = N + T.
   - (a) Prover sorts L by address to produce L'. Prover authenticates all entries of L' via F_ZK.
   - (b) **Consistency check:** For each adjacent pair in L', verify via Boolean circuit C:
     `(ℓ'_i = ℓ'_{i+1} ∧ d'_i = d'_{i+1}) ∨ (ℓ'_i < ℓ'_{i+1})`
     Also check ℓ'_{k-1} < N. Circuit size: W + 2·log(N) AND gates per pair.
   - (c) **Permutation check via polynomial identity:**
     - Pack each tuple to F_{2^κ}: [[x_i]] := Pack([ℓ_i], [M_i]) and [[x'_i]] := Pack([ℓ'_i], [M'_i])
     - Verifier samples random r ∈ F_{2^κ}
     - Verify ∏_i (x_i - r) = ∏_i (x'_i - r) as an arithmetic circuit over F_{2^κ} with 2(k-1) multiplication gates
     - Soundness error: (N+T)/2^κ (Schwartz-Zippel on degree-(N+T) polynomials)

### Read/Write Array Access (Protocol ZKarray)

**Parameters:** Array size N, word size W, max accesses T, constraint W + ⌈log N⌉ + ⌈log T⌉ ≤ κ.

Extends read-only with timestamps for write ordering:

1. **Access:** Each access appends ([ℓ], [t], [op], [d]) to L, where t is a public counter incremented each access, op ∈ {Read, Write}.

2. **Check:**
   - (a) Prover sorts L by address (primary), then timestamp (secondary), producing L'.
   - (b) **Consistency check** on adjacent pairs via Boolean circuit (Equation 1):
     - Sorted correctly: `(ℓ'_i < ℓ'_{i+1}) ∨ ((ℓ'_i = ℓ'_{i+1}) ∧ (t'_i < t'_{i+1}))`
     - Value consistency: `(ℓ'_i ≠ ℓ'_{i+1}) ∨ (d'_i = d'_{i+1}) ∨ (op'_{i+1} = Write)`
     - First-access-is-write: `(ℓ'_i = ℓ'_{i+1}) ∨ (op'_{i+1} = Write)`
     - Also: first entry must be Write; last address < N.
     - Circuit size: 5 + 2·log(N) + W + log(T) AND gates per pair.
   - (c) **Permutation check:** Same polynomial identity as read-only, packing (ℓ, t, op, d) to F_{2^κ}.

### QuickSilver Optimization for Polynomial Products

The arithmetic circuit for ∏(x_i - r) = ∏(x'_i - r) uses 2(k-1) multiplications over F_{2^κ}. Using QuickSilver's polynomial evaluation technique, communication reduces to ~2(N+T)·κ/h + O(h·κ) bits, where h is a tunable parameter (h=4 in implementation). This gives ~κ/2 bits per access amortized.

### Support for Larger Data Elements

When log(N) + W > 128, instead of using a larger extension field (no hardware acceleration), apply a universal hash H_A: F_{2^128}^c → F_{2^128} where H_A(X_1,...,X_c) = Σ A_i · X_i. Key A chosen by verifier after sorting step. Collision probability ≤ k²/2^{128}. Only requires multiplication by public constants — essentially free in communication.

### RAM Program Execution (Protocol Π_{ZK-RAM})

Layered on FZKarray:
1. Initialize memory via T Write accesses (public addresses, committed values)
2. Each cycle: compute (op, ℓ, d', st') := Π(st, d), prove via Boolean circuit that next-instruction function was applied correctly, then call FZKarray.Access([op], [ℓ], [d'])
3. Total: O(N + T) calls to FZKarray, plus O(T) circuit evaluations of Π

### RISC CPU Architecture

**13-instruction ISA**, 32-bit word size, 32 registers:

| Opcode | Semantics | Category |
|--------|-----------|----------|
| ADD | R[tar] ← R[src0] + R[src1] | Arithmetic |
| SUB | R[tar] ← R[src0] - R[src1] | Arithmetic |
| MUL | R[tar] ← R[src0] · R[src1] | Arithmetic |
| XOR | R[tar] ← R[src0] ⊕ R[src1] | Arithmetic |
| NLG | R[tar] ← (R[src0] ⊕ imm[0]) ∧ (R[src1] ⊕ imm[1]) ⊕ imm[2] | Arithmetic |
| MSK | R[tar] ← ((1 << R[src1]) - 1) ⊕ imm[0] | Arithmetic |
| CSF | R[tar] ← R[src0] ≫ R[src1] (cyclic) | Arithmetic |
| PUT | R[tar] ← src0‖src1‖imm (22-bit immediate) | Arithmetic |
| CMV | R[tar] ← R[src1] if condition, else R[tar] | Arithmetic |
| PC  | R[tar] ← pc; pc ← pc + 1 | Control |
| JMP | pc ← R[src1] if condition, else pc + 1 | Control |
| LDW | R[tar] ← M[R[src0] + imm] | Load/Store |
| STW | M[R[src0] + imm] ← R[src1] | Load/Store |

**Instruction format:** 5-bit opcode | 5-bit tar | 5-bit src0 | 5-bit src1 | 12-bit imm

**Per-cycle ZK proof structure:**
1. Read instruction from program memory (RO-ZKarray): 1 read-only access
2. Read R[src0] and R[src1] from register array (ZKarray): 2 read accesses
3. Compute op and addr = R[src0] + imm, prove correctness
4. Access main memory (ZKarray): 1 access (Read or Write depending on opcode)
5. Evaluate all 11 register-writing instructions in parallel, 11-way multiplexer via one-hot encoding + matrix-vector product (32 inner products)
6. Write result to R[tar] (ZKarray): 1 write access
7. Update pc, prove correctness

**Optimizations:**
- **MSK via one-hot encoding:** OneHot_n uses n-1 AND gates (recursive construction). MSK = one-hot + prefix XOR.
- **CSF (cyclic shift):** Verified via n inner products (degree-2 polynomials, efficient under QuickSilver) rather than n·log(n) AND-gate barrel shifter. Reuses one-hot encoding from MSK.
- **Instruction multiplexing:** One-hot encode opcode, then matrix-vector multiply S·v = s_{opcode} using 32 inner products. Verified efficiently as degree-2 polynomials.
- **Instruction delay:** Expensive ops (MUL, CSF, LDW, STW) evaluated only every n-th cycle (n = public delay parameter). Off-cycle executions delayed by up to n-1 cycles.

### Mixed Boolean + Arithmetic Circuit Support

The protocol naturally combines:
- **Boolean circuits** for consistency checks (comparisons, equality) and CPU instruction evaluation
- **Arithmetic circuits over F_{2^κ}** for permutation checks (polynomial products)
- **Free conversion** between the two via bit-packing of IT-MACs

## Cost Model

### Per-Access Communication

| Component | Read-Only (RO-ZKarray) | Read/Write (ZKarray) |
|-----------|----------------------|---------------------|
| Authentication (input) | W + κ bits | 1 + log(N) + W + κ bits |
| Consistency circuit | W + 2·log(N) AND gates | 5 + 2·log(N) + W + log(T) AND gates |
| Polynomial product (amortized) | ~κ/2 bits (with h=4) | ~κ/2 bits (with h=4) |
| Arithmetic multiplications | 2·(N+T-1) total over F_{2^κ} | 2·(T-1) total over F_{2^κ} |

### Per-Cycle CPU Communication

| Component | Cost |
|-----------|------|
| Program memory read (RO-ZKarray) | 1 read-only access |
| Register reads (ZKarray) | 2 accesses |
| Main memory access (ZKarray) | 1 access |
| Register write (ZKarray) | 1 access |
| Instruction circuit (all 13 ops + mux) | Boolean circuit evaluation |
| **Total (N_data=2^24, N_prog=2^20)** | **~320 bytes/cycle** |

### Communication vs Prior Work (bytes/access, N=2^24, W=32)

| Protocol | Bytes/Access | Scaling |
|----------|-------------|---------|
| BubbleRAM | ~2900 | O(log N) per access |
| BubbleCache | ~240 | O(log N) per access (with cache misses) |
| **This work (ZKarray)** | **~34** | **O(1) per access** |
| Linear scan | N·W bits | O(N) per access |

### Instruction Delay Effect on Communication (bytes/cycle, N_data=2^24)

| Delay | prog=2^10 | prog=2^16 | prog=2^20 |
|-------|-----------|-----------|-----------|
| 1 (none) | 311 | 315 | 338 |
| 2 | 226 | 230 | 253 |
| 4 | 183 | 187 | 210 |
| 8 | 161 | 166 | 189 |

## Concrete Performance

### Memory Access Microbenchmarks (µs, averaged over 10^6 accesses, W=32)

**RO-ZKarray (N=2^15):**

| Component | 25 Mbps | 50 Mbps | 100 Mbps |
|-----------|---------|---------|----------|
| Access | 2.1 | 1.4 | 1.2 |
| Check consistency | 9.5 | 6.6 | 5.8 |
| Check set equality | 5.5 | 3.7 | 3.0 |
| **Total** | **17.8** | **12.4** | **10.8** |

**ZKarray (N=2^15, T=2^25):**

| Component | 25 Mbps | 50 Mbps | 100 Mbps |
|-----------|---------|---------|----------|
| Access | 2.2 | 1.6 | 1.6 |
| Check consistency | 12.8 | 9.2 | 8.6 |
| Check set equality | 5.1 | 3.4 | 2.8 |
| **Total** | **21.7** | **15.7** | **14.4** |

### CPU Memory Component Access Times (µs, at 100 Mbps)

| Memory | Index size (bits) | Time (µs) |
|--------|------------------|-----------|
| Program (RO) | 15 | 10.05 |
| Register (RW) | 5 | 8.26 |
| Main (RW) | 20 | 13.38 |

### CPU Clock Rate

- **6.6 KHz** at 100 Mbps with 64 MB main memory + 4 MB program memory
- BubbleCache: 0.23 KHz at 1 Gbps (same memory sizes) — **~29× slower on 10× more bandwidth**
- Computation becomes bottleneck at ~100 Mbps (no improvement at 500 Mbps)

### MIPS-I Emulation Cost

Most instructions: 1–3 cycles. Worst case: arithmetic shift (srav) = 12 cycles. Full table in Appendix A covers 36 MIPS instructions (excluding exceptions/interrupts).

## Key Definitions

- **IT-MAC:** Information-theoretic MAC where K_x = M_x ⊕ x·Δ (for bits) or K_x = M_x ⊕ x·Δ (for F_{2^κ} elements, with field multiplication). Verifier holds global Δ.
- **[x] / [[x]]:** Authenticated handle for bit x (resp. field element x ∈ F_{2^κ}). Prover holds (x, M_x), verifier holds K_x.
- **Pack([x_0],...,[x_{κ-1}]):** Local (zero-communication) conversion from κ authenticated bits to one authenticated F_{2^κ} element.
- **F_ZK:** Stateful ideal functionality for Boolean and arithmetic circuit ZK proofs, supporting Input, Const, Boolean circuit evaluation, and Arithmetic circuit evaluation.
- **F_{RO-ZKarray}:** Ideal functionality for private read-only array access: Init with committed values, Read at private address, deferred Check.
- **F_{ZKarray}:** Ideal functionality for private read/write array access: Init (empty), Access with [op]/[ℓ]/[d], deferred Check. Timestamps enforce write-before-read and last-write consistency.
- **Polynomial equality check:** L is a permutation of L' iff ∏(x_i - R) = ∏(x'_i - R) as formal polynomials. Tested at random r with soundness error k/2^κ.
- **Consistency check:** Adjacent sorted entries with same address must have same value (read-only) or the second must be a Write (read/write).
- **Instruction delay:** Public parameter n; expensive instructions (MUL, CSF, LDW, STW) evaluated only every n-th cycle to reduce average cost.
- **One-hot encoding circuit:** OneHot_n maps log(n)-bit integer to n-bit vector with single 1. Uses n-1 AND gates via recursive construction.
- **Next-instruction circuit Π:** Given (state, data_in), outputs (op, address, data_out, new_state). Defines one cycle of RAM computation.

## Relevance to VOLE-Based zkVM

1. **Establishes the polynomial-equality permutation check as the fundamental alternative to sorting networks** for RAM consistency in VOLE-ZK. This is the core technique that all subsequent VOLE-based RAM work (Two Shuffles, etc.) builds upon or replaces.

2. **Proves O(N+T) total complexity is achievable** for interactive ZK over RAM programs using VOLE-based IT-MACs, setting the asymptotic baseline.

3. **The free bit-packing property of VOLE IT-MACs** (Boolean [x_i] → field [[x]] with zero communication) is essential to the protocol's efficiency. This must be preserved in any VOLE-based zkVM design.

4. **The read-only vs read/write array separation** (RO-ZKarray for program memory, ZKarray for registers and data memory) is a practical architectural pattern: program memory uses cheaper read-only checks, while registers and main memory use timestamped read/write checks.

5. **The RISC CPU design pattern** — evaluate all instructions in parallel, then multiplex via one-hot encoding — is the template used by subsequent VOLE-based zkVM work and should inform ISA design choices.

6. **The instruction delay optimization** (batching expensive ops every n cycles) trades program generality for throughput. A zkVM design must decide whether to adopt this or handle all instructions uniformly.

7. **34 bytes/access for 64 MB memory** is the concrete baseline to beat. Two Shuffles improves this by 2–20×, but this paper's numbers remain useful for understanding the cost structure (authentication vs consistency vs permutation).

8. **~320 bytes/cycle and 6.6 KHz** set the CPU throughput baseline. The bottleneck shifts from communication to computation at ~100 Mbps, meaning a zkVM design targeting higher throughput must optimize computation (not just communication).

9. **The QuickSilver degree-optimization** (using polynomial evaluation to reduce arithmetic circuit communication from 2k multiplications to ~2k·κ/h + O(hκ) bits) is critical for making the permutation check practical. Parameter h trades computation for communication.

10. **Mixed Boolean + arithmetic circuit support** is not optional — the consistency checks require Boolean comparisons while the permutation check requires F_{2^κ} arithmetic. A VOLE-based zkVM must support both and convert between them efficiently.

11. **The timestamp-based write ordering** (sort by address then timestamp, check last-write consistency) is simpler than ORAM but requires the prover to perform a full sort of all accesses at check time. This is the main cost that Two Shuffles aims to reduce.

12. **Statistical security parameter:** soundness error is (N+T)/2^κ with κ=128. For a zkVM processing billions of cycles, this may need attention (though 2^{128} provides ample margin).
