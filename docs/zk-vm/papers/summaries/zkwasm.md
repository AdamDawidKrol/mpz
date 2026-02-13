# ZKWASM: A ZKSNARK WASM Emulator

**Authors**: Sinka Gao, Guoqiang Li, Hongfei Fu
**Venue**: IEEE Transactions on Services Computing, Vol. 17, No. 6, November/December 2024
**DOI**: [10.1109/TSC.2024.3422798](https://doi.org/10.1109/TSC.2024.3422798)

## Overview

ZKWASM is a ZKSNARK-backed virtual machine that emulates WebAssembly execution and generates zero-knowledge proofs of the execution result. It is the first system to implement the full WASM semantics (except floating-point) in arithmetic circuits. The proof system is built on Halo2's Plonkish constraint system with KZG polynomial commitments. The system supports 148 WASM instructions (64-bit), handles the full stack machine model, and includes a program partition and proof batching mechanism for long execution traces.

## Execution Model

### State Representation

The WASM runtime state is a tuple:

```
S = (iaddr, F, M, G, SP, I, IO)
```

| Component | Description |
|-----------|-------------|
| `iaddr` | Current instruction address, indexed by `(moid, mmid, fid, iid)` — module ID, memory block instance ID, function ID, instruction offset within function |
| `F` | Calling frame, with a `depth` field tracking call stack depth |
| `M` | Memory state (linear memory) |
| `G` | Global variables |
| `SP` | Operand stack |
| `I` | WASM image, containing code section `C` and initial memory `H` |
| `IO` | Host functions (stdin/stdout); used for private/public input |

### Valid Execution Trace

Given input `(I(C, H), E, IO)` where `E` is the entry point, an execution trace is a sequence of state transition functions `[t_0, t_1, t_2, ...]` where:

1. `t_0` matches the semantics of `op(E)` (the instruction at the entry point).
2. For all `k`, state `s_k = t_{k-1} ∘ ... ∘ t_1 ∘ t_0(s_0)`, and `t_k` enforces the semantics of `op(s_k.iaddr)`.
3. The final state `s_e` has `s_e.F.depth = 0` (all call frames returned).

The output is `s_e.IO.output`. It is valid if and only if there exists a valid execution trace producing it.

### Privacy Model

Standard WASM has no private variables. ZKWASM adds privacy through host functions:
- `read_private(index)` — reads a private input (not leaked in the proof).
- `get_public_input(index)` — reads a public input (visible to verifier).

Public inputs are placed in a separate column with polynomial lookup constraints linking them to the execution circuit. Private inputs are placed into witness cells with **no constraints** — the proof system's zero-knowledge property hides them.

## Circuit Architecture

ZKWASM uses Halo2's Plonkish constraint system. The architecture has four stages and multiple interacting circuit tables.

### Stage 1: Image Setup

The WASM image is encoded into lookup tables:

- **Code table `T_C`**: Maps `(moid, mmid, fid, iid) → opcode`. Encodes the entire code section. Used to verify each instruction in the execution trace is a valid instruction from the image.
- **Initial memory table `T_H`**: Maps `(ltype, mmid, offset) → (value, isMutable)`. Encodes both the initial memory section and the global data section. `ltype` is either `Memory` or `Global`. Used to verify all init entries in the memory access log match the image.

### Stage 2: Execution Trace Generation

A standard WASM interpreter executes `(I, E, IO)` and generates the trace `[t_0, t_1, ...]`. The interpreter is **not a trusted component** — if it produces an invalid trace, the circuit constraints will reject it.

### Stage 3: Circuit Synthesis

The trace fills the main execution circuit `T_E`, together with:
- `T_F` — calling frame table
- `T_M` — memory access log table (covers memory, stack, and global accesses)
- `T_SP` — stack access log table
- `T_G` — global access log table

### Stage 4: Proof Generation

Halo2's proof system generates a ZKSNARK proof from the synthesized circuits.

## Execution Trace Circuit (`T_E`)

The execution circuit is the main table. Each WASM instruction occupies a **fixed-size block** of rows (4 rows in practice). The block layout:

| Column | Row 0 (start) | Row 1 | Row 2 | ... | Row n (next start) |
|--------|---------------|-------|-------|-----|-------------------|
| `start` | `true` | `0` | `0` | ... | `true` |
| `opcode` | `op` | `mop_0` | `mop_1` | ... | next `op` |
| `bit cell` | `b_0` | `b_1` | `b_2` | ... | `b` |
| `state` | `tid_0` | `frame` | `s_3` | ... | `tid_1` |
| `aux` | `aux` | `aux_0` | `aux_1` | ... | `aux` |
| `address ∈ T_I` | `iaddr_0` | `addr_0` | `addr_1` | ... | `iaddr_1` |
| `sp ∈ T_F` | `sp` | ... | ... | ... | `sp'` |
| `u64 cell` | `w_0` | `w_1` | `w_2` | ... | `w_0'` |

### Per-Instruction Semantics as Micro-Operations

Each opcode's semantics is decomposed into a sequence of **micro-operations**:

```
mop_i = {
    w_i = load(ltype, addr)              // memory/stack/global read
    write(ltype, addr, v)                // memory/stack/global write
    w_i = arith(p_1, ..., p_k, w_0, ..., w_{i-1})  // arithmetic
    FallThrough                          // advance to next instruction
    Goto(iaddr)                          // jump
    if b then {mop...} else {mop...}     // conditional
}
```

where `addr ∈ {p_1, ..., p_k, w_0, ..., w_{i-1}}` (addresses can be operands or prior witnesses).

### Common Constraints on All Instructions

Every instruction block enforces:

1. **Instruction validity**: `(iaddr, opcode) ∈ T_C` — the instruction exists in the code section.
2. **Stack reads**: if operand `p_i` is read from stack at `sp`, then `(sp, read, iaddr, k, p_i) ∈ T_M`.
3. **Memory reads**: if witness `w_i` is loaded from address `addr_j`, then `(mem, addr_j, ltype, k, w_i) ∈ T_M`.
4. **Bit column**: all cells in the bit column are 0 or 1.
5. **Range**: all cells in the u64 witness and operand columns are in `T_64` (< 2^64).

### Instruction Constraint Composition

Each opcode `op_i` defines its own constraint `C_{op_i}`. The constraint is guarded by the start flag and opcode match:

```
C_{op}(cur) = start_cur × (opcode_cur == op) × C_{op_i}(cur) = 0
```

The final constraint for the execution table is the sum over all opcodes:

```
C_{op}(cur) = Σ_i start_cur × (opcode_cur == op_i) × C_{op_i}(cur) = 0
```

This means at each row, only the constraint for the actual opcode is active; all others multiply to zero.

## Memory Access Model

### Access Log Representation

Every state access (memory, stack, global) is recorded as an access log tuple:

```
(tid, mid, accessType, address, value)
```

| Field | Meaning |
|-------|---------|
| `tid` | Transition index (which instruction in the trace) |
| `mid` | Micro-operation index within that instruction |
| `accessType` | `Init`, `Read`, or `Write` |
| `address` | Access address |
| `value` | The value read/written/initialized |

### Memory Access Table (`T_M`)

The access log is **rearranged** into a lookup table `T_M` sorted by `(address, (tid, mid))`. Within each address block, entries are ordered by their occurrence time.

### Consistency Constraints on `T_M`

The table is equipped with constraints (equation 6 in the paper):

```
C_T = {
    address_cur == address_next → id_cur ≤ id_next
    accessType_cur == init → address_prev ≠ address_cur
    accessType_next == read → value_next == value_cur
    address_cur ≠ address_prev ↔ accessType_cur == init
    address_cur ≠ address_prev → address_cur > address_next
}
```

**What these enforce:**
1. Within an address block, access logs are time-ordered.
2. `init` happens exactly once at the beginning of each address block.
3. A `read` returns the value from the immediately preceding entry (which is the latest write or init).
4. Address blocks are ordered (for uniqueness).

**Theorem 1**: A memory access log `L_i` is valid if and only if there exists a table `T` satisfying these constraints where each `L_i ∈ T`.

**Key insight**: The prover provides the sorted table as a witness. The constraints verify the sorting is correct and reads are consistent, without the verifier needing to sort. The lookup `(tid, mid, accessType, addr, v) ∈ T` in the execution circuit connects each instruction's memory operations to the global access log.

### Unified Memory Model

Memory, stack, and global accesses all use the **same access log mechanism**. They are distinguished by `ltype` (Memory, Stack, or Global) and address namespace `(mmid, offset)`. The stack pointer `SP` is tracked as addresses in the stack region.

## Frame Circuit (`T_F`)

The frame circuit is a separate table for tracking the call stack. Each entry is a tuple:

```
(prevFrame, currentFrame, iaddr)
```

| Field | Meaning |
|-------|---------|
| `prevFrame` | `tid` of the call instruction that started the previous frame |
| `currentFrame` | `tid` of the call instruction that started this frame |
| `iaddr` | Call instruction address of the current frame |

**How it works for return**: When instruction `t_i` is a `return` at state `s_i`, the constraint:

```
plookup(T_F, (prevFrame, currentFrame, s_{i+1}.(iaddr - 1))) = 0
```

verifies that the return address is correct — the next instruction after return goes to the instruction following the call site.

## Instruction Circuits (Details)

### Numeric (Arithmetic) Instructions

General pattern for arithmetic instructions (add, sub, mul, div, etc.):

```
def arithop :=
    param1 = read(stack, sp);
    ...
    paramN = read(stack, sp-N+1);
    result = arith(param1, ..., paramN);
    write(stack, sp-N+1, result);
    sp = sp - N + 1;
    FALLTHROUGH;
```

**Constraint for arithmetic opcodes (`C_arith`)**:

```
C_arith = {
    arith(param_0, param_1, ..., param_N) - result = 0
    plookup(T_M, (stack, read, sp - k, tid, k, param_k)) = 0
    plookup(T_M, (stack, write, sp' - 1, tid, N, result)) = 0
    iaddr_0 + 1 - iaddr_1 = 0
    sp - sp' - N + 1 = 0
}
```

**Example — `add` instruction**: The semantics `w_0 = (w_1 + w_2) mod 2^64` is encoded as:

```
C_add = {
    w_curr + bit_curr × 2^64 - w_next + w_curr[2] = 0
    plookup(T_M, (stack, read, sp_cur, tid, 0, w1)) = 0
    plookup(T_M, (stack, read, sp_prev, tid, 1, w2)) = 0
    plookup(T_M, (stack, write, sp_prev, tid, 2, w0)) = 0
    iaddr_curr + 1 - iaddr_curr[4] = 0
    sp_cur[4] + 1 - sp_cur = 0
}
```

The `bit_curr` witness captures the overflow bit for modular arithmetic.

**Example — `divu` (unsigned division)**: Semantics `divu(a,b) = (a - a mod b) / b`. Encoded with auxiliary witness `r` (remainder) and `k`:

```
C_divu = {
    a = divu(a,b) * b + r
    b = r + k + 1
    a, r, b, k ∈ T_64
}
```

The constraint `b = r + k + 1` enforces `r < b` using range checks (since all values are constrained to be in [0, 2^64)).

### Control Flow Instructions

**Three categories**: FallThrough (covered by arithmetic), branch, and call/return.

**Call instruction (`C_call`)**:

```
C_call = {
    plookup(T_M, (stack, write, sp+i, tid, i, param_i)) = 0   // push params
    plookup(T_F, (tid, pFrameId, iaddr_0)) = 0                  // register frame
    iaddr_1 - targetIaddr = 0                                    // jump to callee
    sp' - sp - N = 0                                             // adjust SP
    nFrameId - tid = 0                                           // new frame ID = current tid
}
```

**Return instruction (`C_return`)**:

```
C_return = {
    plookup(T_F, (pFrameId, nFrameId, iaddr_1 - 1)) = 0   // find return address from frame table
    sp' - sp = 0                                             // restore SP
}
```

**Branch instruction (`C_branch`)**: Abstracted as three steps: read parameters from stack, compute target address via `select`, jump. The branch condition determines the target:

```
C_branch = {
    plookup(T_M, (stack, write, sp+i, tid, i, param_i)) = 0
    iaddr_1 - select(param_0, param_1, ...) = 0
    nFrameId - pFrameId = 0
}
```

Covers `br`, `br_if`, `if/then/else`, `br_table` — all unified into the same pattern with different `select` logic.

### Memory (Stack, Global) Instructions

All memory-class instructions (load/store for memory, stack, and globals) use the same abstraction:

```
(category, ltype, address, size = 8|16|32|64, value)
```

The constraint is simply:

```
(category, ltype, tid, address, value') ∈ T_M  ∧  trunc(value', size) = value
```

For reads, this ensures the loaded value matches the access log. For writes, the access log guarantees the next read returns the written value.

## Type Representation

WASM types are `i32` and `i64`, which don't match the Halo2 scalar field `F`. ZKWASM uses **range check tables**:

- `T_N` contains elements `{0, 1, ..., 2^N - 1}`.
- A value `x` is constrained to be a valid `i32` by `plookup(T_32, x) = 0`.
- For `i64` (too large for a single table), the value is decomposed into byte-sized pieces, each range-checked against `T_8`.

## Customized Instruction Extension

ZKWASM supports two optimization mechanisms:

### Inline Custom Instructions

When multiple WASM instructions have simple combined semantics, they can be fused into a single instruction block. Example: SHA256 uses sequences like `x, y, (x&y)|(complete(x)&z)` which takes 4 standard instructions (4 rows each = 16 rows). An inline custom instruction encodes all the arithmetic in one block (4 rows).

**SHA256 case study (Table XII)**:

| Original sequence | Original rows | Customized | Optimized rows |
|---|---|---|---|
| `x, y, (x&y)\|(complete(x)&z)` | 4 | `ch(x, y)` | 1 |
| `x, y, z, s\|(zk(y\|z))` | 2 | `maj(x, y, z)` | 1 |
| `x, rotr32(x, 2)\|rotr32(x, 13)\|rotr32(x, 22)` | 5 | `lsigma0(x)` | 1 |
| `x, rotr32(x, 6)\|rotr32(x, 11)\|rotr32(x, 25)` | 5 | `lsigma1(x)` | 1 |

### Foreign Function Extension

For complex multi-instruction sequences that can be abstracted as a pure function, a **foreign function** is a separate special-purpose circuit. The execution trace records the call; a separate circuit proves the function's input/output relationship. Saves trace rows but adds circuit overhead.

## Program Partition and Proof Batching

For long execution traces that exceed Halo2's circuit size limit, ZKWASM splits the trace into chunks and proves each chunk separately.

### Splitting the Execution Trace

Given trace `[t_0, t_1, ...]`, split into chunks `t_{[a,b]} = t_a, t_{a+1}, ..., t_{b-1}`. Each chunk has its own:
- Memory access sub-log `M_{[a,b]}`
- Stack access sub-log `SP_{[a,b]}`
- Global access sub-log `G_{[a,b]}`

Each chunk is proved independently: `P_{[a,b]}` proves `t_{[a,b]}` is valid under `(F, M_{[a,b]}, SP_{[a,b]}, I(C, H))`.

### Glue Instructions

To maintain the connection between consecutive chunks, a **glue instruction** (pad) is appended at the end of each sub-sequence. The glue instruction's address equals the first instruction's address of the next chunk. This preserves the polynomial constraints that span instruction boundaries.

### Memory Access Log Stitching

Each chunk has its own memory access sub-table `T_{M_k}`. An **access glue table** `T_{GM}` is constructed containing the boundary entries: the last entry per address in `T_{M_k}` and the first entry per address in `T_{M_{k+1}}`. The global table `T_M = ∪ T_{M_k}` is valid if all sub-tables and the glue table satisfy the memory consistency constraints (equation 6).

### Batching Condition

The batched proof `P_batch` proves valid execution if:

1. Each `P_k` proves `t_{[a_k, b_k]}` is valid.
2. `t_{b_{k+1}}.iaddr = t_{a_k}.iaddr` when `k > 0` (chunks are contiguous).
3. `t_{b_k}` is a glue instruction (except the last chunk).
4. `T_{GM}` satisfies the memory consistency constraints.

The verifying algorithm for each `P_k` is written into arithmetic circuits `V_k`, and the batch circuit combines all verifiers with the continuity checks.

## Concrete Performance

### Single Segment Benchmarks

Each instruction occupies 4 rows. Circuit sizes tested: 2^18 through 2^22 rows.

**Simple functions (Fibonacci, binary search, hash functions)**:

| Circuit size | Max trace size | Synthesize time | Proof time | Verify time |
|---|---|---|---|---|
| 2^18 | 49,076 | 0.19s | 1.2s | 22ms |
| 2^19 | 110,259 | 0.6s | 2.4s | 24ms |
| 2^20 | 150,084 | 0.9s | 5s | 22ms |
| 2^21 | 440,737 | 2.2s | 9.8s | 22ms |
| 2^22 | 862,349 | 4s | 17.9s | 29ms |

**General purpose functions (JSON decoder/encoder, syntax parser)**:

| Circuit size | Max trace size | Synthesize time | Proof time | Verify time |
|---|---|---|---|---|
| 2^18 | 63,134 | 0.22s | 1.2s | 22ms |
| 2^19 | 109,368 | 0.6s | 2.4s | 22ms |
| 2^20 | 240,523 | 1.2s | 5s | 22ms |
| 2^21 | 420,426 | 2.1s | 9.8s | 22ms |
| 2^22 | 970,862 | 4.6s | 17.9s | 29ms |

**Comparison with RISC0 (1 million instructions, Nvidia 4090 GPU)**:

| ZKVM | Instruction set | Trace size | Synthesize time | Proof time |
|---|---|---|---|---|
| RISC0 | 32 instructions (32-bit) | 1M | 1.25s | 13s |
| ZKWASM | 148 instructions (64-bit) | 1M | 4.6s | 17s |

### Proof Time Breakdown (1M instructions)

| Component | Time |
|---|---|
| Witness MSM (61 columns) | 1.667s |
| Lookup (17 cols) and permutation MSM | 6.5s |
| Polynomial evaluation | 6.963s |
| Misc evaluation | 1.292s |
| Multi-open | 1.728s |

MSM (multi-scalar multiplication) is ~40% and NTT is ~23% of total proving time.

### Proof Batching Performance

| Circuit size k | Synthesize time | Proving time | Verify time |
|---|---|---|---|
| 2^22 | 5s | 6s | 4.7ms |
| 2^23 | 7s | 13s | 4.63ms |

With `k = 2^23`, the system achieves ~2^18 instructions per second for large programs. The pipeline recursively batches `n` segments at a time.

## Key Design Decisions

1. **Stack machine preserved**: ZKWASM does **not** compile to a register representation. The operand stack is modeled directly as a memory region with stack pointer tracking. All stack operations go through the unified memory access log.

2. **Uniform instruction block size**: Every instruction occupies exactly 4 rows in the execution circuit, regardless of complexity. This simplifies the constraint system (constraints are row-based and all instructions share the same matrix structure) at the cost of wasting rows for simple instructions.

3. **Unified memory model**: Memory, stack, and globals all use the same access log mechanism with the same consistency constraints. Differentiated only by `ltype` and address namespace.

4. **Sort-then-lookup memory proving**: The prover sorts the access log by address and provides it as a witness table. The verifier checks the sorting constraints and uses polynomial lookup to connect execution circuit entries to the sorted table. This is the standard permutation-based RAM approach adapted to Plonkish circuits.

5. **Code image as lookup table**: The entire WASM code section is encoded as a table `T_C`. Each instruction in the execution trace is verified against this table, ensuring the trace corresponds to the actual program.

6. **Proof batching via IVC**: Long traces are split into chunks, each proved independently, then batched using accumulation schemes. This enables unbounded execution length.

## Relevance to VOLE-Based zkVM Design

1. **Execution trace structure**: ZKWASM's execution trace model — a sequence of state transitions with per-instruction micro-operations — is the same conceptual model needed for the execution plan. The state tuple `(iaddr, F, M, G, SP, I, IO)` maps to our VM state. The difference is that ZKWASM materializes the *entire* trace as a circuit witness, while our execution plan would capture only the symbolic portion.

2. **Memory access log pattern**: The sorted-access-log approach to memory consistency (sort by address, enforce read-after-write within address blocks) is conceptually similar to Two Shuffles RAM's approach, though implemented differently (polynomial lookup vs. permutation proof). The access log tuple `(tid, mid, accessType, address, value)` is a useful reference for what our `MemAccess` records need to contain.

3. **Stack as memory region**: ZKWASM treats the operand stack as a memory region with the same access log mechanism as linear memory. This is relevant to our decision to compile the stack away to registers — ZKWASM's approach shows the alternative (keeping the stack) and its costs (every push/pop is a memory access requiring a lookup).

4. **Frame circuit for call stack**: The separate frame table `T_F` with `(prevFrame, currentFrame, iaddr)` tuples is a concrete implementation of call stack tracking. This is directly relevant to our open question OQ4 (call stack structure). ZKWASM uses a linked-list-like structure where each frame points to its predecessor.

5. **Uniform vs. variable instruction blocks**: ZKWASM uses fixed 4-row blocks for every instruction. This simplifies constraint composition but wastes space. Our plan allows variable-size blocks, which is more efficient but requires more complex dispatch (exactly what Batchman/Tight ZK CPU address).

6. **No taint/visibility concept**: ZKWASM has no equivalent of the VC spec's taint system. All values are either public inputs or private witnesses — there's no notion of "concrete vs. symbolic" execution or folding concrete operations. The entire trace is proved, not just the symbolic portion. This is the fundamental difference from our execution plan approach, where concrete computation is folded away.

7. **Proof batching model**: ZKWASM's trace partitioning and glue-instruction approach to proof batching is relevant to our open question about incremental proving. The key insight is the "glue" mechanism: chunks must overlap at boundaries (the last entry of chunk k connects to the first entry of chunk k+1) to maintain continuity of both the execution trace and the memory access log.

8. **Custom instruction extension**: The inline custom instruction and foreign function mechanisms show how to optimize hot instruction sequences. For a VOLE-based zkVM, the analog would be specialized circuits for common patterns (hash functions, signature verification) — similar to what AntMan's SIMD mode offers.

9. **No branching optimization**: ZKWASM does not optimize for branching — it simply traces every instruction taken. There is no concept of disjunction or hiding which branch was taken. All control flow is explicit in the trace. This is acceptable for their SNARK setting (non-interactive, public verifier) but would not work for our interactive ZK setting where the execution path may need to be hidden.
