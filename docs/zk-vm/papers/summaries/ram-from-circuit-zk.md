# Efficient Proof of RAM Programs from Any Public-Coin Zero-Knowledge System

**Authors**: Cyprien Delpech de Saint Guilhem, Emmanuela Orsini, Titouan Tanguy, Michiel Verbauwhede
**Venue**: SCN 2022 | **ePrint**: [2022/313](https://eprint.iacr.org/2022/313)

## Role in the VOLE-ZK Design Space

This paper presents the first constant-overhead arithmetic circuit `C_check` for verifying RAM consistency, usable as a drop-in module inside *any* public-coin ZK system for circuit satisfiability (MPC-in-the-Head, VOLE-ZK, zkSNARKs, STARKs). The core technique replaces expensive bit-decomposition-based comparisons with a permutation + consecutive-difference trick, achieving **27 non-linear gates per access** (8 multiplications + 6 constant checks, after optimization) over prime fields of any characteristic. This is the direct predecessor to the Two Shuffles construction (Yang & Heath, USENIX Security 2024), which improves the gate count by ~3x to 10 non-linear gates per access. For a VOLE-based zkVM, this paper establishes the foundational approach: encode RAM accesses as tuples, sort by (address, timestamp) via a second permuted list, and verify consistency through three arithmetic sub-circuits (EqCheck, BdCheck, PermCheck) without any Boolean decomposition.

## Key Ideas

### Memory Access Encoding

An initial array `M` of size `N` is encoded as a list `L` of access tuples:

```
L = [(1, 1, write, M_1), ..., (N, N, write, M_N),
     ([l_{N+1}], N+1, [op_{N+1}], [d_{N+1}]), ..., ([l_{N+T}], N+T, [op_{N+T}], [d_{N+T}])]
```

Each tuple `([l], t, [op], [d])` encodes:
- `l` -- memory address (secret)
- `t` -- global timestamp (public, unique per access, starts at N+1)
- `op in {read=0, write=1}` (secret)
- `d` -- value accessed (secret)

Brackets `[x]` denote secret (hidden) wire values. The first `N` entries hard-code write operations with public timestamps `1..N`, so the circuit never needs to check "first access must be write."

### Consistency Check via Sorted Permutation

P provides a second list `L'` that is a permutation of `L`, sorted first by address `l`, then by timestamp `t` within each address group. The circuit `C_check` verifies five criteria:

1. **Permutation**: `L'` is a permutation of `L`
2. **Adjacency**: consecutive entries in `L'` either share the same address or have addresses differing by exactly 1, with correct timestamp ordering within each address group
3. **Bounds**: the last address in `L'` equals `N` (combined with adjacency, all addresses are in `[1, N]`)
4. **Bit check**: each `op` is in `{0, 1}`
5. **Read consistency**: every read returns the value of the most recent write to that address

### Sub-Circuit 1: EqCheck (Equality Test)

Tests `[x] = [y]` without revealing the result, outputting a secret bit `[1-b]`:

```
1. Input [r] = (x - y)^{-1} if x != y, else random non-zero
2. Input [r^{-1}]
3. Check [r] * [r^{-1}] = 1       (verifies r is non-zero)
4. [b] <- ([x] - [y]) * [r]
5. Check (1 - [b]) * [b] = 0      (verifies b is a bit)
6. Return 1 - [b]
```

**Cost**: 2 Input gates, 3 multiplications, 2 constant checks.

**Soundness**: Deterministic -- if `r` or `r^{-1}` are incorrect, one of the checks fails with probability 1.

**Zero-knowledge**: Both `r * r^{-1}` and `(1-b) * b` always evaluate to the same constant (1 or 0) when inputs are correct, so checking against constants reveals nothing.

### Sub-Circuit 2: PermCheck (Permutation Check)

Verifies that two arrays of `S` tuples of field elements are permutations of each other.

**One-dimensional case** (arrays `[A]`, `[B]` in `F^S`):

```
1. Sample random challenge r from V
2. Compute P_A(r) = prod_{i=1}^{S} (r - a_i)
3. Compute P_B(r) = prod_{i=1}^{S} (r - b_i)
4. Check P_A(r) - P_B(r) = 0
```

Soundness error: `S / |F|` by Schwartz-Zippel.

**Multi-dimensional case** (matrices `[A]`, `[B]` in `F^{4 x S}`, for 4-tuples):

1. Sample random `s in F^4` (inner-product compression challenge)
2. Compress each column: `a_i = <s, A[i]>`, `b_i = <s, B[i]>`
3. Apply one-dimensional check on compressed arrays

Soundness error: `(S+1) / |F|` (1/|F| from inner-product failure + S/|F| from polynomial identity test).

**Cost** (`nu = 4`): `nu` Rand commands for `s`, 1 Rand for `r`, `2 * nu * S` multiplications for inner products, `2(S-1)` multiplications for grand products, 1 constant check. Total: `2S(nu + 1) - 2` multiplications.

### Sub-Circuit 3: BdCheck (Bounds Check via Consecutive-Difference Trick)

Verifies that `T` secret values `{[x_i]}` all lie in `[B_1, B_2]`. This is the key innovation that avoids bit decomposition.

```
1. Build L = [B_1, B_1+1, ..., B_2, [x_1], ..., [x_T]]    (size S = B_2 - B_1 + 1 + T)
2. Input [L'] containing entries of L sorted lowest to highest
3. PermCheck([L], [L'])                                      (verifies L' is a permutation of L)
4. For each consecutive pair in L':
     [alpha_i] <- L'[i+1] - L'[i]
     Check [alpha_i] * (1 - [alpha_i]) = 0                  (alpha_i in {0, 1})
5. Check L'[1] = B_1 and L'[S] = B_2
```

**Why this works**: If `L'` is a sorted permutation of `L`, and every consecutive difference is 0 or 1, and the first element is `B_1` and last is `B_2`, then every element in `L` lies in `[B_1, B_2]`. Repeated values are allowed (each `x_i` duplicates some value in the range `[B_1, B_2]`).

**Cost**: `S` Input gates, 1 PermCheck call (one-dimensional, so `2(S-1)` multiplications + 1 constant check), `S-1` bit checks (multiplications), `S+2` constant equality checks.

### Complete C_check Circuit

For each consecutive pair `(i, i+1)` in `L'` (the address-time sorted list):

```
[alpha_i] <- EqCheck([l'_i], [l'_{i+1}])           -- same address?
[lambda_i] <- [l'_{i+1}] - [l'_i]                  -- address difference
[tau_i] <- [alpha_i] * ([t'_{i+1}] - [t'_i]) + (1 - [alpha_i])  -- timestamp gap (or 1 if different address)
Check [alpha_i] + [lambda_i] = 1                    -- address either same or +1
Check (1 - [op'_i]) * [op'_i] = 0                  -- op is a bit
[beta_i] <- EqCheck([d'_i], [d'_{i+1}])            -- same value?
[gamma_i] <- 1 - [alpha_i] * (1 - [beta_i]) * (1 - [op'_{i+1}])  -- read consistency: (diff addr) OR (same val) OR (write)
Check [gamma_i] = 1
```

Then:
```
BdCheck({[tau_i]}, 1, N + T - 1)                   -- all timestamp gaps are positive and bounded
Check [l'_{N+T}] = N                                -- last address is N (bounds all addresses)
```

### Per-Access Gate Accounting (from the loop body)

Per adjacent pair in `L'` (one per access, amortized):
- 2 EqCheck calls: 2 * (3 mult + 2 const-check) = 6 mult + 4 const-check
- 1 multiplication for `[tau_i]`
- 1 multiplication for `[gamma_i]`
- 3 constant checks (adjacency, op-bit, read-consistency)

Subtotal per pair: **8 multiplications + 7 constant checks**

After the Limbo "equality with constant" optimization (checking `[x] * [y] = c` costs 0 extra communication since the product is public), constant checks on multiplication outputs are free. The authors report an effective cost of **8 Mult gates + 6 constant checks per access**.

### Stateless ZK via F_ZK^in

Standard circuit ZK (`F_ZK`) evaluates a deterministic circuit. But `C_check` uses Rand gates (for PermCheck challenges), which must be sampled *after* P commits inputs. The paper defines `F_ZK^in`:

1. **Init**: P commits initial array `M` (stored as list `L`)
2. **Input**: P appends additional values (access tuples, auxiliary inputs for EqCheck/BdCheck/PermCheck)
3. **Prove**: V sends random challenges `{r_i}` for Rand gates; P proves the deterministic circuit `C_check^{r_i}` with no additional input
4. **Verify**: V checks

This separation (commit inputs, then receive challenges, then prove) makes the protocol **public-coin** and compatible with Fiat-Shamir for non-interactivity.

### Extension to Full RAM Programs

A RAM program is a next-instruction circuit `Pi(state, d) -> (op, l, d', state')` executed repeatedly. To prove RAM program execution:

1. Init `F_ZKArray` with memory `M`
2. For each step, evaluate `Pi` as an arithmetic circuit and issue `Access(l, op, d)`
3. Check consistency of all accesses via `C_check`

The combined proof contains: (a) `C_check` for memory consistency, (b) the unrolled arithmetic circuit for `Pi` evaluations, (c) wiring constraints linking `Pi` outputs to access tuples.

### Instantiation with Limbo (MPC-in-the-Head)

The paper instantiates `F_ZK^in` with Limbo, an MPC-in-the-Head protocol:

- Limbo handles addition gates for free, multiplication gates via MultCheck protocol
- Constant checks `[x] * [y] = c` add a multiplication tuple `(x, y, c)` with no extra communication (the output is public)
- Bit checks `(1 - [b]) * [b] = 0` are a special case of the above
- The generalized LimboUC protocol handles Init/Input/Prove/Verify commands by having P commit to input shares via `F_Commit` before V sends challenges

## Cost Model

### Per-Access Non-Linear Gate Count

| Component | Mult Gates | Const Checks | Input Gates |
|-----------|-----------|-------------|------------|
| EqCheck (address) | 3 | 2 | 2 |
| EqCheck (value) | 3 | 2 | 2 |
| tau_i computation | 1 | 0 | 0 |
| gamma_i computation | 1 | 0 | 0 |
| Adjacency check | 0 | 1 | 0 |
| Op bit check | 0 | 1 | 0 |
| Read consistency check | 0 | 1 | 0 |
| **Subtotal (per pair)** | **8** | **7** | **4** |

After Limbo optimization: **8 mult gates + 6 constant checks per access** (one const check is absorbed).

### Amortized Costs Including PermCheck and BdCheck

| Sub-circuit | Multiplications | Constant Checks | Inputs |
|-------------|----------------|----------------|--------|
| PermCheck(4, L, L') | `2 * 4 * (N+T) + 2(N+T-1)` = `10(N+T) - 2` | 1 | `N + T` (for L') |
| BdCheck({tau_i}, 1, N+T-1) | `2(S-1) + (S-1)` where `S = 2(N+T) - 1` | `S + 3` | `S` |
| Loop body (N+T-1 pairs) | `8(N+T-1)` | `7(N+T-1)` | `4(N+T-1)` |
| Boundary checks | 0 | 1 | 0 |
| **Total** | `O(N+T)` | `O(N+T)` | `O(N+T)` |

All costs are **linear** in `N + T` with constant overhead.

### Soundness Error

```
Pr[C_check outputs 1 on inconsistent L] <= 2(N + T - 1) / |F|
```

For `F = GF(2^61 - 1)` and `N + T = 2^20`: soundness error ~ `2^{-40}`.

### Comparison of Non-Linear Gates Per Access

| Construction | Non-Linear Gates/Access | Overhead Type | Setting |
|-------------|------------------------|---------------|---------|
| TinyRAM (sorting network) | `O(T log T)` total | super-linear | public-coin |
| BubbleRAM | `O(log^2 N)` | super-constant | private-coin |
| PrORAM | `O(log N)` | super-constant | private-coin |
| Franzese et al. [CCS'21] | super-constant (Boolean comparisons) | super-constant | private-coin |
| **This work** | **~27** (8 mult + 6 const + inputs) | **constant** | **public-coin** |
| Two Shuffles [USENIX'24] | **10** (4 input + 6 mult) | **constant** | public-coin |

## Concrete Performance

### Per-Access Amortized Cost (RAM size 2^18, `F = GF(2^61 - 1)`, 40-bit security)

| Configuration (parties, reps, threads) | Access Time (ms) | Access Size (KB) |
|----------------------------------------|-----------------|-----------------|
| (64, 7, 1) | 1.11 | 0.920 |
| (4, 21, 1) | 0.42 | 2.82 |
| (8, 14, 1) | 0.44 | 1.82 |
| (8, 14, 14) | 0.12 | 1.82 |

### Comparison with Prior Work (RAM size 2^18)

| Scheme | Field | Asymptotic | Access Time (ms) | Access Size (KB) | Setting |
|--------|-------|-----------|-----------------|-----------------|---------|
| BubbleRAM | GF(2^40 - 87) | O(log^2 N) | 0.15 | 1.5 | private-coin |
| PrORAM | GF(2^40 - 87) | O(log N) | 0.01 | 0.4 | private-coin |
| Franzese et al. | Z_{2^32} | O(1) | 0.01 | 0.031 | private-coin |
| **Ours (4, 21, 1)** | GF(2^61 - 1) | O(1) | 0.42 | 2.82 | **public-coin** |
| **Ours (8, 14, 14)** | GF(2^61 - 1) | O(1) | 0.12 | 1.82 | **public-coin** |

### Initialization Cost (Scales Linearly)

| RAM Size | Prover Time (s), (64,7,1) | Prover Time (s), (8,14,14) | Proof Size (MB), (64,7,1) |
|----------|--------------------------|---------------------------|--------------------------|
| 2^4 | ~0.01 | ~0.01 | ~0.01 |
| 2^10 | ~0.1 | ~0.05 | ~0.1 |
| 2^16 | ~5 | ~2 | ~10 |
| 2^19 | ~50 | ~15 | ~80 |

Hardware: Intel i9-9900 (3.1GHz), 128GB RAM. Times averaged over 20 runs.

## Key Definitions

- **Access tuple** -- `([l], t, [op], [d])`: address (secret), timestamp (public, unique), operation (secret bit: 0=read, 1=write), data value (secret). Timestamps `1..N` are assigned to initial array writes; `N+1..N+T` to program accesses.

- **`L` (original list)** -- The list of `N + T` access tuples in chronological order. First `N` entries encode the initial array `M` with hard-coded write operations.

- **`L'` (sorted list)** -- A permutation of `L`, sorted first by address `l`, then by timestamp `t` within each address group. Provided by P as auxiliary input. Enables local (adjacent-pair) checks instead of global searches.

- **`C_check`** -- The complete arithmetic checking circuit. Takes `L` as initial input, `L'` and auxiliary values as prover inputs. Uses Rand gates for permutation challenges. Outputs 1 iff all accesses are consistent.

- **EqCheck** -- Sub-circuit producing a secret equality bit `[x = y]` without bit decomposition. Uses the inverse trick: P inputs `r = (x-y)^{-1}` (or random non-zero if `x = y`), circuit verifies `r` is non-zero and result is a bit. Cost: 3 multiplications.

- **PermCheck** -- Sub-circuit verifying `L' ~ L` (permutation) via polynomial identity testing. For 4-tuples, uses inner-product compression with random `s in F^4` to reduce to 1-D, then Schwartz-Zippel on grand products `prod(r - a_i) = prod(r - b_i)`.

- **BdCheck (consecutive-difference trick)** -- Verifies `B_1 <= [x_i] <= B_2` for `T` values by: (1) merge `{x_i}` with `{B_1, ..., B_2}` into array `L`, (2) P provides sorted permutation `L'`, (3) PermCheck verifies it is a permutation, (4) check every consecutive difference in `L'` is 0 or 1, (5) check endpoints are `B_1` and `B_2`. Avoids all bit decomposition.

- **Constant-overhead** -- The total cost of verifying `T` accesses to a size-`N` array is `O(N + T)` field operations, with the constant independent of `N` and `T`. No `log N` or `log T` factors.

- **`F_ZK^in` (ZK with separate input)** -- Extended ZK functionality that accepts inputs (Init + Input) before the circuit is proven (Prove). Enables P to commit auxiliary values before V sends Rand challenges, which is required for soundness of the permutation checks.

- **Stateless ZK** -- The protocol does not require the ZK system to maintain state across multiple proofs. All access tuples and auxiliary inputs are bundled into a single circuit proof. Contrast with Franzese et al. which requires a stateful `F_ZK` that can reactively re-use committed inputs.

- **Inner-product compression** -- Reduces multi-dimensional permutation check (4-tuples) to one-dimensional: sample random `s in F^4`, compress each column `a_i = <s, A[i]>`. If columns differ, compressed values differ except with probability `1/|F|`.

## Relevance to VOLE-Based zkVM

1. **Foundational RAM technique for VOLE-ZK.** The `C_check` circuit is a pure arithmetic circuit with public-coin Rand gates, directly instantiable in VOLE-ZK systems (QuickSilver, Wolverine, Mac'n'Cheese). A VOLE-based zkVM can use this as its memory consistency module, with each load/store translating to one access tuple.

2. **The consecutive-difference BdCheck is the key innovation.** Replacing comparison circuits (which require bit decomposition costing `O(log |F|)` gates per comparison) with the permutation + sorted-difference trick achieves constant gates per bound check in prime fields. This technique is reusable beyond RAM -- any range check in a VOLE-based zkVM (e.g., verifying ALU operands are in range, checking memory addresses are valid) can use BdCheck.

3. **Baseline for Two Shuffles improvement.** Two Shuffles (Yang & Heath, 2024) reduces the per-access cost from ~27 non-linear gates to 10 by restructuring the two permutation checks (one for address-sorted consistency, one for timestamp bounds) and eliminating EqCheck calls. Understanding this paper's 27-gate baseline is necessary to evaluate the 3x improvement and to identify which components (EqCheck overhead, BdCheck overhead) were eliminated.

4. **Public-coin property enables non-interactivity.** Because `C_check` uses only public Rand gates, the entire RAM proof can be made non-interactive via Fiat-Shamir. For a VOLE-based zkVM that needs to produce transferable proofs (e.g., for blockchain verification), this is essential. Private-coin protocols (Franzese et al., BubbleRAM) cannot be straightforwardly compiled to non-interactive proofs.

5. **Generic compiler structure informs modular zkVM design.** The architecture -- (a) encode execution trace as access tuples, (b) separate `C_check` from the CPU circuit `Pi`, (c) compose via `F_ZK^in` -- is a template for zkVM design. The CPU logic (instruction decode, ALU, control flow) and memory consistency (C_check) are independent modules connected only by shared wire values for `(l, op, d)` tuples.

6. **Inner-product compression for tuple permutations.** The technique of compressing 4-tuple permutation checks to 1-D via random inner products (`s in F^4`) at cost `1/|F|` soundness per compression is directly applicable to any VOLE-ZK setting that needs to permute structured records (e.g., execution trace rows containing `(pc, opcode, operands, result)`).

7. **Cost model for capacity planning.** At ~27 non-linear gates per access in the generic circuit model (or 8 mult gates + 6 constant checks after Limbo optimization), a VOLE-based zkVM designer can estimate: for a program with `T` memory accesses and `M` arithmetic operations in the CPU circuit, total VOLE correlations ~ `27T + M`. This provides a concrete budget for deciding when RAM is cheaper than circuit-level memory (the crossover point vs. a Merkle-tree approach at `O(log N)` hash gates per access).
