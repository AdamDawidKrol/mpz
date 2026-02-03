# Two Shuffles Make a RAM: Improved Constant Overhead Zero Knowledge RAM

**Authors**: Yibin Yang, David Heath
**Venue**: USENIX Security 2024 | **ePrint**: [2023/1115](https://eprint.iacr.org/2023/1115)

## Role in the VOLE-ZK Design Space

This paper gives the most gate-efficient constant-overhead ZK RAM for arithmetic circuits: **4 input gates + 6 multiplication gates per access**. The entire RAM reduces to two permutation checks (one for reads-vs-writes, one for set membership timestamps) plus one auxiliary multiplexer, hence the title. Because 5 of the 6 multiplications are high fan-in (grand products inside permutation proofs), QuickSilver's polynomial evaluation mode can compress them, yielding a VOLE-specific optimization that further reduces communication. Concrete VOLE-based implementation achieves ~600K RAM accesses/sec at 1 Gbps and ~50 bytes/access, improving over Franzese et al. by 2-20x and over Delpech de Saint Guilhem et al. by 2.2-2.9x.

## Key Ideas

### Architecture: Three Data Structures

The RAM is built from three nested primitives:

1. **Read-Only Key-Value Store (RO-KVS / ROM)** -- base layer
2. **Set with Membership Queries** -- RO-KVS with 0-length values
3. **Read/Write RAM** -- RO-KVS permutation + set membership for timestamps

### Read-Only Memory (ROM)

The ROM maintains two vectors `reads` and `writes`. On setup, for each address `i` with value `x[i]`, append `(i, x[i], 0)` to `writes` (version 0). On each `lookup(i)`:

1. P inputs: the value `x[i]` and the latest version `v` for address `i`
2. Append `(i, x[i], v)` to `reads`
3. Append `(i, x[i], v+1)` to `writes`

On teardown, for each address `i`, P inputs the final version `v` and appends `(i, x[i], v)` to `reads`. Then check:

```
reads ~ writes    (permutation check)
```

**Why this is sound**: Versions monotonically increase, forcing P to build per-address chains from setup writes to teardown reads. P cannot form cycles (would require `v > v` for some version). P *can* "read from the future" (read a version written later), but this is harmless for ROM since values never change.

**Gate cost (ROM)**: For size `n`, `T` lookups, storing `l`-tuples:
- `(n + T)(l + 1)` input gates
- Two fan-in `(n + T)` multiplication gates (grand products for permutation)
- `O(n + T)` linear gates

### Set Data Structure

A set is a ROM with `l = 0` (0-length values). Initialize an RO-KVS with keys `{1, ..., T}`. To prove membership of value `t`, call `lookup(t)`. If `t` is not in the set, the permutation proof fails (no setup-time write exists for that key).

**Gate cost (Set)**: For set `{1, ..., T}` with `T` membership queries:
- `2T` input gates
- Two fan-in `2T` multiplication gates
- `O(T)` linear gates

### Read/Write RAM

Each `access(op, addr, w)` performs a read *and* a write. Maintains a monotonically increasing `clock` (starts at 1). Vectors `reads` and `writes` store triples `(address, value, time)`.

**Setup**: For each address `i` with initial value `x[i]`, append `(i, x[i], 0)` to `writes`.

**Access** (at time `clock`):
1. P inputs: old value `old` and the time `t` when it was last written
2. **Set membership check**: prove `(clock - t) in {1, ..., T}` (ensures `t < clock`, i.e., read from the past)
3. Compute new value: `new = old + op * (w - old)` (multiplexer: `load` keeps `old`, `store` writes `w`)
4. Append `(addr, old, t)` to `reads`
5. Append `(addr, new, clock)` to `writes`
6. Increment `clock`

**Teardown**: For each address `i`, P inputs `(val, t)`, append to `reads`. Then check:

```
reads ~ writes    (permutation check on length-(T+n) vectors)
```

Also teardown the internal set.

**Key invariant (RAM Invariant)**: Before each access, for each address `i`, `writes` contains exactly one tuple `(i, val, t)` not in `reads`, and that tuple has the highest time among all writes to `i`. The permutation check plus the timestamp set membership enforce that P must read the most recent write.

**Why set membership alone suffices**: P doesn't need to prove "I read the *latest* write" explicitly. The permutation forces each write to be consumed by exactly one read, and the timestamp check forces reads to point backward in time. Together these imply a linked-list structure per address where each read must match the immediately preceding write.

### Gate Cost Summary (RAM per access, assuming `T >> n`)

```
Per access:
  4 input gates           (old value l=1, timestamp t, set version, multiplexer op)
  1 fan-in-2 mult         (multiplexer: op * (w - old))
  5 high fan-in mults     (2 from reads~writes permutation, 2 from set permutation,
                           1 optimized away -- see below)
                           
Total: 4 input gates + 6 multiplication gates
```

Full accounting with setup/teardown:
- `4T + 2n` input gates
- Two fan-in `2T` mults (set) + two fan-in `(T+n)` mults (RAM permutation)
- `T` fan-in-2 mults (multiplexers)
- `O(n + T)` linear gates

### Permutation Checking via Polynomial Identity Testing

Given vectors `x` and `y` of length `n`, define grand-product polynomials:

```
p(X) = prod_i (X - x[i])
q(X) = prod_i (X - y[i])
```

If `x ~ y` then `p = q`. Test by sampling random `r` from V and checking `p(r) = q(r)`. For vectors of `l`-tuples, take a random linear combination with challenge vector `s in F^l`:

```
p(X, Y) = prod_i (X - <Y, x[i]>)
q(X, Y) = prod_i (X - <Y, y[i]>)
```

Cost: `2n - 2` private multiplications (one subtraction and one multiplication per element, minus boundary).

Soundness error from Schwartz-Zippel: `d/|F|` where `d = n` is the polynomial degree.

### VOLE-Specific Optimization: QuickSilver Polynomial Proofs

Five of six per-access multiplications are part of high fan-in grand products. QuickSilver's polynomial proof mode proves batches of the form:

```
p(x_0) = 0, ..., p(x_{n-1}) = 0
```

where `p` has degree `d`, using only `d` VOLE correlations (instead of `n`).

**Application to fan-in-n multiplication**: Decompose a fan-in-`n` mult into `ceil(n/(epsilon-1))` fan-in-`epsilon` multiplications. Each fan-in-`epsilon` mult is proved as a degree-`epsilon` polynomial check. Parameter `epsilon >= 2` trades VOLE correlations against prover computation (`O(epsilon^2)` per sub-proof).

**Optimization for public set writes**: The set `{1, ..., T}` has publicly known writes `(1, 0), (2, 0), ..., (T, 0)`. After V sends challenge `r`, both parties locally compute the product of these `T` terms, saving `T` ZK multiplications.

**VOLE cost per access** (with polynomial proof optimization):

```
VOLE correlations per access = 5 + 5/(epsilon-1) + o(1)
```

For `epsilon = 16`: approximately `5 + 5/15 = 5.33` VOLE correlations per access.

### Protocol Rounds

5-round protocol in the VOLE-hybrid model:

1. P commits inputs and multiplication outputs
2. V sends challenge for permutation proofs
3. P commits intermediate values of product circuits
4. V sends challenge for batch polynomial proofs
5. P sends final proof

### Field Requirement

Field `F = Z_p` for prime `p` with `|F| >= 2T`. Implementation uses `F_{2^61 - 1}`.

Soundness error: `O((T + m) / |F|)` where `T` is the largest number of accesses and `m` is the number of multiplications.

## Cost Model

### Per-Access Gate Counts

| Data Structure | Input Gates | Fan-in-2 Mults | High Fan-in Mults | Total Non-linear |
|----------------|------------|----------------|-------------------|-----------------|
| ROM (`l`-tuples) | `l + 1` | 0 | 2 (fan-in `n+T`) | `l + 3` |
| Set | 1 | 0 | 2 (fan-in `2T`) | 3 |
| RAM (`l = 1`) | 4 | 1 | 5 (2 set + 2 perm + 1 optimized) | 10 |

### Per-Access VOLE Correlations (RAM, `l = 1`)

| Component | Without poly opt | With poly opt (`epsilon`) |
|-----------|-----------------|--------------------------|
| Input gates | 4 | 4 |
| Fan-in-2 mults | 1 | 1 |
| High fan-in mults (5 grand products) | ~5T total | `5/(epsilon-1)` per access |
| **Total per access** | **~10** | **5 + 5/(epsilon-1)** |

### Communication Per Access (bytes)

| Direction | ROM | Set | RAM |
|-----------|-----|-----|-----|
| P -> V | ~22 | ~12 | ~49 |
| V -> P | ~0.9 | ~0.7 | ~1.3 |
| **Total** | **~23** | **~12** | **~50** |

(At `n = 2^20`, `T = 2^23`, `epsilon = 16`)

### Comparison: Non-linear Gates Per RAM Access

| Construction | Input Gates | Mult Gates | Total Non-linear |
|-------------|------------|------------|-----------------|
| Franzese et al. [CCS'21] | -- | -- | super-constant (log-factor comparisons) |
| Delpech de Saint Guilhem et al. [SCN'22] | -- | -- | 27 |
| **This work** | **4** | **6** | **10** |

### Comparison: Communication Per RAM Access (bytes)

| Construction | `n = 2^12` | `n = 2^16` | `n = 2^20` |
|-------------|-----------|-----------|-----------|
| Franzese et al. | 47 | 49 | 57 |
| Delpech de Saint Guilhem et al. | 130 | 131 | 146 |
| **This work** | **47** | **47** | **50** |

(Franzese et al. has similar total bytes but uses expensive Boolean gates internally.)

## Concrete Performance

### RAM Throughput (`T = 2^23` accesses, `epsilon = 16`)

| Bandwidth | Per-access time (us) | Accesses/sec |
|-----------|---------------------|-------------|
| 25 Mbps | ~15 | ~67K |
| 100 Mbps | ~4.5 | ~220K |
| 500 Mbps | ~1.9 | ~530K |
| 1 Gbps | ~1.7 | **~600K** |

### ROM Throughput (`n = 2^20`, `T = 2^23`, `epsilon = 16`)

| Bandwidth | Per-access time (us) | Accesses/sec |
|-----------|---------------------|-------------|
| 25 Mbps | ~7.5 | ~133K |
| 100 Mbps | ~2.1 | ~480K |
| 500 Mbps | ~1.0 | ~1M |
| 1 Gbps | ~0.85 | **~1.2M** |

### Set Throughput (`n = 2^20`, `T = 2^23`, `epsilon = 16`)

| Bandwidth | Per-access time (us) |
|-----------|---------------------|
| 25 Mbps | ~4.1 |
| 100 Mbps | ~1.2 |
| 500 Mbps | ~0.65 |
| 1 Gbps | ~0.61 |

### Effect of Epsilon on RAM (`n = 2^20`)

| `epsilon` | Comm (bytes) | Time at 25 Mbps (us) | Time at 1 Gbps (us) |
|-----------|-------------|---------------------|---------------------|
| 2 | 88.3 | 28.92 | 2.14 |
| 8 | 55.9 | 18.38 | 1.72 |
| 16 | 50.3 | 16.55 | 1.67 |
| 32 | 47.4 | 15.58 | 1.78 |
| 64 | 46.1 | 15.59 | 2.11 |

Sweet spot: `epsilon = 8` to `32` depending on network. Paper uses `epsilon = 16`.

### Speedup Over Prior Work

| Comparison | Speedup Range | Notes |
|-----------|--------------|-------|
| vs Franzese et al. (RAM) | **2-20x** | Larger at higher bandwidth (computation-dominated) |
| vs Franzese et al. (ROM) | **3-34x** | |
| vs Delpech de Saint Guilhem et al. (RAM) | **2.2-2.9x** | With all VOLE optimizations applied to both |

### Runtime Breakdown (`n = 2^20`, 100 Mbps)

| Phase | ROM (us) | RAM (us) |
|-------|---------|---------|
| Setup | 0.26 | 0.38 |
| Access | 1.38 | 2.73 |
| Teardown | 0.45 | 1.42 |
| **Total** | **2.09** | **4.53** |

Hardware: Intel Xeon Platinum 8175 @ 3.10GHz, single-threaded, Amazon EC2 m5.2xlarge.

## Key Definitions

- **ROM (Read-Only Memory)** -- A key-value store initialized at setup; values never change. Checked via version-chain permutation proof. Cost: `l + 3` non-linear gates per lookup.
- **Set** -- A ROM with `l = 0` (no stored values). Membership proof is a lookup on the key. If the key doesn't exist, the permutation proof fails.
- **RAM** -- Read/write memory. Each access reads old value and writes new value. Reads-writes permutation ensures consistency; set membership on `clock - t` ensures reads come from the past.
- **Version chain** -- Per-address linked list of (value, version) pairs connecting setup writes through lookups to teardown reads. Enforced by the permutation `reads ~ writes` with monotonically increasing versions.
- **Grand product permutation check** -- Prove `x ~ y` by checking `prod_i (r - x[i]) = prod_i (r - y[i])` for random challenge `r`. For `l`-tuples, compress each tuple with a second challenge vector `s`. Cost: `2n - 2` multiplications.
- **`epsilon` (polynomial degree parameter)** -- Fan-in of sub-multiplications when using QuickSilver polynomial proofs to compress grand products. Higher `epsilon` reduces VOLE correlations (`5/(epsilon-1)` per access) but increases prover computation (`O(epsilon^2)`). Best in range 8-32.
- **`clock`** -- Monotonically increasing public counter incremented on each RAM access. The timestamp of each write is `clock` at the time of that access.
- **Multiplexer gate** -- `new = old + op * (w - old)`. When `op = load = 0`, `new = old`; when `op = store = 1`, `new = w`. Costs 1 fan-in-2 multiplication.
- **Teardown** -- Final phase where P reads remaining unread entries (one per address) so that `|reads| = |writes|` and the permutation check can succeed.

## Relevance to VOLE-Based zkVM

1. **Direct RAM primitive for zkVM memory.** The 4+6 gate cost per access is the best known constant-overhead ZK RAM. For a VOLE-based zkVM executing Wasm with memory operations, each load/store instruction maps to this construction at ~50 bytes and ~1.7 us (1 Gbps).

2. **ROM for lookup tables and instruction decoding.** The ROM (cost: `l + 3` gates per lookup, sub-microsecond at 1 Gbps) can serve as the lookup table primitive for range checks, instruction opcode decoding, and ALU truth tables. At 1.2M lookups/sec it is fast enough for inner-loop use.

3. **Set membership for range proofs.** The set data structure provides O(1)-gate membership proofs in publicly known sets. Useful for range checks (`x in {0, ..., 2^k - 1}`), valid-address checks, and timestamp validation within the VM execution trace.

4. **QuickSilver polynomial proof synergy.** The construction is specifically designed so that 5/6 multiplications are high fan-in (grand products), which aligns with QuickSilver's polynomial mode. A zkVM built on QuickSilver can exploit this structure directly, achieving `5 + 5/(epsilon-1)` VOLE correlations per access rather than ~10.

5. **Composability with arithmetic ZK.** The RAM is a pure arithmetic circuit with public-coin challenges, operating in the `F_ZK^perm`-hybrid model. It composes directly with any VOLE-based circuit prover (QuickSilver, Wolverine, Mac'n'Cheese) without protocol changes. The 5-round combined protocol adds only 2 rounds beyond a basic 3-round VOLE-ZK.

6. **Field size constraint.** Requires `|F| >= 2T` where `T` is the number of accesses. For `F_{2^61 - 1}`, this allows up to `~2^60` accesses, which is effectively unlimited. For smaller fields, periodic RAM reset (teardown + fresh setup) can keep `T` bounded while reusing the same set structure.

7. **Amortizable set cost.** The set `{1, ..., T}` can be shared across multiple RAMs or reused for range proofs elsewhere in the zkVM. This saves up to 1 input gate + 1 multiplication per access when amortized. For a zkVM with multiple memory regions (stack, heap, linear memory), a single shared set reduces total overhead.
