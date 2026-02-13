# Symbolic IR

**Status**: Draft
**Date**: February 2026

## Introduction

This document describes the **symbolic IR**, the central data structure
of the zkVM. The symbolic IR is a protocol-agnostic intermediate
representation of the symbolic computation that both parties must prove. It
is the output of partially evaluating a WebAssembly program under the
[Verifiable Compute specification][vc-spec] with respect to concrete inputs.

### Scope

This document defines:

- The structure and semantics of the symbolic IR.
- How VM state flows through the IR.
- How linear memory interacts with the IR via an access log.
- The construction procedure (partial evaluation with concrete folding).
- The padding policy for prover-communicated bounds.

This document does not define:

- Which proof protocols are used (the IR is protocol-agnostic).
- How the IR is compiled to proof obligations (that is the role of the
  **IR compiler**, a separate component).
- The wire format or serialization of the IR.

### Audience

An implementer building the zkVM runtime and IR compiler. A reviewer
evaluating the design for correctness and efficiency.

### Dependencies

- [Verifiable Compute Specification][vc-spec] — defines the two-party model,
  visibility, taint propagation, and embedding interface.
- [WebAssembly Core Specification][wasm-spec] — defines the instruction set,
  type system, and structured control flow that the symbolic IR operates
  over.

[vc-spec]: https://sinui0.github.io/vc-spec/docs/spec
[wasm-spec]: https://webassembly.github.io/spec/core/

---

## Overview

Both parties execute the same Wasm module with the same call configuration.
Taint — whether a value is concrete or symbolic — is a deterministic,
publicly computable property of every value during execution. Both parties
agree, at every point, on which instruction is executing, the taint of every
operand, and the concrete values of all concrete operands.

Fully concrete computation needs no proof. Both parties compute the same
result. These operations are **folded**: they produce no nodes in the IR.
Their results are absorbed as constants into subsequent symbolic operations.

The symbolic IR is the record of everything that was not folded. It is a
tree whose leaves are straight-line sequences of symbolic operations and whose
interior nodes represent symbolic control flow (branches and loops).

The IR is constructed **independently and deterministically** by both
parties during co-execution. No communication is required to agree on the
IR's structure, except for padded loop iteration counts communicated by the
prover (see [Padding Policy](#padding-policy)).

---

## Concepts

**Concrete folding.** When an instruction's operands are all concrete, both
parties execute it locally and obtain the same result. The instruction produces
no node in the IR. Its result is a constant available to subsequent
operations.

**Taint environment.** The mapping from every value-carrying location (locals,
globals, memory bytes) to its taint (concrete or symbolic) and, for concrete
locations, the value. Both parties maintain identical taint environments. The
taint environment is the context in which the IR is constructed, not part
of the IR itself.

**Wire.** An authenticated symbolic value within the IR. A wire carries a
type but no bit pattern — the prover knows the value; the verifier does not.
Wires are the edges of the symbolic IR.

**Interface.** The set of symbolic wires crossing a node boundary. Each node
has an input interface (wires it reads) and an output interface (wires it
produces). Concrete state is not listed — both parties know it from the taint
environment.

**Access log.** The ordered sequence of memory operations (loads and stores)
that occurred during execution. The access log is a flat structure orthogonal
to the IR tree. Every memory-touching node in the IR contributes entries
to a single global access log.

---

## Node Types

The symbolic IR is a tree with three node types.

### Block

A **block** is a straight-line sequence of symbolic operations with no control
flow. Every operation in a block has at least one symbolic operand (otherwise
it would have been folded). A block is the atomic unit of the IR.

Each operation records:

- The **instruction** (a Wasm opcode or fused-operation identifier).
- The **operands**, each either a concrete value or a wire reference.
- The **result type**.

Operations within a block execute in sequence. The output of operation *i* is
referenceable by subsequent operations via a wire reference.

A block also records its **memory access descriptors** — metadata about the
loads and stores that occurred during the block's execution (see
[Access Log](#access-log)).

### Disjunction

A **disjunction** is a symbolic branch point. The branch condition is
symbolic: the verifier does not know which arm was taken (the prover does).

A disjunction contains B **arms**, each a sub-tree of the IR. Both parties
derive each arm independently by walking the Wasm code and applying concrete
folding. The arms may differ in size.

The output interface of a disjunction is fully symbolic for any wire that is
symbolic on *any* arm, per the [VC spec's taint rules][vc-spec] (the branch
condition is symbolic, so the verifier cannot distinguish which arm produced
the output).

### Loop

A **loop** represents repeated computation with a symbolic exit condition. The
verifier does not know the iteration count (the prover does).

A loop records:

- The **body**, a sub-tree describing one iteration.
- The **padded iteration count** N, communicated by the prover.

Both parties analyze the loop body with the known input taints, producing the
body sub-tree. The loop then represents N repetitions of that body. See
[Padding Policy](#padding-policy) for how N is chosen.

---

## Grammar

```
IR          = Node*
Node        = Block | Disjunction | Loop

Block       = { ops: Op*, mem: MemDescriptor*, in: Interface, out: Interface }
Disjunction = { arms: IR*, in: Interface, out: Interface }
Loop        = { body: IR, iterations: u32, in: Interface, out: Interface }

Op          = { instruction: InstructionId, operands: Operand*, result: Type }
Operand     = Concrete(value) | Symbolic(WireRef)

MemDescriptor = { kind: Load | Store,
                  address: Concrete(u32) | Symbolic,
                  value_taint: Concrete | Symbolic,
                  width: u32,
                  offset: u32 }

Interface   = { wires: WireSlot* }
WireSlot    = { id: WireId, type: Type }
```

---

## State Model

The VM has two kinds of mutable state. They flow through the IR by
different mechanisms.

### Register State

**Registers** — Wasm locals, the compiled operand stack, and globals — are
named wires. Register indices are always statically known:

- `local.get`, `local.set`, and `local.tee` use static immediate indices.
- `global.get` and `global.set` use static immediate indices.
- Operand stack depth at every program point is statically determined by
  Wasm's type system.

Registers never have symbolic addresses. Both parties always know which
register is being accessed. Registers are wires in the IR — no RAM proof
is needed.

Each function has its own register namespace. When a function calls another
under concrete control flow, the caller's live symbolic registers are listed
in the interface at the call boundary. The IR does not prescribe how they
are preserved — the IR compiler decides (wire forwarding, RAM-based
save/restore, or a stack structure).

A `call_indirect` with a symbolic table index produces a disjunction over
the possible callees.

### Memory State

**Linear memory** is too large to represent as wires. It is accessed
indirectly via load/store instructions, and its consistency is proved by a
separate RAM argument that operates alongside the IR.

The IR does not track memory contents. Memory state lives in the taint
environment, which both parties maintain during co-execution.

---

## Wire Threading

Register state threads through the IR via **interfaces**. Each node's input
interface lists the symbolic wires it reads; its output interface lists the
wires it produces. Between adjacent nodes in a sequence, the output interface
of node *i* is the input interface of node *i+1*. The IR compiler enforces
wire equality at each boundary.

### Sequential Nodes

For a sequence of blocks `[B₀, B₁, B₂]`:

```
B₀.out = B₁.in
B₁.out = B₂.in
```

Concrete values computed between nodes (by folded operations) are not in the
interfaces — both parties know them.

### Disjunctions

Both arms receive the same input interface. Each arm produces its own output
interface. The disjunction's output interface is the union: any wire that is
symbolic on any arm is symbolic in the output. The IR compiler constrains
the output wires to match whichever arm was actually taken (this is hidden
from the verifier).

### Loops

The body's output interface feeds back into the body's input interface for the
next iteration. The loop's overall input interface feeds the first iteration.
The loop's output interface is the body's output after the final iteration.

```
loop.in  → body.in  (iteration 1)
body.out → body.in  (iteration 2)
...
body.out → loop.out (iteration N)
```

---

## Access Log

All memory operations across the entire symbolic IR contribute to a single,
global **access log**. The access log is a flat, ordered sequence of memory
access records — it is orthogonal to the tree structure of the IR.

The access log has two levels:

- **Public structure.** The *number* of entries and their timestamps are
  determined by the IR, which is public. Both parties can compute these by
  traversing the IR tree (including padded access counts for disjunctions
  and loops).
- **Authenticated contents.** The *addresses*, *values*, access *kinds*,
  and *widths* of each entry are wires — the prover knows them; the
  verifier holds authentication tags but does not see the plaintext. The RAM
  argument operates over these authenticated values.

### Structure

Each entry in the access log is a tuple:

```
(clock, op, address, value, width)
```

| Field     | Visibility | Description |
|-----------|------------|-------------|
| `clock`   | Public     | Monotonically increasing timestamp, derived from the IR structure. |
| `op`      | Wire       | Load (0) or Store (1). Wire because inside a disjunction, different arms may have different access patterns at the same clock position. |
| `address` | Wire       | The memory address. |
| `value`   | Wire       | The value read or written. |
| `width`   | Wire       | Access width in bytes. Wire for the same reason as `op`. |

The **clock** is the only public field per entry. It advances as the IR is
traversed in execution order (depth-first, left-to-right within sequences).
Because the IR structure is public and includes padded access counts for
disjunctions and loops, the clock schedule is deterministic — both parties
know every entry's timestamp without communication.

All other fields are wires. This is necessary because inside a disjunction,
the access at a given clock position depends on which arm was taken. If `op`,
`address`, `value`, or `width` were public, they would leak the arm identity.

> **Note**
> Outside of disjunctions, the `op` and `width` are in practice determined
> by the `MemDescriptor` (which is public). The IR compiler may exploit this
> by folding known-public fields into constants within the RAM argument's
> circuit, avoiding the cost of authenticating them as wires. This is an
> optimization, not a semantic distinction — the access log treats all
> non-clock fields uniformly as wires.

### Relationship to the IR

Each block's `MemDescriptor` records describe the *shape* of the block's
memory accesses: how many, what kind, what width. The descriptors are public
metadata, part of the IR. They determine the access log's public structure
(the number of entries and their timestamps). The authenticated contents
(op, address, value, width) are provided by the prover during proving.

The IR compiler collects all descriptors by traversing the tree, assigns
timestamps from the global clock, and constructs the access log as input to
the RAM argument. The RAM argument proves that every load returns the value
of the most recent store to that address — without the verifier learning
which addresses were accessed or what values were read/written.

### Interaction with Wire State

Memory loads and stores are the boundary where register state and memory
state interact:

- A **load** produces a wire in the containing block. The same wire appears
  in the access log as a read entry. The RAM argument constrains this wire
  to equal the value of the most recent store to that address.
- A **store** consumes a wire from the block. The same wire appears in the
  access log as a write entry.

At these points, the block's circuit and the RAM argument share the same
authenticated value. The IR compiler ensures the wire identity is the same
in both places.

### Disjunctions and the Access Log

Both parties see the IR for both arms of a disjunction, including each arm's
`MemDescriptor` records. Both parties compute the **padded access count** —
the maximum number of memory accesses across all arms. The disjunction
contributes exactly this many entries to the global access log, regardless of
which arm was taken. The clock advances by the padded count.

During proving, the prover fills in the access log entries for the taken arm's
real accesses plus dummy accesses (no-op reads to a fixed address) to reach
the padded count. Because addresses and values are wires (authenticated but
not visible to the verifier), the verifier cannot distinguish real accesses
from dummy accesses and cannot determine which arm was taken from the access
pattern.

The disjunction proof connects the taken arm's block circuits to the
corresponding access log entries, without revealing which arm is active.

### Loops and the Access Log

Each loop iteration contributes its memory accesses to the global log.
Timestamps increase across iterations. The total number of access log entries
from a loop is (accesses per iteration) × N, where N is the padded iteration
count. Both parties know this count from the IR.

The RAM argument does not know or care that these entries came from a loop —
they are entries in the flat log like any others.

### Memory Regions

A symbolic-address store poisons memory taint: after such a store, the
verifier cannot determine which bytes were written, so every byte in the
affected range must be treated as potentially symbolic. A subsequent load from
any address in that range produces a symbolic result.

To contain this, linear memory may be partitioned into **regions**. A
symbolic-address store to region A poisons only region A; loads from region B
retain precise taint. Each region has its own segment of the access log, and
the IR compiler instantiates a separate RAM argument per region.

The mechanism for defining regions (embedder annotations, static analysis, or
custom sections) is an implementation decision. The default is a single region
covering all of linear memory.

---

## Construction

The symbolic IR is constructed by both parties during co-execution of
the Wasm program. The procedure is a walk over the Wasm instructions,
maintaining the taint environment and emitting nodes as symbolic operations
are encountered.

### Instruction Processing

For each instruction during execution:

1. Look up the taint of each operand in the taint environment.
2. Apply the VC spec's taint propagation rules (default rule, annihilator
   exceptions, select rule).
3. If the result is **concrete**: execute the instruction, update the taint
   environment. No node is emitted.
4. If the result is **symbolic**: append an operation to the current block.
   Concrete operands are recorded with their values. Symbolic operands are
   recorded as wire references.

### Control Flow

- **Concrete branch condition**: both parties know which path is taken. They
  follow the taken path. No disjunction is emitted.
- **Symbolic branch condition**: a Disjunction node is emitted. Both parties
  independently derive each arm by walking its code with
  concrete folding.
- **Concrete loop bound**: the loop is unrolled during IR construction. Each
  iteration is processed as straight-line code.
- **Symbolic loop bound**: a Loop node is emitted. The prover communicates the
  padded iteration count.
- **Concrete recursion depth**: the recursion is decomposed into a descent
  loop (D iterations: save frame, compute body prefix), a base case block,
  and an ascent loop (D iterations: restore frame, compute body suffix).
  The frame save/restore is a RAM concern decided by the IR compiler.
- **Symbolic recursion depth**: the prover communicates a padded depth D.
  Once D is known, both parties decompose the recursion into loops — same
  as concrete-depth recursion.

### Block Boundaries

A new block starts when:

- A control flow node (Disjunction or Loop) is encountered.
- A function call boundary is crossed.
- A reveal operation occurs (reveals update the taint environment, which is
  external to the IR, but they cause a block boundary because subsequent
  folding may differ).

Within straight-line code, operations accumulate in the current block. There
is no maximum block size — blocks are as large as the straight-line segments
between control flow points.

### Memory Access Recording

When a memory load or store is processed:

1. Record a `MemDescriptor` in the current block (kind, address taint, value
   taint, width).
2. Assign a clock timestamp from the global counter.
3. Append the full access record (with timestamp, address, and value) to the
   access log.

If the access has a concrete address and a concrete value (for stores) or
all loaded bytes are concrete (for loads), the access is fully concrete and
is folded — no `MemDescriptor` is recorded, no access log entry is needed.

If the access has a symbolic address or involves symbolic bytes, a
`MemDescriptor` is recorded in the block and an entry is added to the access
log.

---

## Padding Policy

Symbolic loop iteration counts and recursion depths are communicated by the
prover before IR construction proceeds past that point. Once communicated, they
are concrete and known to both parties. The padded value is public and leaks
information about the private inputs that determined the actual count. This is
analogous to non-constant-time code leaking information via observable resource
consumption.

The best mitigation is at the source: **guest code should be written with
constant-time control flow wherever possible.** Loops should iterate a fixed,
publicly known number of times. Recursive functions should reach a fixed,
publicly known depth.

When constant-time guest code is not feasible, the padding strategy is
**configurable by the embedder**. The IR records the padded value and is
agnostic to how it was chosen. Reasonable strategies:

| Strategy | Behavior | Leakage | Overhead |
|----------|----------|---------|----------|
| Reject | Abort on symbolic bound | None | None |
| Exact | N = actual count | Full | 0 |
| Power-of-2 | N = next power of 2 | ≤ 1 bit per loop | ≤ 2× |
| Step(K) | N = ceil(actual / K) × K | K-wide range | ≤ K − 1 iterations |
| Max(M) | N = M (fixed bound) | None | M / actual |

These may be combined (e.g., power-of-2 capped at a declared maximum).

The same policy applies to:

- Loop iteration counts.
- Recursion depths (which become concrete after padding, enabling inlining).
- Memory access counts within disjunction arms (padded to the maximum across
  arms).

---

## Derived Quantities

Both parties can independently compute the following from the IR:

| Quantity | Source |
|----------|--------|
| Total symbolic operations | Sum of ops across all blocks |
| Operation frequency histogram | Grouped by instruction type |
| Total memory accesses | Sum of access descriptors across all blocks |
| Memory access profile | Loads vs. stores, concrete vs. symbolic addresses |
| Region usage | Which regions are touched, access counts per region |
| Disjunction count and nesting depth | Tree shape |
| Per-disjunction arm count and arm sizes | Disjunction nodes |
| Loop iteration counts and body sizes | Loop nodes |
| Wire threading width at each boundary | Interfaces |

The **IR compiler** consumes these quantities to produce a proving
strategy: which proof mechanism to use for each node, how to batch nodes,
where to place domain conversions, and how to allocate preprocessing
resources.

---

## What the IR Does Not Contain

- **Private values.** The IR never contains the bit patterns of symbolic
  values. The prover holds those separately as its witness.
- **Which arm was taken** in a disjunction. The IR records the structure
  of all arms; the prover's witness records the actual path.
- **Proof protocol choices.** The IR is protocol-agnostic.
- **Field assignments.** The IR does not specify which algebraic field a
  computation occurs in.
- **Wire layout.** Wire positions in an extended witness vector are a
  protocol-specific concern.
- **Batching strategy.** Whether blocks are merged, iterations are batched,
  or disjunctions are flattened is an IR-compiler decision.
- **Call stack implementation.** How caller registers are preserved across
  function calls is an IR-compiler decision.
- **RAM implementation.** The IR records access descriptors but not which
  RAM protocol is used.
- **Access log contents.** The IR records access *descriptors* (metadata).
  The access log itself — with timestamps and values — is constructed
  alongside the IR but is a separate data structure consumed by the RAM
  argument.

---

## Open Questions

### Loop Body Variation Across Iterations

If a loop body contains concrete state that varies per iteration (e.g., a
concrete loop counter), different iterations may fold differently. Options:

- **Uniform body**: analyze once with conservative taint (treat varying
  values as symbolic). Wastes proof capacity but keeps the IR compact.
- **Per-iteration bodies**: each iteration has its own sub-tree. More
  efficient but O(N) IR size.
- **Grouped by taint signature**: iterations with the same taint pattern
  share a sub-tree.

### Memory Region Definition

How are regions defined? Candidates:

- Embedder-declared boundaries.
- Static analysis of access patterns.
- Custom section annotations (advisory, per the VC spec).
- A single region as the default, with splitting as an optimization.
