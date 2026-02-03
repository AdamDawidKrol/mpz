# Jolt: SNARKs for Virtual Machines via Lookups

**Authors**: Arasu Arun, Srinath Setty, Justin Thaler
**Venue**: EUROCRYPT 2024 | **ePrint**: [2023/1217](https://eprint.iacr.org/2023/1217)

## Role in the VOLE-ZK Design Space

Jolt is **not** a VOLE-based protocol. It uses polynomial commitments (MSM-based) and the sum-check protocol. Its relevance to a VOLE-based zkVM lies in its **ISA decomposition strategy**: it demonstrates how to reduce VM instruction execution entirely to lookups into pre-determined tables, realizing the "lookup singularity" vision. For a VOLE-based zkVM targeting Wasm, Jolt's decomposition patterns -- breaking each instruction into sub-operations verifiable via table lookups -- transfer directly as a design template, even though the underlying proof system differs entirely.

## Key Ideas

### The Lookup Singularity

Jolt realizes Barry Whitehat's vision: front-ends that produce circuits performing **only lookups** into pre-determined tables. For each ISA instruction `f` operating on two W-bit inputs, the "evaluation table" stores `f(x,y)` for all `(x,y) in {0,1}^W x {0,1}^W`, yielding a table of size `2^{2W}` (e.g., `2^{128}` for 64-bit). The key insight is that these tables need never be materialized -- they are structured (decomposable), enabling the Lasso lookup argument to verify lookups efficiently.

The overall Jolt circuit for one CPU step:
1. Simple R1CS constraints identify which instruction to execute and prepare operands
2. A **single lookup** into the giant instruction table performs the actual computation
3. Memory checking is handled separately via offline memory checking

### Instruction Decomposition Strategy

Every RISC-V instruction's evaluation table is shown to be **decomposable**: a lookup into the size-`N` table can be replaced by `~c` lookups into subtables of size `N^{1/c}`.

**Mechanism**: For an instruction `f` on W-bit input `x`:
- Decompose `x` into `c` chunks `X_0, ..., X_{c-1}`, each of size `W/c`
- Evaluate a small number of subtable functions `f_i(X_j)` on each chunk
- Combine results via a "collation polynomial" `g`

**Concrete decompositions**:

| Instruction | Subtables | Collation |
|---|---|---|
| AND/OR/XOR | 1 subtable (bitwise op on W/c bits) | `sum 2^{(W/c)*i} * SubOp(X_i, Y_i)` |
| ADD/SUB | 1 subtable (truncation/range check on (W+1)/c bits) | `sum 2^i * z_i` (lower W bits) |
| SLTU (less-than unsigned) | 2 subtables: `LTU_{W/c}`, `EQ_{W/c}` | `sum LTU(X_i,Y_i) * prod_{j>i} EQ(X_j,Y_j)` |
| SLT (less-than signed) | 4 subtable types (LTU, EQ, sign-bit ops) | As SLTU plus sign-bit handling |
| SLL/SRL/SRA (shifts) | c subtables `SLL_i`, each size `2^{W/c + log W}` | `sum 2^{i*W'} * SLL_i(X_i, Y_0)` |
| MUL/MULU | Multiplication done in-circuit (1 constraint); lookup is range check on result | Same as ADD |
| MULH/DIV/REM | Decomposed into "virtual instruction" sequences using simpler ops | See below |

**Parameters**: For W=64, c=6 gives subtables of size ~`2^{22}`. For W=32, c=3 gives subtables of size ~`2^{22}`.

### Decomposable Tables (Formal)

A table `T in F^N` is **c-decomposable** if there exist `alpha <= k*c` subtables `T_1,...,T_alpha`, each of size `N^{1/c}` and each MLE-structured, plus a multilinear collation polynomial `g`, such that:

```
T[r] = g(T_1[r_1], ..., T_k[r_1], T_{k+1}[r_2], ..., T_alpha[r_c])
```

where `r = (r_1, ..., r_c)` is the input split into c chunks. Each subtable `T_i` must be **MLE-structured**: its multilinear extension (MLE) `~T_i` can be evaluated at any point in `O(log(N)/c)` field operations.

### Virtual Instructions

Complex instructions (MULH, MULHSU, DIV, REM) are split into sequences of simpler "virtual instructions" executed in the zkVM in place of the original. Virtual registers hold intermediate values. Only the final instruction in a sequence modifies the "real" CPU state.

Example -- **DIVU** (unsigned division) becomes 8 virtual steps:
1. `ADVICE -> v_q` (prover provides quotient)
2. `ADVICE -> v_r` (prover provides remainder)
3. `MULU v_q, r_y -> v_{qy}` (compute q*y)
4. `ASSERT_LTU v_r, r_y` (verify r < y)
5. `ASSERT_LTE v_{qy}, r_x` (verify q*y <= x)
6. `ADD v_{qy}, v_r -> v_0` (compute q*y + r)
7. `ASSERT_EQ v_0, r_x` (verify x = q*y + r)
8. `MOVE v_q -> rd` (store quotient)

### Execution Trace Structure

Each CPU step is a tuple of committed elements:

| Element | Bit-width | Purpose |
|---|---|---|
| `opflags[14]` | 1 bit x14 | Instruction-type flags guiding constraint system |
| `opcode[8]` | 1 bit x8 | Instruction opcode bits |
| `rs1, rs2, rd` | 5 bits x3 | Register indices |
| `PC` | log(\|code\|) | Program counter (possibly two for virtual instructions) |
| `step_counter` | log(#steps) | Global timestamp |
| `read_ts_code` | log(#steps) | Timestamp advice for program code memory-checking |
| `read_ts_rs1, read_ts_rs2` | log(#steps) x2 | Timestamp advice for register reads |
| `imm` | W bits | Sign/zero-extended immediate |
| Values at rs1, rs2 | W bits x2 | Register values read |
| Lookup result | W bits | Instruction output |
| Extra advice | W bits | Non-deterministic advice (div/rem only) |
| Subtable outputs | ~10 bits x2c | Lasso subtable lookup results |
| Chunks `C[c]` | ~22 bits xc | Lookup query decomposition |
| Access counts | log(#steps) x2c | Lasso subtable access counters |

Instruction format: `[opcode, rs1, rs2, rd, imm]` plus 14 boolean operation flags.

### Lasso Lookup Argument (High Level)

Lasso is a lookup argument where no party needs to commit to the table `T`, provided `T` is decomposable or MLE-structured.

**Core mechanism** (offline memory checking):
- View table `T` as read-only memory, cells initialized to `T[i]`
- Each memory cell maintains a counter tracking read count
- Every read returns `(value, count)`, followed by a write incrementing the count
- After all reads, a final pass reads all cells
- Correctness reduces to: the **read-set RS** and **write-set WS** are permutations of each other
- Checked via **permutation-invariant fingerprinting**: pick random `gamma, alpha in F`, verify `prod_i (alpha - RS'_i) = prod_i (alpha - WS'_i)` where `RS'_i = cell + gamma*value + gamma^2*count`
- Grand product computed via GKR protocol or related argument

**For decomposable tables**: Lasso automatically splits each lookup into `c` lookups into subtables, applies the simple (table-linear cost) argument to each subtable, then collates results. Prover commits to ~`3cm + c*N^{1/c}` **small** field elements (all in `{0,...,m}` or subtable entries).

### Memory Checking

Three separate memory regions: (1) program code (read-only), (2) registers, (3) RAM (read-write).

**Read-only** (program code, lookup tables): Handled directly by Lasso.

**Read-write** (registers, RAM): Uses Spice's timestamp-based approach:
- Each read returns `(value, timestamp)`
- On write, the new timestamp is `max(old_ts, global_ts) + 1`
- This prevents "out-of-order" attacks where reads are answered with future values
- The `max` function is computed via a small lookup table (size ~`m^2`)
- All `(cell, value, timestamp)` tuples from reads and writes are fingerprinted and checked via permutation argument

Per step: 6 reads from program code memory, 2 register reads, 1 register write, plus up to W/8 byte-level memory operations for loads/stores.

### Combining Tables for All Instructions

Two approaches:
1. **Generalized-Lasso**: Concatenation of MLE-structured tables is itself MLE-structured (via `~T(x,y) = sum_k EQ(k,x) * ~T_{int(k)}(y)`). Simpler to implement, but c committed field elements per lookup become random (full-size) instead of small.
2. **Lasso with reordering**: Group execution trace by instruction type, apply Lasso separately per instruction. Or: define a single collation polynomial `g(w, x) = sum_y EQ(w,y) * g_{int(y)}(x)` where `w` encodes the opcode, routing subtable lookups to the correct subtables via boolean flags.

## Cost Model

### Non-Memory Instructions

| Bit-length | Count | RV32 (W=32, c=3) | RV64 (W=64, c=6) |
|---|---|---|---|
| 1 | 22 | 22 | 22 |
| [2, 12] | 3+2c | 9 | 15 |
| ~22 (2W/c) | 1+c | 4 | 7 |
| log(T) | 4+2c | 10 | 16 |
| W | 5 | 5 | 5 |
| **Total elements** | **35+5c** | **50** | **65** |
| **256-bit equivalents** | | **~5** | **~6** |

### Memory Instructions (Loads/Stores)

Base cost: 36 elements (~3.5 x 256-bit equivalents). Per byte overhead: 12 elements for loads, 13 for stores (~1.5 x 256-bit each).

A load of k bytes: `36 + 12k` elements. A store of k bytes: `36 + 13k` elements.

### Comparison to Prior Work

| System | Target ISA | Committed elements/step | 256-bit equivalents |
|---|---|---|---|
| **Jolt** (RV64) | RISC-V 64-bit | ~65 small elements | **~6** |
| **Jolt** (RV32) | RISC-V 32-bit | ~50 small elements | **~5** |
| RISC Zero | RISC-V 32-bit | >=275 x 31-bit elements | ~34 |
| Cairo | Cairo-VM | ~50 x 251-bit elements | >=13 |
| Plonk (per gate) | Arithmetic circuit | 11 elements (7 random) | ~11 |

Jolt's advantage: nearly all committed elements are **small** (in `{0,...,m}` or `{0,...,2^{W/c}}`), so Pippenger's MSM algorithm exploits this for ~10x speedup over committing to random 256-bit elements.

## Concrete Performance

The paper provides qualitative cost analysis rather than benchmarks (full implementation was not complete at time of writing). The dominant prover cost is MSM computation for polynomial commitments. Verifier cost: `O(log(T) * log(log(T)))` hash evaluations and field operations, plus one polynomial evaluation proof.

## Key Definitions

- **Decomposable table**: `T in F^N` is c-decomposable if lookups into T can be answered via `~c` lookups into `alpha <= kc` subtables of size `N^{1/c}`, combined by a collation polynomial `g`. Each subtable must be MLE-structured.
- **MLE-structured table**: `T in F^N` such that its multilinear extension `~T(r)` can be evaluated at any `r` in `O(log N)` field operations.
- **Subtable**: A small table (size `N^{1/c}`) that processes one chunk of the decomposed input. Examples: `EQ_{W/c}`, `LTU_{W/c}`, `AND_{W/c}`.
- **Lasso**: Lookup argument where prover commits to ~`3cm + c*N^{1/c}` small field elements. No party commits to the full table. Uses offline memory checking + permutation-invariant fingerprinting + grand product arguments.
- **Execution trace**: Per-step record: `(PC, opcode, opflags, rs1, rs2, rd, imm, register_values, lookup_result, timestamps, subtable_chunks, access_counts)`.
- **Virtual instruction**: A synthetic instruction (ADVICE, MOVE, ASSERT, MOVSIGN) used to decompose complex real instructions (MULH, DIV, REM) into verifiable sequences.
- **Collation polynomial**: The multilinear polynomial `g` that combines subtable lookup results into the full instruction result.

## Relevance to VOLE-Based zkVM

1. **Instruction decomposition transfers directly.** Jolt's core contribution -- showing that every RISC-V instruction can be decomposed into subtable lookups on small chunks -- is ISA-level analysis independent of the proof system. Wasm instructions can be similarly decomposed: i32.add, i32.and, i32.shl, etc. all have analogous decomposable evaluation tables.

2. **The key question for VOLE.** Jolt verifies subtable lookups via polynomial commitments + sum-check. In the VOLE setting, the question becomes: can table lookups be efficiently verified using VOLE correlations? Possible approaches:
   - Encode subtable entries as IT-MAC authenticated values, check membership via VOLE-based equality/range checks
   - Use QuickSilver-style lookup gates (QuickSilver already supports lookup arguments over authenticated values)
   - The small subtable sizes (~`2^{22}`) are feasible to materialize as VOLE-authenticated memory

3. **Virtual instruction pattern.** Decomposing DIV/REM into ADVICE + MUL + ADD + ASSERT sequences is directly applicable: the VOLE prover provides quotient/remainder as authenticated advice, then checks are simple multiplications and comparisons -- all cheap in IT-MAC arithmetic.

4. **Memory checking approach.** Jolt's timestamp-based offline memory checking (from Spice/Lasso) can be adapted to the VOLE setting. The core operation -- permutation checking via fingerprinting -- requires computing a grand product, which in VOLE terms means a sequence of authenticated multiplications.

5. **What Jolt's cost model implies.** At ~6 x 256-bit equivalents per CPU step with polynomial commitments, the question for VOLE is whether authenticated multiplications per step can be competitive. Jolt's ~50-65 small elements per step map to ~50-65 authenticated values in VOLE, plus the cost of verifying subtable membership. The multiplicative gates needed are primarily in the R1CS constraints (a few hundred per step) and the grand product computations.

6. **Execution trace structure is reusable.** The per-step tuple format `(PC, opcode, flags, operands, result, timestamps)` and the three-region memory model (code / registers / RAM) apply directly to a Wasm VM trace, with Wasm's operand stack adding a fourth region.
