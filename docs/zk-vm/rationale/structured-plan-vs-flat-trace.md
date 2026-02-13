# Rationale: Structured Plan vs. Flat Trace

This document records the design rationale for choosing a **structured execution
plan** (tree of blocks, disjunctions, loops, recursions) over a **flat
instruction-level trace** (every symbolic step dispatched through Batchman over
the instruction set). The analysis compares the two approaches on cost, privacy,
and complexity.

## The Two Approaches

### Approach A: Flat trace (instruction-level dispatch)

The prover executes the program, produces a flat sequence of symbolic steps, and
proves each step using Batchman dispatch over the Wasm instruction set. Every
step is one row. The dispatch is a disjunction over opcodes (which instruction
was executed at this step). Loops and recursion require no special treatment —
a loop is just more steps, recursion is just steps with call/return
bookkeeping tracked via a frame table.

This is the approach used by ZKWASM (adapted from their SNARK setting to our
VOLE-based interactive setting). ZKWASM materializes the entire trace; our
adaptation would materialize only the symbolic portion (after concrete folding).

### Approach B: Structured plan (block-level dispatch)

The execution plan is a tree. Straight-line symbolic operations are grouped
into **blocks** and proved directly with QuickSilver — no dispatch. Symbolic
branches produce **disjunction** nodes proved with LogRobin++. Symbolic loops
and recursions are distinct node types with prover-communicated bounds.

Batchman dispatch is used only where needed (e.g., inside disjunction arms if
there are multiple possible code paths). Straight-line blocks — which are the
majority of most programs — pay zero dispatch overhead.

## Cost Analysis

### Parameters

| Symbol | Meaning | Typical value |
|--------|---------|---------------|
| B | Number of distinct Wasm opcodes in the dispatch table | ~200 |
| \|C_max\| | Multiplication gates in the largest instruction circuit | ~50 |
| \|C_avg\| | Average multiplication gates per instruction | ~10 |
| m | Number of registers forwarded per step | ~20 |
| R | Total symbolic steps (after folding) | varies |

### Per-step cost: flat trace with Batchman

Batchman's total cost for R repetitions of a B-way disjunction with branch
circuit size |C| is `O(RB + R|C| + B|C|)`. Amortized per step (for large R),
each step pays:

1. **Extended witness commitment**: `O(|C_max|)` VOLE correlations. All
   instructions are padded to the size of the largest (Batchman requires
   uniform branch size).
2. **Topology vector commitment**: `O(|C_max|)` VOLE correlations over the
   extension field.
3. **Topology validation** (product circuit proving the committed topology
   matches a valid branch): `O(B)` multiplication gates.
4. **Register forwarding**: `m` multiplication gates per step (each register
   is forwarded as a `1 × reg_k = reg_k` mult gate).

Total per step:

```
VOLE_flat ≈ 2(|C_max| + m) + B
          = 2(50 + 20) + 200
          = 340 VOLE correlations
```

### Per-step cost: flat trace with Tight ZK CPU

Tight ZK CPU eliminates the padding to |C_max| — each step pays for the
*actual* instruction size. But it introduces a 6–7× overhead from the UROM
mechanism:

```
VOLE_tight ≈ 7 × (|C_actual| + m)
           = 7 × (10 + 20)
           = 210 VOLE correlations per step
```

### Per-operation cost: structured plan

Within a block (straight-line code), each operation is proved with QuickSilver
circuit mode at its actual size:

```
VOLE_block = 1 VOLE correlation per multiplication gate
```

No dispatch overhead. No topology vectors. No product circuits. No
per-operation register forwarding — registers within a block are wires;
register state is threaded only at block boundaries (via interfaces).

At disjunction nodes, LogRobin++ proves the active arm:

```
VOLE_disjunction ≈ |C_arm| + O(log B_d)
```

where |C_arm| is the active arm's gate count and B_d is the number of arms
(typically 2 for an if/else).

### Concrete comparison

**Scenario**: 10,000 symbolic operations after folding. 8,000 in straight-line
blocks, 2,000 inside 100 two-way symbolic branches (20 operations per arm).

| Approach | Straight-line cost | Branch cost | Total VOLE |
|----------|-------------------|-------------|------------|
| Flat trace (Batchman, B=200) | 8,000 × 340 = 2,720,000 | 2,000 × 340 = 680,000 | **3,400,000** |
| Flat trace (Tight ZK CPU) | 8,000 × 210 = 1,680,000 | 2,000 × 210 = 420,000 | **2,100,000** |
| Structured plan | 8,000 × 1 = 8,000 | 100 × 21 = 2,100 | **10,100** |

**Ratio: structured plan is ~340× cheaper than Batchman, ~200× cheaper than
Tight ZK CPU.**

### Why the difference is so large

Three factors compound:

1. **Batchman's per-step B term.** Every step pays `O(B)` for the topology
   validation product circuit, even when the actual instruction is trivial.
   With B = 200, this is 200 mult gates of overhead on every step, regardless
   of what the step does. The structured plan pays zero dispatch overhead on
   straight-line code.

2. **Padding to the largest instruction.** Batchman requires uniform branch
   circuits, so every step commits a witness of size |C_max| even for a simple
   `i32.add`. (Tight ZK CPU fixes this at 6–7× overhead.) The structured plan
   proves each operation at its actual size with QuickSilver.

3. **Per-step register forwarding.** In Batchman, every step must thread all m
   registers through the circuit as `1 × reg = reg` multiplication gates. In
   the structured plan, registers within a block are named wires — no
   per-operation forwarding. State is threaded only at block boundaries.

### When the flat trace closes the gap

The flat trace wins or narrows the gap when:

- **Almost all code is inside symbolic branches.** If 100% of operations are
  behind symbolic control flow, the structured plan must also dispatch at every
  branch point. But even then, it uses LogRobin++ per-disjunction (`O(log B_d)`
  overhead) rather than Batchman per-step (`O(B)` overhead per step).

- **The instruction set is very small.** If B = 4, Batchman's `O(B)` per step
  is small. But Wasm's ~200 opcodes make this irrelevant.

- **Privacy requires hiding the total amount of computation.** See below.

## Privacy Analysis

### What the structured plan leaks

The plan tree is public — both parties construct it. The verifier sees:

- The tree shape: how many disjunctions, their nesting depth, arm counts.
- Padded loop iteration counts (communicated by the prover).
- Padded recursion depths (communicated by the prover).
- Block sizes (how many symbolic operations per block).
- Interface widths at each node boundary.

The verifier does **not** learn:

- Which disjunction arm was taken (hidden by LogRobin++).
- Private values (never in the plan).
- The concrete execution path within blocks.

### What a flat padded trace leaks

In a flat trace padded to a fixed total length T, the verifier learns:

- T (the padded total trace length). That is the only structural information.
- Nothing about how many loops, branches, or recursive calls occurred.

This is strictly less structural information than the structured plan leaks.

### Assessment

The structural information leaked by the plan (tree shape, iteration counts,
block sizes) is largely derivable from **public data** — the Wasm module and
the taint configuration. The verifier already knows the program and the taint
rules. The tree shape follows deterministically from the program structure and
the visibility annotations in the call configuration.

The only genuinely new information the prover contributes is:

1. **Padded loop iteration counts.** These leak information about private
   inputs that control loop bounds.
2. **Padded recursion depths.** Same concern.

The mitigation for both is **more aggressive padding** — padding to declared
or inferred upper bounds hides the count entirely. This costs extra proof work
proportional to the padding, but far less than switching to a flat trace.

**Quantitative comparison**: Suppose aggressive padding doubles the proof work
(2× overhead). The structured plan at 2× is still ~100–170× cheaper than the
flat trace. The privacy benefit of the flat trace does not justify its cost.

## Complexity Analysis

### Flat trace: simpler conceptually

- No tree structure. No Loop, Recursion, or Disjunction node types.
- Loops and recursion require no special treatment — just more steps.
- One uniform dispatch mechanism for everything.
- Frame table handles call/return (ZKWASM-style).

### Structured plan: more complex but better separated

- Tree structure with four node types (Block, Disjunction, Loop, Recursion).
- Loop iteration counts and recursion depths must be communicated.
- Plan compiler must handle each node type.
- But: straight-line code (the majority) is trivially simple — just a sequence
  of QuickSilver gates. Complexity is concentrated at symbolic control flow
  points, which are the minority.

The structured plan's complexity is proportional to the complexity of the
program's symbolic control flow. For programs that are mostly straight-line
with occasional branches, the plan is simple. The flat trace's complexity is
constant regardless of program structure, but that constant is higher than
necessary for the common case.

## Conclusion

The structured plan is the right approach. The cost advantage (~200–340×
fewer VOLE correlations) dominates the privacy and complexity trade-offs:

- **Privacy**: the information leaked by the tree structure is mostly public.
  Where it is not (loop counts, recursion depths), targeted padding is far
  cheaper than uniform-trace padding.
- **Complexity**: the structured plan is more complex at the plan-compiler
  level, but the proof work for the common case (straight-line blocks) is
  trivially cheap.
- **Loops and recursion**: the structured plan requires dedicated node types,
  which adds design complexity. But this complexity exists because the
  *problem* is structurally complex — the flat trace hides the complexity
  inside the dispatch overhead rather than eliminating it.

### References

- [Batchman](../papers/summaries/batchman-robin.md): `O(RB + R|C| + B|C|)` batched dispatch
- [Tight ZK CPU](../papers/summaries/tight-zk-cpu.md): pay-per-instruction-size dispatch, 6–7× overhead
- [QuickSilver](../papers/summaries/quicksilver.md): 1 VOLE per multiplication gate (circuit mode)
- [LogRobin++](../papers/summaries/logrobin.md): `|C| + O(log B)` per disjunction
- [Two Shuffles RAM](../papers/summaries/two-shuffles-ram.md): ~5.33 VOLE per RAM access
- [ZKWASM](../papers/summaries/zkwasm.md): flat-trace SNARK approach (reference point)
- [Execution Plan](../../../plans/plan-execution-plan.md): the structured plan design
