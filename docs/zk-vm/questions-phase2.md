# Phase 2: Open Questions & Cost Analysis

This document identifies questions from Phase 1 that remain open or tentative,
and introduces new questions that emerged from the Phase 1 answers. Each
question is tagged with a **dependency** (what it blocks) and a **resolution
path** (what analysis is needed to answer it).

## Summary of Phase 1 Decisions (Settled)

The following decisions are final and not revisited here:

| Domain | Decision |
|--------|----------|
| Security model | Malicious, κ=128, σ=40, RO acceptable |
| Primary algebraic domain | Binary (F\_2 / F\_{2^k}), prime field via COPEe where needed |
| Gate checking | QuickSilver (circuit + polynomial mode) |
| Instruction dispatch | Batchman (batched) + LogRobin++ (isolated) |
| RAM protocol | Two Shuffles RAM (prime field) |
| Operand stack | Compiled away (register representation) |
| Taint tracking | Public, outside the proof circuit |
| Reveal | Batched, provable via IT-MAC opening |
| Symbolic branching | Supported, adaptive strategy per branch point |
| Symbolic addressing | Supported (Two Shuffles handles private addresses) |
| Floating point | Out of scope |
| Wasm features | MVP + bulk memory |
| VOLE generation | Black box |
| Excluded protocols | AntMan sublinear, Justvengers, VOLE-in-the-Head |
| Document | `decisions.md`, prescriptive, audience = implementer + reviewer |

---

## Open Questions

### OQ1: RAM Granularity (from Q5.1)

**Status:** Resolved — i32-word-level (4 bytes per word).

**Resolution:** With Mystique conversions costed at ~30-45μs each (OQ2), the
per-access conversion overhead makes byte-level RAM prohibitively expensive:
an i32.load costs ~147μs (4 accesses × ~37μs) at byte-level vs ~37μs (1
access) at i32-word-level, a 4x difference.

Comparison of word sizes showed i32-word is optimal for wasm32 workloads:
- i32.load/store (aligned): 1 RAM access — same as i64-word.
- i64.load/store (aligned): 2 RAM accesses — 2x more than i64-word, but
  i64 operations are less common in wasm32 programs.
- i32.store: 1 RAM access — i64-word would require read-modify-write (2
  accesses) since i32.store is a partial write to an i64 word.
- i8/i16.store: read-modify-write in both word sizes.

The i32 word optimizes for the dominant wasm32 case (i32 pointers, i32
arithmetic) while keeping the RAM uniform. Unaligned accesses and sub-word
stores require additional logic (read-modify-write, two-word spanning) but
these are bounded cases handled in the compiler.

**Per-byte taint:** Tracked externally by the embedder as 4 taint bits per
word. On load, if any relevant byte is symbolic, the result is symbolic (per
VC spec). No circuit cost — the embedder maintains the taint map.

**Blocks:** None — resolved.

---

### OQ2: Domain Conversion Mechanism (from Q7.1, Q7.2)

**Status:** Resolved — Mystique zk-edaBits required.

**The tension:** The primary domain is binary but the RAM and permutation
checks operate over a prime field. Every RAM access involves a domain
crossing. The conversion mechanism determines a large fraction of per-
instruction cost for memory-touching instructions.

**Resolution:** Proven domain conversions are required for soundness. A
cheating prover could commit different values in the binary and prime-field
domains if the two representations are not cryptographically linked. Simply
re-committing a value in the other domain (via COPEe) does not prove the
two commitments match. Mystique zk-edaBits provide the proven link:

- **Store (binary → prime):** Mystique B2A converts authenticated binary
  bits `[x_0, ..., x_{k-1}]_2` to a prime-field MAC `[x]_p` with a proven
  consistency guarantee. Cost: 1 zk-edaBit + 1 AdderModp circuit + open m
  bits. ~33-49μs at 200Mbps-1Gbps.
- **Load (prime → binary):** Mystique A2B converts `[x]_p` back to
  authenticated bits. Cost: 1 zk-edaBit + open 1 field element + 1
  AdderModp circuit. ~29-45μs at 200Mbps-1Gbps.
- **Preprocessing:** zk-edaBits are generated via cut-and-bucket (3x
  overhead) independently of the program. This fits the VOLE preprocessing
  model.

COPEe (already implemented) is still used to generate the prime-field VOLE
correlations that back the prime-field MACs. Mystique layers on top to
provide the proven link between domains.

**The Mystique conversion cost must be included in the per-instruction cost
accounting for all memory-touching instructions.** This is the dominant cost
for loads/stores and will significantly influence the RAM granularity decision
(OQ1).

**Blocks:** RAM granularity decision (OQ1), per-instruction cost estimates.

---

### OQ4: Call Stack Structure (from Q5.4)

**Status:** TBD.

**The question:** How does the zkVM handle function calls and returns?

**Options:**
- **Register-forwarding via Batchman's wire-equality constraints** — the call
  stack is implicit. Each function call is a dispatch step (Batchman selects
  the callee's circuit), and return values are threaded through registers.
  The return address is a register value. No explicit call stack RAM.
- **Explicit call stack RAM** — a separate RAM (or stack primitive from ZK
  Stacks & Queues) stores return addresses and saved registers. Function
  calls push a frame, returns pop it.
- **Hybrid** — small fixed-depth calls use register forwarding, deep/recursive
  calls use an explicit stack.

**Considerations:**
- Wasm has structured function calls (no computed gotos), so the set of
  possible callees at each call site is known at compile time.
- `call_indirect` (function pointers via tables) requires dynamic dispatch —
  a disjunction over all functions with matching type signature.
- Recursive and mutually-recursive functions require a stack of unbounded
  depth (up to resource limits).
- The specialized stack primitive (3.5 mults/access vs 6 for RAM) may be
  worth using here if the call stack is explicit.

**Analysis needed:**
1. How deep are typical call stacks in real Wasm programs?
2. Can Batchman's wire-equality constraints handle function call/return
   without an explicit stack?
3. What is the cost of `call_indirect` as a disjunction?

**Blocks:** Full execution model design, instruction cost for call/return.

---

### OQ5: Register File Structure (from Q5.3, Q5.4)

**Status:** Registers exist as a separate structure, details TBD.

**The question:** How is the register file implemented?

**Options:**
- **Two Shuffles RAM** — same as linear memory, just a separate instance.
  Supports random access to any register by index (symbolic or concrete).
- **Wire-equality constraints** — for a fixed set of registers, values are
  threaded between steps via Batchman's register-forwarding mechanism (each
  instruction's topology matrix includes register pass-through). No RAM
  needed.
- **Hybrid** — a small set of "hot" registers are forwarded via wires, and
  a larger spill region uses RAM.

**Considerations:**
- Wasm locals are the register set. The number of locals varies per function
  (typically 0-50, can be up to thousands).
- If register count is small and fixed, wire forwarding is cheapest (zero
  RAM cost per step).
- If register count varies per function or is large, RAM-based registers
  are more flexible.
- The Batchman paper's CPU benchmark uses wire-forwarded registers with a
  fixed count.

**Analysis needed:**
1. What is the distribution of local counts across real Wasm programs?
2. What is the per-step cost of wire-forwarding N registers through Batchman
   vs RAM-based register access?
3. Can the register set be split (some forwarded, some in RAM) without
   excessive complexity?

**Blocks:** Instruction dispatch circuit design, per-step overhead.

---

### OQ6: Globals Implementation (from Q5.4)

**Status:** Likely a separate RAM, details TBD.

**The question:** How are Wasm globals stored and accessed?

**Considerations:**
- Wasm globals are typically few in number (0-10 in most modules).
- They are accessed by static index (not computed), so the access pattern
  is known at compile time.
- If the number is small and indices are static, they can be treated as
  additional registers (wire-forwarded).
- If treated as registers, `global.get`/`global.set` have zero RAM cost.

**Likely resolution:** Treat globals as registers (wire-forwarded) unless the
module has an unusually large number of globals.

**Blocks:** Minor — unlikely to be a cost driver.

---

### OQ7: Compilation Strategy for Symbolic Branches (from Q4.1, Q8.1)

**Status:** Adaptive strategy decided, but the compilation rules are not
defined.

**The question:** What are the concrete rules for deciding, at compile time,
whether a symbolic branch is handled via:
- A 2-way disjunction (LogRobin++/Batchman)
- Full CPU emulation with padding
- Some other approach

**Considerations:**
- Wasm's structured control flow means all branch targets are known at
  compile time. The compiler can analyze the CFG.
- A simple `if/else` with small bodies → 2-way LogRobin++.
- A `br_table` (switch) → B-way Batchman disjunction.
- A loop with symbolic bound → requires CPU emulation (unknown iteration
  count, must pad to a maximum).
- Nested symbolic branches → may compound (2-way × 2-way = 4-way, or
  sequential disjunctions).

**Analysis needed:**
1. Define the heuristic/rules for selecting dispatch mode per branch.
2. What is the cost threshold where CPU emulation becomes cheaper than
   nested disjunctions?
3. How should loop bounds be handled? (Pad to a declared maximum? Abort
   if no bound is known?)

**Blocks:** Design document's compilation model section.

---

## Priority Order

| Priority | Question | Dependency |
|----------|----------|------------|
| 1 | OQ4: Call stack structure | Blocks execution model and OQ5 |
| 2 | OQ5: Register file structure | Blocks dispatch circuit design |
| 3 | OQ7: Symbolic branch compilation rules | Blocks compilation model |
| 4 | OQ6: Globals implementation | Minor |
