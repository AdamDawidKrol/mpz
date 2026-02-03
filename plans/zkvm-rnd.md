# VOLE-Based zkVM: Research & Development Plan

This plan covers the remaining R&D phases for a VOLE-based zkVM that proves
correctness of WebAssembly programs. It builds on the completed Phase 1
(literature survey) and Phase 2 (paper summarization), and covers:

1. **Empirical Wasm analysis** -- build tooling and benchmarks to ground design
   decisions in real data
2. **Phase 1 decision review** -- re-validate the 15 settled decisions against
   empirical data
3. **Open question resolution** -- structured approach to answering OQ4-OQ7
4. **Protocol selection** -- produce `decisions.md`
5. **zkVM design** -- produce the full architecture document

The key principle is that **cost analysis must be empirically grounded in real
Wasm programs**, not just paper benchmarks. The Wasm analysis workstream runs
*in parallel* with and *informs* the protocol selection and design phases.

---

## Phase 0: Empirical Wasm Analysis

This phase is new relative to the original process described in
`docs/zk-vm/README.md`. It runs in parallel with Phases 3-4 and provides the
quantitative foundation for all design decisions.

### 0A: Benchmark Program Suite

Build a collection of representative Wasm programs spanning two categories.

**General-purpose benchmarks:**

| Program | Why it matters |
|---------|---------------|
| Sorting (qsort, mergesort) | Heavy comparison + memory access |
| String processing (search, parsing) | Byte-level memory, branches |
| Data structure operations (hash maps, trees) | Pointer-chasing, indirect calls |
| Compression (LZ-family) | Mixed arithmetic/bitwise, large memory |
| JSON/protobuf parsing | Structured control flow, string ops |

**Cryptographic benchmarks:**

| Program | Why it matters |
|---------|---------------|
| AES-128/256 (ECB, CTR) | Lookup tables, bitwise ops, tight loops |
| SHA-256 / SHA-3 | Arithmetic + bitwise mix, structured rounds |
| HMAC | Composition of hash + XOR |
| ChaCha20 | Pure arithmetic, no tables |
| Poly1305 | Large integer arithmetic |
| ECDSA / Ed25519 verification | Field arithmetic, conditional branches |
| RSA modular exponentiation | Large integer multiply |

Each benchmark should be compiled to Wasm (via Rust `wasm32-unknown-unknown` or
`wasm32-wasi`) with and without optimization flags to understand compiler output
variation.

### 0B: Static Analysis Tooling

Build a Wasm analysis tool that, given a `.wasm` binary, produces the following
metrics.

**Instruction-level metrics:**
- Opcode frequency histogram (how often each Wasm opcode appears)
- Opcode bigram/trigram frequencies (common sequences)
- Instruction mix ratios: arithmetic vs bitwise vs memory vs control flow

**Memory access patterns:**
- Load/store frequency and alignment distribution
- Address range and stride analysis (sequential vs random)
- Unaligned access frequency
- Sub-word (i8/i16) access frequency vs word-level (i32/i64)

**Control flow metrics:**
- Basic block size distribution (instructions per block)
- Branch density (branches per N instructions)
- `br_table` fanout distribution
- Call depth histogram and `call_indirect` frequency
- Loop iteration bound analysis (where bounds are statically determinable)

**Register / local variable metrics:**
- Locals count per function distribution
- Globals count per module
- Live range analysis (how many locals are simultaneously live)
- Operand stack depth distribution (before compilation to registers)

**Domain crossing indicators:**
- Sequences where an arithmetic result feeds a bitwise op (and vice versa)
- Estimated A2B/B2A conversion count per instruction window

### 0C: Compilation Transform Analysis

Investigate what Wasm-level or IR-level transformations can improve provability:

- **Register allocation strategies**: how many registers minimize
  wire-forwarding cost?
- **Block merging/splitting**: optimal basic block granularity for Batchman
  dispatch
- **Instruction scheduling**: reordering to minimize domain crossings (group
  arithmetic ops, group bitwise ops)
- **Memory access coalescing**: merging adjacent byte loads into word loads
- **Loop unrolling trade-offs**: unrolling reduces branch overhead but increases
  trace length
- **Dead code elimination in symbolic branches**: if both branches must be
  padded, how much dead code exists?

---

## Phase 3: Protocol Selection (`decisions.md`)

### 3A: Re-validate Phase 1 Decisions

Using empirical data from Phase 0, revisit each of the 15 settled decisions
from `docs/zk-vm/questions.md`. The key ones to stress-test:

| Decision | Risk | What empirical data resolves it |
|----------|------|-------------------------------|
| Binary field preference (Q2.1) | Domain crossing cost may dominate | Phase 0B domain-crossing frequency across benchmarks |
| i32-word RAM granularity (Q5.1) | Sub-word access frequency may be higher than assumed | Phase 0B sub-word access stats |
| QuickSilver over JesseQ (Q3.1) | JesseQ is 3x faster; is battle-tested-ness worth the gap? | Sensitivity analysis: does 3x gate throughput change any system-level conclusion? |
| Batchman + LogRobin++ (Q4.3) | Optimal dispatch depends on branch density and block sizes | Phase 0B control flow metrics |
| Two Shuffles RAM (Q5.2) | RAM may not be the bottleneck if conversion cost dominates | Phase 0B memory access frequency vs compute instruction frequency |

For each decision, produce:
- The original rationale
- New empirical evidence (for or against)
- Reaffirmation or revision with justification

### 3B: Resolve Open Questions

Each open question gets a structured resolution approach.

**OQ4: Call Stack Structure**

- *Empirical input needed*: call depth histogram from Phase 0B,
  `call_indirect` frequency, recursive function prevalence
- *Analysis approach*: cost-model three options (register-forwarding via
  Batchman, explicit stack RAM, ZK Stacks & Queues stack primitive) at the
  observed call depths. Compare per-call overhead for each.
- *Decision criteria*: if typical call depth < K, register forwarding wins; if
  `call_indirect` is frequent, explicit stack is needed; if recursion is rare,
  hybrid may be optimal.

**OQ5: Register File Structure**

- *Empirical input needed*: locals-per-function distribution from Phase 0B,
  live range analysis
- *Analysis approach*: cost-model wire-forwarding N registers through Batchman
  vs RAM-based register access. Compute crossover point: at what N does RAM
  become cheaper per step?
- *Decision criteria*: if 90th-percentile locals count < crossover N,
  wire-forward all; else hybrid (forwarded hot registers + RAM spill area).

**OQ6: Globals Implementation**

- *Empirical input needed*: globals count per module from Phase 0B
- *Expected resolution*: globals count is almost always < 10; treat as
  wire-forwarded registers.

**OQ7: Symbolic Branch Compilation Rules**

- *Empirical input needed*: branch structure distribution from Phase 0B, basic
  block size distribution
- *Analysis approach*: define cost functions for LogRobin++ disjunction vs CPU
  emulation with padding, parameterized by branch body size and nesting depth.
  Plot crossover curves. Match against observed branch patterns.
- *Decision criteria*: produce concrete heuristic rules (e.g., "if both
  branches < K gates, use 2-way LogRobin++; if loop bound unknown, pad to
  declared maximum; etc.").

### 3C: Write `decisions.md`

The protocol selection document. Structure:

1. **Preamble**: security model, assumptions, scope
2. **Protocol stack summary**: one-paragraph overview of the full stack
3. **Per-role decisions** (one section each):
   - Gate checking: QuickSilver (or JesseQ if re-validation warrants it)
   - Instruction dispatch: Batchman + LogRobin++
   - RAM: Two Shuffles RAM
   - Domain conversion: Mystique zk-edaBits
   - Register file: (resolved from OQ5)
   - Call stack: (resolved from OQ4)
   - Globals: (resolved from OQ6)
   - Symbolic branching: (resolved from OQ7)
4. **Excluded protocols** with rationale
5. **Upgrade paths**: JesseQ, Tight ZK CPU, AntMan SIMD, ZK Stacks & Queues
6. **Cost summary table**: per-component costs with references to paper data

---

## Phase 4: zkVM Design Document

### 4A: Execution Model

- Wasm-to-register compilation: how the stack machine maps to a register
  representation
- Step circuit structure: what constitutes one "step" (instruction-level or
  block-level)
- State representation: PC, registers, memory, globals -- what is threaded
  through each step
- Concrete vs symbolic execution: how taint determines the execution mode per
  value

### 4B: Memory Architecture

- **Linear memory**: Two Shuffles RAM with i32-word granularity
  - Load/store instruction mapping (aligned, unaligned, sub-word)
  - Per-byte taint tracking (external)
  - Mystique conversion at the RAM boundary
- **Register file**: (per OQ5 resolution)
- **Globals**: (per OQ6 resolution)
- **Call stack**: (per OQ4 resolution)
- **Memory region composition**: how multiple RAM instances share set structures

### 4C: Instruction Dispatch

- Batchman dispatch loop: topology matrices for each Wasm opcode
- Wire-equality constraints for state threading between steps
- LogRobin++ for isolated disjunctions (symbolic if/else, br_table)
- Compilation rules for symbolic branches (per OQ7 resolution)

### 4D: Domain Crossing

- Where crossings occur (RAM boundary, arithmetic-to-bitwise instruction
  transitions)
- Mystique conversion protocol integration
- Optimization opportunities (instruction reordering to batch crossings)

### 4E: Instruction Cost Table

For each supported Wasm instruction (MVP + bulk memory, i32/i64, no float):

| Metric | Description |
|--------|-------------|
| Multiplication gates | Circuit-level non-linear gate count |
| VOLE correlations | Preprocessing resource consumption |
| RAM accesses | Two Shuffles RAM accesses per instruction |
| Domain conversions | Mystique A2B/B2A conversions required |
| Estimated time | At reference bandwidth (e.g., 1 Gbps) |

Organized by category: arithmetic, bitwise, comparison, memory, control flow,
conversion, bulk memory.

### 4F: Integration Points

- How the zkVM interfaces with the embedder (taint tracking, reveal protocol)
- VOLE preprocessing budget: how many correlations per step, amortized
- Batched reveal protocol design
- Proof finalization (how the batch consistency check works end-to-end)

---

## Workstream Dependencies

```
Phase 0A (benchmarks) ──────────────────────────────┐
Phase 0B (static analysis) ──┬──> Phase 3A (review) ┤
Phase 0C (transforms) ──────┘         |              |
                                      v              |
                              Phase 3B (open Qs) ────┤
                                      |              |
                                      v              |
                              Phase 3C (decisions) <─┘
                                      |
                                      v
                              Phase 4A-4F (design)
```

Phase 0 and the early parts of Phase 3A can run in parallel. Phase 3B depends
on Phase 0B output. Phase 3C depends on 3A and 3B. Phase 4 depends on 3C.

---

## Reference: Settled Phase 1 Decisions

These decisions from `docs/zk-vm/questions.md` are subject to re-validation in
Phase 3A:

| # | Domain | Decision |
|---|--------|----------|
| Q1 | Security model | Malicious, kappa=128, sigma=40, RO acceptable |
| Q2 | Algebraic domain | Binary (F_2 / F_{2^k}), prime field via COPEe where needed |
| Q3 | Gate checking | QuickSilver (circuit + polynomial mode) |
| Q4 | Instruction dispatch | Batchman (batched) + LogRobin++ (isolated) |
| Q5 | RAM protocol | Two Shuffles RAM (prime field), i32-word granularity |
| Q5 | Operand stack | Compiled away (register representation) |
| Q6 | Taint tracking | Public, outside the proof circuit |
| Q7 | Domain conversions | Needed, mechanism deferred (Mystique likely) |
| Q8 | Symbolic branching | Supported, adaptive strategy per branch point |
| Q9 | Symbolic addressing | Supported (Two Shuffles handles private addresses) |
| Q10 | Reveal | Batched, provable via IT-MAC opening |
| Q11 | Floating point | Out of scope |
| Q12 | Wasm features | MVP + bulk memory, per-instruction cost table |
| Q13 | VOLE generation | Black box |
| Q14 | Excluded protocols | AntMan sublinear, Justvengers, VOLE-in-the-Head |

## Reference: Open Questions (from `docs/zk-vm/questions-phase2.md`)

| # | Question | Status | Blocks |
|---|----------|--------|--------|
| OQ1 | RAM granularity | Resolved (i32-word) | -- |
| OQ2 | Domain conversion mechanism | Resolved (Mystique) | -- |
| OQ4 | Call stack structure | TBD | Execution model, OQ5 |
| OQ5 | Register file structure | TBD | Dispatch circuit design |
| OQ6 | Globals implementation | Likely wire-forwarded | Minor |
| OQ7 | Symbolic branch compilation rules | TBD | Compilation model |
