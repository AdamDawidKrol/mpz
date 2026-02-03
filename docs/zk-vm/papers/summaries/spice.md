# Spice: Proving the Correct Execution of Concurrent Services in Zero-Knowledge

**Authors**: Srinath Setty, Sebastian Angel, Trinabh Gupta, Jonathan Lee
**Venue**: OSDI 2018 | **ePrint**: [2018/907](https://eprint.iacr.org/2018/907)

## Role in the VOLE-ZK Design Space

Spice introduces a **timestamp-based offline memory checking** technique (SetKV) that is the direct ancestor of the memory-checking approach used in Jolt and Lasso. While Spice itself is a full verifiable state machine (VSM) system built atop zkSNARKs, its core contribution relevant to VOLE-based zkVM design is the storage primitive: replacing Merkle-tree-based memory verification (cost logarithmic in state size per operation) with a set-based audit mechanism where each read/write costs a constant number of constraints and correctness is checked via a single batch audit comparing multiset fingerprints. This is the technique Jolt adopts for read-write memory (registers, RAM). The audit reduces to checking `RS ∪ M = WS` where RS is the read-set, WS is the write-set, and M is the final memory state -- implemented via an incremental multiset hash (`MSet-Mu-Hash`) over an elliptic curve. In a VOLE context, this pattern translates to: authenticate all (address, value, timestamp) tuples with IT-MACs, then verify memory consistency via a grand product / permutation argument over those tuples.

## Key Ideas

### Offline Memory Checking via Sets (SetKV)

The central insight: instead of verifying each storage operation individually (via Merkle tree traversal costing `O(log n)` hash computations per operation), maintain two sets -- a **read-set** (RS) and a **write-set** (WS) -- and batch-verify all operations with a single audit check.

**State representation**: The key-value store `K` stores tuples `(k, v, t)` where `k` is a key, `v` is the value, and `t` is a Lamport timestamp indicating the last time the key was read or updated.

**Verifier state**: The verifier `VK` maintains only:
```
struct VKState {
    SetDigest rs;  // incremental hash of RS
    SetDigest ws;  // incremental hash of WS
    int ts;        // local Lamport clock
}
```

This is tens of bytes regardless of state size.

### Operations Protocol

**insert(s, k, v)**:
1. Increment `ts' = s.ts + 1`
2. Send `(k, v, ts')` to prover via RPC(INSERT)
3. Update `ws' = s.ws ⊙ H({(k, v, ts')})`
4. Return updated VKState

**get(s, k)** (put is identical except line 4 uses the new value):
1. Receive `(v, t)` from prover via RPC(GET)
2. Add to read set: `rs' = s.rs ⊙ H({(k, v, t)})`
3. Update timestamp: `ts' = max(s.ts, t) + 1`
4. Write-back: send `(k, v, ts')` via RPC(PUT)
5. Add to write set: `ws' = s.ws ⊙ H({(k, v, ts')})`
6. Return updated VKState and value `v`

Critical observation: every get/put adds one element to RS and one to WS. The write-back in step 4 re-writes the same key-value pair but with a fresh timestamp, ensuring RS "trails" WS by exactly the last write to each key.

**audit(s)**:
1. Request full state `M = {(k_i, v_i, t_i)}` from prover
2. For each tuple in M, accumulate into `rs'`
3. Check: (a) no duplicate keys in M, and (b) `rs' == s.ws`

The check is: **RS ∪ M = WS**, where M is the final memory snapshot.

### Why the Audit Works (Correctness Invariants)

1. **Uniqueness**: Every element added to RS and WS is unique because `ts` is incremented after each operation.
2. **RS trails WS**: After all operations, RS contains all tuples that were read (with their timestamps at read-time), and WS contains all tuples ever written (including the write-backs from reads). RS ⊆ WS, with the difference being exactly one entry per key (the most recent write).
3. **Cheating detection**: If the prover returns incorrect value `v' != v` for a get on key `k`, the tuple `(k, v', t)` enters RS but `(k, v, t)` was what was actually in WS. Now RS ⊄ WS, so no M can make RS ∪ M = WS. The prover has "permanently damaged" its ability to pass audit.

### Timestamp Mechanics (vs. BEG+91)

The original Blum-Evans-Gemmell-Kannan-Naor (BEG+91) offline memory checking uses **counters**: each memory cell has a counter incremented on each access. The read-set/write-set approach is:
- **Init**: For address `a` with initial value `v`, write `(a, v, 0)` to WS.
- **Read**: Read `(a, v, t)` from memory, add to RS. Write `(a, v, t+1)` back, add to WS. Counter `t` increments by 1 per read.
- **Final**: Read all cells, adding final `(a, v, t_final)` to RS.
- **Check**: WS = RS (as multisets).

**Spice's modification (timestamp-based, adopted by Jolt)**:
- Uses a **global Lamport clock** `ts` instead of per-cell counters: `ts' = max(ts, t) + 1`
- The timestamp `t` returned by the prover is the timestamp from the *last* access to that cell
- This global clock approach is necessary for **concurrent/multi-writer** settings: different writers share the same address space but maintain independent VKState objects
- Spice also supports **insert** of new keys (BEG+91 has fixed memory size), adding the uniqueness check on keys during audit

### Multiset Hash Function (MSet-Mu-Hash)

The hash function `H` acts on (multi)sets and produces a set-digest. Two properties:

1. **(Multi)set collision-resistant**: Computationally infeasible to find two distinct (multi)sets that hash to the same digest.
2. **Incremental**: Given digest `d_S = H(S)` and a set `W`, compute `H(S ∪ W) = d_S ⊙ H(W)` in time linear in `|W|`.

**Instantiation**: MSet-Mu-Hash from Clarke et al., defined over an elliptic curve `EC`:

```
H({e_1, ..., e_l}) = Σ_{i=1}^{l} H(e_i)
```

where `H(e) = φ(R(e))` maps an element to a curve point:
- `R(·)`: random oracle mapping elements to `F_p`, instantiated with MiMC block cipher (low multiplicative complexity: ~167 constraints)
- `φ(·)`: map from `F_p` to `EC`, instantiated with Elligator-2 (~105 constraints)
- `⊙` is elliptic curve point addition (~8 constraints)

**Multiset vs. set**: Spice's construction is multiset collision-resistant. Using point addition (not XOR) for `⊙` means adding the same element twice does **not** cancel out. This is critical for the concurrent setting where different writers may produce duplicate entries.

**Spice's relaxation**: Spice proves that `H(·) = φ(R(·))` need not be a full random oracle. It suffices for `R(·)` to be a random oracle and `φ(·)` to be an efficiently invertible surjection meeting mild preimage-size conditions. This allows using the cheap Elligator-2 map instead of the expensive Farashahi et al. construction.

### Splitting Ψ_req and Ψ_audit (Amortization)

The key cost optimization: Spice splits the program into two parts:
- **Ψ_req**: Processes one request. Calls get/put but does NOT call audit. Cost: constant number of constraints per storage operation (~1,500 constraints for a get or put).
- **Ψ_audit**: Called once per batch of `m` requests. Verifies RS ∪ M = WS. Cost: `O(n)` constraints where `n` = number of key-value pairs in state.

Proving `m` requests = proving `m` instances of Ψ_req + 1 instance of Ψ_audit. The audit cost `O(n)` amortizes to `O(n/m)` per request.

**Amortized per-operation cost**: `1,500 + 582M/m` constraints, where M = number of key-value pairs. For 1M key-value pairs and m = 800,000 operations: `1,500 + 728 ≈ 2,228` constraints per operation.

### Concurrent Memory Checking (C-SetKV)

For concurrent execution with `l` writers (threads):
- Each thread `j` maintains an independent `VKState^(j)` with its own `rs`, `ws`, `ts`
- After all threads complete, combine VKState objects: `rs_comb = rs^(0) ⊙ ... ⊙ rs^(l)`, similarly for `ws`
- Combined timestamp is 0 (not used in audit)
- Run single audit on combined VKState

**Why this works**: Sets are unordered, `⊙` is commutative, so combining set-digests from independent writers is correct.

**Duplicate entry problem**: Different threads start with `ts=0`, so two threads reading the same key can produce identical `(k, v, t)` tuples in their local RS/WS. Two solutions:
1. Use **multiset** collision-resistant hash (Spice's primary approach): tracks multiplicity, so duplicate entries don't cancel
2. Assign unique thread IDs: tuples become `(k, v, ts, tid)`, guaranteed unique via Lamport clock

### Lock/Unlock Primitives for Transactions

Spice decomposes get into lock + unlock to support transactions:

**lock(s, k)**: Execute the first RPC of a get (GET), add to RS, update ts. Prover must block other operations on key `k` until unlock.

**unlock(s, k, v)**: Execute the second RPC (PUT with new value), add to WS, increment ts.

Transactions: `beg_txn` acquires locks on all keys, `end_txn` releases all locks. Serializability guaranteed by two-phase locking (acquire all before releasing any).

## Cost Model

### Per-Operation Constraint Costs

| Operation | Constraints | Notes |
|-----------|------------|-------|
| Random oracle R(·) on 32-byte message | 167 | MiMC-based |
| Map φ(·) to elliptic curve | 105 | Elligator-2 |
| EC point addition (⊙) | 8 | |
| commit to 32-byte message | 168 | MiMC-based, 300x cheaper than HMAC-SHA256 |
| Single get or put (Ψ_req) | ~1,500 | Constant, independent of state size |
| Single insert | ~1,500 | Similar to get |
| Ψ_audit (total) | ~582M | M = number of key-value pairs in state |
| **Amortized get/put** | **1,500 + 582M/m** | m = batch size |

### Comparison: Per-Storage-Operation Constraints

| System | n=1 | n=10^3 | n=10^6 |
|--------|-----|--------|--------|
| Pantry (Merkle tree) | 4,100 | 44,900 | 85,700 |
| Pantry+Jubjub | 2,100 | 3,000 | 3,000 |
| Geppetto | 8,200 | 89,800 | 171,500 |
| **Spice (Ψ_req only)** | **1,500** | **1,500** | **1,500** |
| Spice Ψ_audit (÷ m) | 1,250/m | 561K/m | 582M/m |

For n = 1M key-value pairs: Spice is 57x fewer constraints than Pantry, 2,000x fewer than Geppetto (for the per-request part). Audit amortization: m ≥ 6,920 operations suffices to beat Pantry overall.

### Argument Protocol Costs

| Component | Cost |
|-----------|------|
| Prover CPU-time per constraint | ~149 μs |
| Verifier CPU-time per proof | ~3 ms |
| Proof size | 128 bytes |

### Parallelized Audit (MapReduce)

For 1M key-value pairs, 1,024 mappers + 33 reducers on 1,024 CPU cores:
- Audit proof generation: 3.63 minutes
- Proof size: `(M + R + 1) × 128 bytes` = (1024 + 33 + 1) × 128 ≈ 135 KB
- Verification time: `(M + R + 1) × 3 ms` ≈ 3.2 CPU-seconds

## Concrete Performance

### Throughput (operations/second, state = 1M key-value pairs)

| Cores | get (uniform) | put (uniform) | get (Zipfian) | put (Zipfian) |
|-------|--------------|--------------|--------------|--------------|
| 1 | 4 | 4 | 4 | 4 |
| 16 | 46 | 46 | 47 | 47 |
| 64 | 163 | 170 | 161 | 172 |
| 256 | 496 | — | 447 | — |
| 512 | 711 | 683 | 648 | 686 |

Near-linear speedup: 379x on 512 cores (uniform), 180x (Zipfian, due to lock contention).

### Throughput Comparison (512 cores vs. baselines)

| System | get ops/sec | put ops/sec |
|--------|-----------|-----------|
| Pantry | 0.078 | 0.039 |
| Pantry+Jubjub | 0.153 | 0.076 |
| Geppetto | 0.002 | 0.002 |
| Spice (1 thread) | 3.6 | 3.6 |
| **Spice (512 threads)** | **1,366** | **1,370** |

Spice: 18,000--685,000x higher throughput than prior work.

### Application Throughput (512 cores, uniform distribution)

| Application | Transaction type | Throughput (tx/sec) |
|-------------|-----------------|-------------------|
| Cloud ledger | issue | 1,126 |
| Cloud ledger | transfer | 573 |
| Cloud ledger | retire | 1,134 |
| Payment network | debit | 584 |
| Payment network | credit | 570 |
| Dark pool | submit | 488 |

## Key Definitions

- **Verifiable state machine (VSM)**: A state machine `(Ψ, S_0)` that produces succinct zero-knowledge proofs of correct state transitions. Prover processes requests, generates proofs; verifier checks proofs without re-execution or access to plaintext state.
- **SetKV**: Spice's verifiable key-value store based on set data structures. Verifier state is two set-digests (RS, WS) plus a Lamport timestamp -- tens of bytes regardless of key-value store size.
- **Offline memory checking**: Technique from Blum et al. (BEG+91). Verify correctness of an untrusted memory by maintaining read-set and write-set, checking RS ∪ final_state = WS (or equivalently WS = RS ∪ S). Per-operation cost is constant; verification is a single batch check.
- **Read-set (RS)**: Set of all `(key, value, timestamp)` tuples read from the key-value store, including reads during audit.
- **Write-set (WS)**: Set of all `(key, value, timestamp)` tuples written to the key-value store, including write-backs from reads.
- **Set-digest**: A succinct (fixed-size) fingerprint of a set, produced by an incremental set collision-resistant hash function `H`. The ⊙ operation allows incremental updates: `H(S ∪ W) = H(S) ⊙ H(W)`.
- **MSet-Mu-Hash**: Clarke et al.'s incremental multiset collision-resistant hash. `H_H(M) = Σ_{b∈B} H(b) · M_b` where `M_b` is the multiplicity of element `b` and `H` maps elements to an elliptic curve group. Multiset collision-resistance holds under hardness of discrete logarithm in the curve group.
- **Lamport timestamp / clock**: A logical clock incremented after each operation: `ts' = max(ts, t) + 1` where `t` is the timestamp returned by the prover. Ensures total ordering of operations and uniqueness of all tuples.
- **Audit**: The batch verification procedure. Requests the full memory state `M` from the prover, checks (1) no duplicate keys in M, and (2) `RS ∪ M = WS` via set-digest comparison. Cost is linear in state size but amortized over all operations since the last audit.
- **Ψ_req / Ψ_audit split**: Decomposition of the program into per-request logic (constant cost, no audit) and batch audit logic (linear cost, run once). Proving `m` requests = `m` proofs of Ψ_req + 1 proof of Ψ_audit.
- **C-SetKV**: Concurrent SetKV. Multiple independent verifier instances maintain separate VKState objects; set-digests are combined (via commutative ⊙) before a single audit. Guarantees sequential consistency (and cross-batch linearizability).

## Relevance to VOLE-Based zkVM

1. **Spice's memory checking is the pattern Jolt uses.** Jolt's read-write memory (registers, RAM) uses exactly Spice's timestamp-based offline memory checking: each access produces an `(address, value, timestamp)` tuple, timestamps use a global Lamport clock with `max(ts, t) + 1`, and correctness reduces to a multiset equality check on read-set vs. write-set. Any VOLE-based zkVM implementing Jolt-style memory must implement this pattern.

2. **The multiset equality check maps to a grand product in VOLE.** The audit check `RS ∪ M = WS` is verified via multiset fingerprinting: pick random `τ, γ` and check `Π_i (τ - hash(RS_i, γ)) × Π_j (τ - hash(M_j, γ)) = Π_k (τ - hash(WS_k, γ))`. In a VOLE context, this becomes a grand product over IT-MAC authenticated values -- a sequence of authenticated multiplications. QuickSilver-style protocols can compute this, but the cost is one authenticated multiplication per tuple in RS, WS, and M.

3. **Per-operation cost is constant (plus amortized audit).** The ~1,500 constraints per get/put in Spice's R1CS setting translate to: in a VOLE setting, each memory access requires authenticating one `(addr, value, timestamp)` tuple (3 field elements), computing the Lamport clock update `max(ts, t) + 1`, and accumulating into the grand product fingerprint. The max function requires a comparison, which is the most expensive part in VOLE (bit-decomposition or a lookup).

4. **The `max` function is the key bottleneck for VOLE.** Spice's timestamp update `ts' = max(ts, t) + 1` requires proving `ts' = max(ts, t) + 1`, which means proving either `ts >= t` or `ts < t` and branching. In Jolt, this is handled via a small lookup table. In a VOLE-based system, comparisons are expensive (require bit decomposition). This is a concrete design point: either use a lookup-based approach for `max`, or restructure the timestamp protocol to avoid comparisons (e.g., the BEG+91 counter-based approach where each cell has its own monotonic counter, avoiding the global `max`).

5. **BEG+91 counters vs. Spice timestamps: trade-offs for VOLE.** The original BEG+91 approach uses per-cell read counters (no `max` needed, just increment). This avoids comparisons but requires the prover to supply the correct counter value for each read, which is checked by the multiset equality. Spice's global timestamp adds the `max` operation but enables concurrent writers. For a single-threaded VOLE-based zkVM, BEG+91 counters may be cheaper (no comparison circuits). For concurrent execution, Spice's timestamps are necessary.

6. **Amortization structure transfers.** The Ψ_req / Ψ_audit split -- where per-request proofs are cheap and the expensive audit is batched -- maps directly to VOLE-based zkVM design. Per-instruction proof is cheap (authenticate a few tuples). The grand product verification (analogous to Ψ_audit) is done once at the end of the trace, costing one authenticated multiplication per memory access in the entire trace.

7. **The multiset hash construction matters.** Spice uses MSet-Mu-Hash over an elliptic curve (expensive in constraints but with algebraic structure). In a VOLE setting, the multiset fingerprinting is done differently: the standard approach is Reed-Solomon fingerprinting `H_{τ,γ}(S) = Π_{s ∈ S} (τ - (s.addr + γ·s.value + γ²·s.timestamp))`. This requires only field arithmetic (no elliptic curve), making it significantly cheaper in VOLE. The grand product argument then verifies the products match.

8. **Concurrent memory is relevant for parallel VOLE proving.** If a VOLE-based zkVM parallelizes proof generation across trace segments, each segment's memory accesses produce independent partial read/write sets that must be combined. C-SetKV's approach -- combine via commutative ⊙, then run single audit -- provides the template. In VOLE terms: each parallel prover thread accumulates its own grand product partial products, which are multiplied together at the end.
