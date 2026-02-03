# Zero Knowledge Memory-Checking Techniques for Stacks and Queues

**Authors**: Alexander (Sasha) Frolov
**Venue**: ePrint 2024 | **ePrint**: [2024/2084](https://eprint.iacr.org/2024/2084)

## Role in the VOLE-ZK Design Space

This paper presents specialized memory-checking techniques for stacks and queues that are substantially cheaper than general-purpose RAM schemes. The queue scheme uses the coefficient hash (Horner's rule polynomial evaluation) to check vector equality of enqueues vs dequeues, completely avoiding permutation checks, at **3 mult gates + 1 advice per dequeue** and **2 mult gates per enqueue**. The stack scheme optimizes the Yang-Heath RAM construction by exploiting the paired push/pop access pattern inherent to stacks: pushes and pops are already permutations of each other (no dummy operations needed), the clock only increments on pushes (halving the timestamp set), and initialization costs vanish (values can only be popped after being pushed), achieving **5 mult gates + 4 advice per pop** and **2 mult gates per push**. For a VOLE-based zkVM executing Wasm, the operand stack, call stack, and control-flow label stack are all stacks, and structured data flows (e.g., operand passing) can be modeled as queues, making these cheaper primitives directly applicable to the dominant memory access patterns.

## Key Ideas

### Universal Hash Functions

Two polynomial-based hash functions are used throughout:

- **Root hash**: `H_r(k, x) = prod_{i=0}^{n-1} (k - x_i)`. Universal hash for multisets with collision probability `n/|F|`. Incrementally evaluated by multiplying running product by `(k - x_i)`.
- **Coefficient hash**: `H_c(k, x) = sum_{i=0}^{n-1} k^i * x_i`. Universal hash for vectors (order-sensitive) with collision probability `(n-1)/|F|`. Incrementally evaluated via Horner's rule: `H_c(k, x ++ x_n) = k * H_c(k, x) + x_n`.

The root hash checks multiset (permutation) equality; the coefficient hash checks vector (ordered) equality. Both are evaluated at random points sampled independently of prover's advice.

### Operation Privacy: Conditional Push/Pop

Rather than a single multiplexed `access(op, addr, val)` as in RAM, this paper proposes **conditional push** and **conditional pop** with a guard bit `g`:

- `conditional_push(s, v, g)`: pushes `v` if `g = 1`, no-op if `g = 0`
- `conditional_pop(s, g)`: pops and returns a value if `g = 1`, no-op if `g = 0`

This makes access patterns data-dependent (the nontrivial case) while being easier to program with than a single multiplexed operation that always mutates state. If access patterns are statically known, a trivial zero-cost wire-connection scheme suffices.

### Queue Scheme

The queue maintains:
- `enqueues`: running coefficient hash of enqueued values
- `dequeues`: running coefficient hash of dequeued values
- `queue-depth`: integer counter tracking current depth
- `depth-check`: running product to verify the queue is never read when empty
- `r`: random challenge point (sampled independently of advice)
- `alpha`: random vector in `F^l` for hashing `l`-tuples to field elements

**Enqueue** (`g = 1`):
```
q.enqueues += g * (q.enqueues * r + <v, alpha> - q.enqueues)
q.queue-depth += g
```
Cost: 2 multiplications (1 for Horner step, 1 for conditional guard), 0 advice.

**Dequeue** (`g = 1`):
```
v <- input()                          // advice: the value to dequeue
q.depth-check *= (q.queue-depth + (1 - g))  // 0 iff depth=0 and g=1
q.dequeues += g * (q.dequeues * r + <v, alpha> - q.dequeues)
q.queue-depth -= g
return v
```
Cost: 3 multiplications (1 Horner, 1 guard, 1 depth-check), 1 advice value.

**Teardown**:
1. Drain remaining entries: for each remaining entry, input value as advice, update `dequeues` via Horner step.
2. Assert `depth-check != 0` (queue was never read empty).
3. Assert `queue-depth = 0` (queue fully drained).
4. Assert `enqueues = dequeues` (vector equality of all enqueued/dequeued values).

**Soundness mechanism**: `enqueues` and `dequeues` are evaluations of degree-`T` polynomials at random point `r`. If the i-th enqueued value differs from the i-th dequeued value, the coefficient vectors differ. By Schwartz-Zippel, collision probability is `(T - 1)/|F|` for `l = 1`, or `(T - 1 + l)/|F|` for `l`-tuples using the multivariate form.

**Key property**: No permutation checking is needed. The coefficient hash directly enforces vector (ordered) equality, which is exactly what queue FIFO semantics require.

### Stack Scheme (Optimized Yang-Heath)

The stack borrows two primitives from the Yang-Heath RAM paper:

1. **`ro-kvs-set`** (read-only key-value set): For a set of `M` elements with `N` queries, costs `M + N` advice values, two fan-in `M + N` multiplication gates, and `O(M + N)` linear gates. Used to prove membership in `{0, ..., T}` for timestamp validation.
2. **Permutation check** (`~`): For vectors of `T` elements, checks `root_hash(x, r) = root_hash(y, r)` at random `r`. For `l`-tuples, first takes inner product with random `alpha in F^l`. Cost: `2T` multiplication gates.

The stack maintains:
- `pushes`: list of records `{address, value, time}`
- `pops`: list of records `{address, value, time}`
- `valid-diffs`: set `{0, ..., T}` for timestamp range checks
- `clock`: monotonically increasing counter (starts at 1)
- `depth`: current stack depth

**Setup**:
```
valid-diffs <- setup-set(0, ..., T)
return Stack { {}, {}, valid-diffs, 1, 0 }
```

**Push** (conditional, `g = 1`):
```
s.depth <- s.depth + g
if (g = 1) { s.pushes.append({s.depth, w, s.clock}) }
s.clock <- s.clock + g
```
Cost: 2 multiplications (1 for if/append, 1 for permutation check amortized), 0 advice.

**Pop** (conditional, `g = 1`):
```
val <- input_l()                           // advice: popped value
t   <- input_1()                           // advice: timestamp of matching push
prove-member(s.valid-diffs, s.clock - t)   // range check: clock - t in {0,...,T}
addr <- s.depth
if (g = 1) { s.pops.append({addr, val, t}) }
s.depth <- s.depth - g
return val
```
Cost: 5 multiplications (1 for if/append, 1 for permutation check amortized, 3 for set range check amortized), 4 advice values (value `l=1`, timestamp, 2 for set membership).

**Teardown**:
1. For each remaining entry `i` in `[s.depth]`: input `val`, `t` as advice; append `{i, val, t}` to `pops`.
2. Check `s.pushes ~ s.pops` (permutation check).
3. Teardown the set.

**Three key optimizations over generic RAM**:

1. **No dummy operations**: In Yang-Heath RAM, each access generates both a read and a write entry, requiring dummy entries to balance. In a stack, every push is eventually matched by a pop (assuming the stack is emptied), so `pushes` and `pops` are naturally the same length. This halves the permutation proof cost.

2. **Clock increments only on push**: The timestamp set for Yang-Heath RAM ranges over `{1, ..., T}` where `T` is the total number of accesses. For the stack, only pushes increment the clock, so the set is `{0, ..., T}` where `T` is the number of pushes (roughly half the total operations). This halves the set membership cost.

3. **No initialization/setup writes**: In Yang-Heath RAM, each address needs an initial write entry during setup (costing `O(n)` operations). Since stack values can only be read after being written (pushed), no initial entries are needed.

**Soundness mechanism**: The permutation check on `pushes ~ pops` enforces that each popped `{addr, val, t}` matches some pushed `{addr, val, t}`. The timestamp set membership check `clock - t in {0, ..., T}` ensures the pop reads a value that was pushed in the past. Together these enforce the RAM Invariant from Yang-Heath: the prover must read the most recently pushed value at each depth.

**Handling non-empty stacks at teardown**: If the stack is not empty, teardown adds remaining entries directly to `pops` without calling `prove-member`, since all pushes occurred at earlier clock values by construction.

### Baseline Comparisons

**HashStack (Reef)**: Hash-chaining with Poseidon. Digest `D' = H(v, D)` per push; verify `D = H(v, D')` per pop. Constant overhead, 1 advice per pop, but each access requires a full Poseidon evaluation (~hundreds of constraints).

**WireStack (Reef)**: Maintains `n` wires (max stack depth), linear pass per push/pop. Better than HashStack for small stacks but `O(n)` per access.

**RAM-based stack**: Use Yang-Heath RAM plus a depth counter and depth-check product. Adds 1 multiplication per pop over the RAM scheme's per-access cost. Totals **7 mult + 4 advice per pop** when instantiated with Yang-Heath RAM.

## Cost Model

### Per-Operation Gate Counts (l = 1, conditional operations)

| Operation | Mult Gates | Advice Values | Linear Gates |
|-----------|-----------|---------------|-------------|
| Queue enqueue | 2 | 0 | O(l) |
| Queue dequeue | 3 | 1 | O(l) |
| Stack push | 2 | 0 | O(l) |
| Stack pop | 5 | 4 | O(l) |

### Average Per-Access Cost (assuming balanced reads/writes)

| Scheme | Avg Mults/Access | Avg Advice/Access |
|--------|-----------------|-------------------|
| Queue | 2.5 | 0.5 |
| Stack | 3.5 | 2 |
| Yang-Heath RAM | 6 | 4 |
| RAM-based stack | 4.5 | 2 |

### Detailed Stack Cost Breakdown (T pushes, T' pops, max depth n, l = 1)

| Component | Multiplications | Advice Values |
|-----------|----------------|---------------|
| Pop: permutation check (pushes ~ pops) | T (amortized from fan-in T) | 0 |
| Pop: if statement guard | T' | 0 |
| Pop: set range check | 3T (amortized from 2 fan-in 2T) | 2T |
| Pop: value + timestamp input | 0 | 2T |
| Push: if statement guard | T | 0 |
| Push: permutation check | T' (amortized from fan-in T) | 0 |
| Teardown | n | (l + 1)n |
| **Total (T >> n)** | **5T + 2T'** | **4T** |

### Detailed Queue Cost Breakdown (T enqueues, T' dequeues, l = 1)

| Component | Multiplications | Advice Values |
|-----------|----------------|---------------|
| Enqueue: Horner step | T | 0 |
| Enqueue: conditional guard | T | 0 |
| Dequeue: Horner step | T' | 0 |
| Dequeue: conditional guard | T' | 0 |
| Dequeue: depth-check | T' | T' |
| **Total** | **2T + 3T'** | **T'** |

### Comparison: Stack Implementations

| Scheme | Mults/Pop | Advice/Pop | Mults/Push | Advice/Push | Notes |
|--------|----------|-----------|-----------|------------|-------|
| **This work (stack)** | **5** | **4** | **2** | **0** | Polynomial-based |
| RAM-based stack (Yang-Heath) | 7 | 4 | 2 | 0 | Generic RAM + depth check |
| HashStack (Reef) | ~hundreds | 1 | ~hundreds | 0 | Poseidon hash per access |
| WireStack (Reef) | O(n) | 0 | O(n) | 0 | n = max stack depth |

## Concrete Performance

No implementation benchmarks are provided in this paper. The cost model is stated purely in terms of multiplication gates and advice values.

**Estimated throughput** (extrapolating from Yang-Heath benchmarks at ~600K RAM accesses/sec at 1 Gbps):

| Data Structure | Mults/Access (avg) | Estimated Relative Speedup vs RAM |
|----------------|-------------------|----------------------------------|
| Queue | 2.5 | ~2.4x fewer mults |
| Stack | 3.5 | ~1.7x fewer mults |
| Yang-Heath RAM | 6 | baseline |

## Key Definitions

- **Coefficient hash** `H_c(k, x)` -- Polynomial `sum k^i * x_i` evaluated at random `k`. Universal hash for vectors with collision probability `(n-1)/|F|`. Used in the queue scheme to check FIFO ordering via Horner's rule incremental evaluation.
- **Root hash** `H_r(k, x)` -- Polynomial `prod (k - x_i)` evaluated at random `k`. Universal hash for multisets with collision probability `n/|F|`. Used in the stack scheme's permutation check on push/pop records.
- **Conditional push/pop** -- Guard-bit-controlled operations: `g = 1` executes, `g = 0` is a no-op. Provides data-dependent access patterns (operation privacy) without the awkwardness of a single multiplexed access that always mutates state. Costs 1 extra multiplication per operation for the guard.
- **Depth-check** -- Running product `depth-check *= (queue-depth + (1 - g))`. Equals zero iff the queue/stack was ever read when empty (depth = 0 and g = 1 simultaneously). Verified nonzero at teardown. Cost: 1 multiplication per read.
- **`ro-kvs-set`** -- Read-only set primitive from Yang-Heath. For `M` elements and `N` queries: `M + N` advice, two fan-in `M + N` multiplications, `O(M + N)` linear gates. Used in the stack scheme for timestamp range checking.
- **Permutation check (`~`)** -- Checks `root_hash(pushes, r) = root_hash(pops, r)` at random `r`. For `l`-tuples, first takes inner product with random `alpha`. Cost: `2T` multiplications for vectors of length `T`.
- **Paired accesses** -- The key structural observation: in a stack, every popped value was previously pushed, so pushes and pops are already natural permutations of each other. This eliminates the dummy operations required in generic RAM schemes.
- **Teardown** -- Final phase that drains remaining elements, asserts consistency checks (hash equality for queues, permutation check for stacks), and verifies depth-check is nonzero. Costs are `O(n)` where `n` is the remaining depth; assumed negligible when `T >> n`.

## Relevance to VOLE-Based zkVM

1. **Direct primitive for Wasm operand stack.** Wasm is a stack machine: every ALU instruction pops operands and pushes results. The stack scheme's 5 mults + 4 advice per pop and 2 mults per push directly replaces a generic RAM scheme (7 mults + 4 advice per pop) for the operand stack, saving ~2 multiplications per pop instruction.

2. **Call stack implementation.** Wasm's call stack (storing return addresses and local frames) follows strict LIFO discipline. The stack scheme eliminates the address-space initialization overhead of a generic RAM for the call stack and avoids dummy operations, since every call frame pushed is eventually popped on return.

3. **Queue for structured data flow.** Any FIFO-ordered data channel in the VM (e.g., function argument passing, structured I/O buffers) can use the queue scheme at 2.5 mults/access average, roughly 2.4x cheaper than generic RAM.

4. **Composability with Yang-Heath RAM.** The stack scheme reuses the `ro-kvs-set` and permutation check primitives from Yang-Heath. A zkVM that already implements Yang-Heath RAM for linear memory can share the set infrastructure (timestamp range `{0, ..., T}`) between the stack and RAM, amortizing setup costs.

5. **No permutation check for queues.** The queue scheme uses only the coefficient hash (Horner evaluation), completely avoiding the grand-product permutation machinery. This simplifies implementation and may reduce round complexity in a VOLE-ZK setting where permutation checks require additional challenge rounds.

6. **Conditional operations match Wasm control flow.** Wasm's `br_if`, `select`, and similar conditional instructions naturally map to conditional push/pop with a guard bit derived from the condition. The guard-based interface avoids the need to always mutate state, which would complicate control-flow merging in the zkVM circuit.

7. **Cost savings compound across the execution trace.** A typical Wasm program's execution trace is dominated by stack operations (operand push/pop per instruction). If 60-70% of memory accesses are stack operations, switching from generic RAM (6 mults/access) to the specialized stack (3.5 mults/access average) reduces total memory-checking overhead by ~25-40%.

8. **Wasm control-flow label stack.** Wasm's structured control flow uses a label stack for `block`/`loop`/`if` constructs. This is a strict LIFO structure that benefits from the stack scheme, with labels pushed on block entry and popped on block exit or branch.
