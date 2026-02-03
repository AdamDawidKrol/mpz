# Mac'n'Cheese: Zero-Knowledge Proofs for Boolean and Arithmetic Circuits with Nested Disjunctions

**Authors**: Carsten Baum, Alex J. Malozemoff, Marc B. Rosen, Peter Scholl
**Venue**: CRYPTO 2021 | **ePrint**: [2020/1410](https://eprint.iacr.org/2020/1410)

## Role in the VOLE-ZK Design Space

Mac'n'Cheese extends the Wolverine commit-and-prove paradigm with communication-efficient disjunctions: proving 1-of-m branches with communication proportional to the *longest* branch (not the sum). This is the key paper for handling branching control flow (if/else, switch/case, br_table) over symbolic values in a VOLE-based zkVM, where naive evaluation of all branches would multiply communication by the branch factor.

## Key Ideas

### Commit-and-Prove via IPs with LOVe

The paper formalizes a new abstraction called **Interactive Proofs with Linear Oracle Verification (IPs with LOVe)**. This generalizes linear IOPs by additionally allowing prover-verifier message exchange before the oracle query phase.

The structure of an IP with LOVe:
1. P fixes a proof string `pi` (vector of field elements) and submits it to an oracle O.
2. P and V exchange messages for `t` rounds.
3. V makes `q` affine queries `(z_j, y_j)` to the oracle, checking `<pi, z_j> = y_j`.
4. V accepts iff all queries pass.

**Key property**: the protocol is *public-coin* if V's messages are uniformly random and the queries are deterministic from the transcript.

### MAC-Based Commitments from VOLE

Each VOLE output is an IT-MAC commitment. The MAC relation is:

```
MAC_{(alpha, beta)}(x) = x * alpha + beta
```

- P holds: value `x` and tag `tau = x*alpha + beta`
- V holds: MAC key `alpha` (global), offset `beta_x` (per-value)
- Opening: P sends `(x, tau)`, V checks `tau = x*alpha + beta_x`
- Binding: cheating requires guessing `alpha`, probability `1/p^k`

These are linearly homomorphic (same `alpha`), so affine operations on commitments are free. The VOLE-to-ZK transformation (Figure 2) is:

1. **Input phase**: Call `F_VOLE` to get random commitments `[r[1]], ..., [r[l]]`. P sends `d_i = r[i] - pi[i]` to adjust each to the desired proof string. Commitments `[pi[i]] = [r[i]] - d_i`.
2. **Protocol phase**: Exchange messages per the IP with LOVe.
3. **Query phase**: For each query `(z_j, y_j)`, compute `[mu_j] = sum(z_j[i] * [pi[i]]) - y_j`. P sends the MAC tag; V checks `tau_{mu_j} = beta_{mu_j}` (i.e., `mu_j = 0`).

**Optimizations**: (a) Random proof elements need no `d_i` transmission (P sets `pi[i] = r[i]`). (b) Subfield VOLE: when `pi` consists of `F_p` elements, use sVOLE so P sends only `F_p` elements while MACs remain over `F_{p^k}`.

### Circuit Satisfiability Protocols

**Simple protocol** (Section 4.2) -- 3 field elements per multiplication gate:

For each `Mult([x], [y])`:
1. `Random(F_{p^k}) -> [a]` (random mask)
2. `Fix(x*y) -> [z]` (commit to product, costs 1 `F_p` element)
3. `Fix(a*y) -> [c]` (commit to masked product, costs 1 `F_{p^k}` element)

Verification (all mult gates share one random challenge `e` from V):
1. `Reveal([epsilon])` where `[epsilon] = e*[x] - [a]` (costs 1 `F_{p^k}` element)
2. `AssertZero(e*[z] - [c] - epsilon*[y])`

Soundness: a cheating prover with error `Delta` in `[z]` needs `e*Delta = Delta'` (error in `[c]`), which holds with probability `1/p^k`.

**Batched protocol** (Section 4.3) -- `1 + epsilon` field elements per gate for large batches:

Uses `AssertMultVec` to verify `n` multiplications simultaneously:
1. V sends random `r`. P shows `<r^i * [x_i], [y_i]> = sum(r^i * [z_i])`.
2. This reduces to `AssertDotProduct`, which recursively halves the problem by encoding values as degree-1 polynomials, evaluating at a random point, and recurring.
3. Base case (n <= 2): use the simple multiplication check.

Communication: `n + O(log n)` field elements for `n` multiplications. For `F_{2^{61}-1}` with batch size 1M: ~64.3 bits per gate. Rounds: `O(log b)` where `b` is batch size.

**Binary circuit optimization** (Section 4.4) -- 9 bits per AND gate:

Uses Reverse Multiplication-Friendly Embeddings (RFMEs). A `(15, 45)_2`-RFME maps 15-bit pairwise multiplication to a single `F_{2^{45}}` multiplication. P provides an advice vector `d` and the parties verify a single extension-field multiplication. Per-AND cost: 9 bits, independent of batch size.

### Disjunctive Proofs (Stacking)

The core contribution. Goal: given `m` public-coin IPs with LOVe `Pi_1, ..., Pi_m`, construct `Pi_OR` proving `x_1 in L(R_1) OR ... OR x_m in L(R_m)` with communication proportional to `max{alpha_i}` instead of `sum(alpha_i)`.

**Equisimulatability**: The prerequisite for stacking. Protocols `Pi_1, ..., Pi_m` are equisimulatable if there exist algorithms `CP` (combined prover) and `dec` (decode) such that:
- `CP` encodes the true branch's message `a_h` into a combined message `c` whose distribution is identical regardless of which branch is true.
- `dec` recovers the branch-specific message from `c`.

This holds automatically when all prover messages are uniformly random (Lemma 1), which is the case for all protocols in the paper.

**Protocol Pi_OR** (Figure 3) -- two phases:

*Phase I (Stacked execution)*:
1. P builds proof string: `pi = pi_{i*} || 0...0 || r_1 ... r_m` (true branch's proof, zero-padded, plus `m` random `F_{p^k}` elements).
2. For `t` rounds: V sends random challenges (length = max over all branches). P runs `Pi_{i*}` on its branch, outputs combined message via `CP`. V uses `dec` for each branch in parallel -- cannot tell which is real.

*Phase II (CDS94-style OR check)*:
3. For each false branch `i != i*`: P picks random `f_i`, computes the query `(z_i, y_i)` that would be made, and sets `d_i = (<pi', z_i> - y_i) / f_i + r_i`. For the true branch: `d_{i*} = r_{i*}`.
4. V sends random challenge `f`. P sets `f_{i*} = f - sum(f_i for i != i*)` and sends `f_1, ..., f_{m-1}`.
5. V makes `m` oracle queries: for each `i`, query `(z_i || beta_i, y_i + f_i * d_i)` where `beta_i` has `f_i` in position `i`.

The trick: for the true branch, the query succeeds because `d_{i*} = r_{i*}` and the underlying proof is valid. For false branches, P chose `f_i` in advance and set `d_i` to make the query pass. But since V's `f` is random and determines `f_{i*}`, a cheating P (with no valid branch) can only satisfy one value of `f`.

**Complexity of Pi_OR**:
- Rounds: `3 + max_i{t_i}`
- Oracle length: `m + max_i{l_i}`
- Message complexity: `2mk + alpha` elements of `F_p` (where `alpha = max` branch message complexity)
- Soundness: `sum(epsilon_i) + 1/p^k`

### Recursive Stacking (Logarithmic Overhead)

`Pi_OR` still has additive `O(m)` overhead from the `m` random values `r_i` and the OR-check messages. The recursive variant `Pi_{R-OR}` reduces this to `O(log m)`:

1. **Base construction** (`Pi_{A-OR}`, Figure 7): For 2 branches, instead of the CDS94 check, P proves that `mu_1 * mu_2 = 0` (where `mu_i` is the query residual for branch `i`). P commits to auxiliary values `u, v` and sends `delta = u*mu_2 - v` and `epsilon = f*mu_1 - u`. V combines two checks into a single oracle query using a second challenge `f'`.

2. **Recursion** (Lemma 2): `Pi_{A-OR}` is itself equisimulatable (its extra messages `delta, epsilon` are uniformly random). So `m` protocols can be combined in a binary tree: pair up into `m/2` instances of `Pi_{A-OR}`, then pair those into `m/4`, etc.

**Complexity of Pi_{R-OR}**:
- Rounds: `3 * log_2(m) + max_i{t_i}`
- Oracle length: `2 * log_2(m) + max_i{l_i}`
- Message complexity: `4k * log_2(m) + max{alpha_i}` elements of `F_p`
- Soundness: `sum(epsilon_i) + (m-1) * 2/p^k`

### Streaming and Non-Interactive Proofs

**Streaming**: The simple protocol can be made streamable by checking each multiplication gate on-the-fly with an independent challenge `e_i` from V (rather than batching). Memory: `O(1)` for linear circuits, `O(m)` for disjunctions. The `AssertZero` calls are batched incrementally: maintain running `[z] = [z] + e_i * [gamma_i]` and check once at the end.

**Fiat-Shamir (NIZK)**: After a VOLE preprocessing phase, the protocol can be made non-interactive by replacing V's challenges with random oracle outputs. The paper defines *round-by-round soundness* for IPs with LOVe and shows this is sufficient for FS soundness even with many rounds (potentially linear in circuit size).

NIZK soundness: `p^{-k} + t*epsilon_{rbs} + Q*(epsilon_{rbs} + 2/|C| + 2^{-lambda})` where `Q` = number of RO queries, `|C|` = smallest challenge set, `epsilon_{rbs}` = round-by-round soundness error.

**Streaming for disjunctions**: Both branches of a 2-way disjunction must be interleaved (not sequential) to avoid revealing which is true. The implementation uses stackful coroutines.

## Cost Model

| Protocol variant | Communication per mult gate | Rounds | Disjunctions |
|---|---|---|---|
| Simple (arithmetic) | 3 `F_p` elements | 3 | yes |
| Simple (boolean) | 9 bits | 3 | yes |
| Batched (arithmetic) | `1 + epsilon` `F_p` elements | `O(log b)` | yes |
| Batched (boolean) | `1 + epsilon` bits (e.g., 1.008 for b=1M) | `O(log b)` | yes |

Disjunction overhead for `Pi_OR`: `+2mk` field elements (`F_{p^k}`) additive.
Disjunction overhead for `Pi_{R-OR}`: `+4k * log_2(m)` field elements additive.

For both: computation scales with `sum` of all branch sizes (prover evaluates all branches), but communication scales with `max` branch size.

- Addition/XOR gates: **free** (no communication, same as Wolverine).
- Comparison to Wolverine: reduces arithmetic cost from 4 to 3 field elements (simple), or ~1 (batched). Boolean cost is 9 bits (simple) vs. Wolverine's 7 bits.
- Comparison to stacked garbling: 128 bits/AND vs. Mac'n'Cheese's 9 bits/AND = ~14x improvement, plus support for arithmetic circuits.

## Concrete Performance

Benchmarks at 95 ms latency, 31.5 Mbps bandwidth (east-to-west coast US):

| Metric | Boolean (`F_{2^{40}}`) | Arithmetic (`F_{2^{61}-1}`) |
|---|---|---|
| Per-gate time (network) | 144 ns | 1.5 us |
| Per-gate time (local) | 141 ns | 276 ns |
| Throughput (network) | 6.9 Mmps | 0.6 Mmps |
| Throughput (local) | 7.0 Mmps | 3.6 Mmps |

Disjunction benchmarks (boolean, 1B AND gates per branch, network):

| Branches | Verify time (s) | Comm. increase over 1 branch |
|---|---|---|
| 1 | 139 | -- (baseline: 124 MB) |
| 2 | 307 | +25 bytes |
| 4 | 568 | +50 bytes |
| 8 | 1254 | +75 bytes |

Communication overhead: `+25 * log_2(m)` bytes. Prover overhead: linear in total circuit size (~4x local evaluation cost). VOLE setup (not included): +85 ns/correlation, +0.42 bits/correlation.

## Key Definitions

- **`[x]`** -- Committed value: P holds `(x, tau_x)`, V holds `(alpha, beta_x)`, with `tau_x = x*alpha + beta_x`. Same as Wolverine's IT-MAC but notation uses additive offset `beta` instead of Wolverine's `K[x]`.
- **IP with LOVe** -- Interactive Proof with Linear Oracle Verification: P commits proof string `pi`; parties exchange `t` rounds of messages; V makes `q` affine queries to `pi` via oracle.
- **Public Coin IP with LOVe** -- V's messages are uniform random; oracle queries are deterministic from transcript.
- **Equisimulatable** -- A set of IPs with LOVe where `CP` can encode any branch's message into a combined message with identical distribution, and `dec` can recover branch-specific messages. Follows automatically when P's messages are uniform random.
- **`AssertZero([x])`** -- Oracle query checking committed value is 0. When instantiated with VOLE: P sends tag `tau_x`, V checks `tau_x = beta_x` (i.e., `x = 0`).
- **`Fix(x) -> [x]`** -- Commit to a chosen value: draw `[r]`, send `x - r`, set `[x] = [r] + (x - r)`. Cost: 1 field element.
- **`Reveal([x]) -> x`** -- Open a commitment: send `x`, then `AssertZero([x] - x)`.
- **C&P IP with LOVe** -- A high-level syntax for IPs with LOVe using abstract homomorphic commitments, instructions `Random`, `Send`, `Fix`, `Reveal`, `AssertZero`.
- **Round-by-round soundness** -- A strengthening of soundness: there exists a `State` function such that if the protocol is in a "reject" state, transitioning to "accept" in any single round happens with probability at most `epsilon` over V's random challenge.
- **RFME** -- Reverse Multiplication-Friendly Embedding: a pair `(phi, psi)` of linear maps such that `psi(phi(x) * phi(y)) = x * y` component-wise. Used for efficient binary multiplication via extension field.

## Relevance to VOLE-Based zkVM

1. **Branching over symbolic values**: Wasm's `if/else`, `br_table`, and `select` operating on symbolic (committed) values cannot be resolved by the prover alone -- both branches must be accounted for. Mac'n'Cheese's disjunction stacking reduces the communication cost from `O(sum of branches)` to `O(max branch) + O(log m)`, making complex control flow tractable.

2. **Nested disjunctions for structured control flow**: Wasm has structured, potentially nested `if/else` blocks. The recursive stacking construction (`Pi_{R-OR}`) handles arbitrary nesting with only logarithmic overhead per level, exactly matching the tree structure of nested conditionals.

3. **Streaming compatibility**: The prover and verifier can process the circuit gate-by-gate with `O(1)` memory (or `O(m)` per disjunction level), matching the instruction-by-instruction execution model of a Wasm interpreter. Disjunction branches are interleaved via coroutines rather than buffered.

4. **Fiat-Shamir for non-interactivity**: After VOLE preprocessing, the multi-round protocol (even with per-gate challenges) can be made non-interactive. This is critical for a practical zkVM where a prover generates a proof offline and a verifier checks it later.

5. **Concrete branch cost**: For 8 branches with 1B gates each, communication increases by only 75 bytes over a single branch. The dominant cost is computation (prover evaluates all branches), not communication. For a Wasm VM with a `br_table` of 8 targets, this means branching is essentially free in bandwidth.

6. **Compatibility with the VOLE-ZK stack**: Mac'n'Cheese uses the same IT-MAC/VOLE commitment scheme as Wolverine and QuickSilver. The disjunction optimization is *orthogonal* to the base multiplication protocol -- any commit-and-prove IP with LOVe (including future improvements) can be stacked. However, stacking is *not* directly compatible with QuickSilver or Line-Point ZK, because their verification checks require verifier input that would leak the true branch.
