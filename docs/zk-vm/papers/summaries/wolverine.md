# Wolverine: Fast, Scalable, and Communication-Efficient Zero-Knowledge Proofs for Boolean and Arithmetic Circuits

**Authors**: Chenkai Weng, Kang Yang, Jonathan Katz, Xiao Wang
**Venue**: IEEE S&P 2021 | **ePrint**: [2020/925](https://eprint.iacr.org/2020/925)

## Role in the VOLE-ZK Design Space

Wolverine is the foundational VOLE-based ZK protocol. It establishes the paradigm of using subfield VOLE (sVOLE) correlations as the offline resource for interactive zero-knowledge proofs over both boolean and arithmetic circuits. It introduces the commit-and-prove approach with information-theoretic MACs (IT-MACs) backed by VOLE, which all subsequent VOLE-ZK papers build on.

## Key Ideas

### IT-MAC Authentication

Every wire value `x ∈ F_p` known to the prover P is authenticated using a global key `Δ ∈ F_{p^r}` held by the verifier V. The authentication is:

- P holds: value `x` and MAC tag `M[x] ∈ F_{p^r}`
- V holds: key `K[x] ∈ F_{p^r}`
- Relation: `M[x] = K[x] + Δ · x`

Denoted `[x]`. These are **additively homomorphic**: given `[x]` and `[x']`, both parties can locally compute `[x + x']`. Multiplication by a public constant is also free: `[b·x] = b·[x]`.

### Two-Phase Protocol

**Offline phase** (input-independent, interactive):
1. Initialize sVOLE: V gets global key `Δ`, parties get authenticated values.
2. Generate authenticated witness masks `{[λ_i]}` for each input wire.
3. Generate authenticated values `{[s_i]}` for each multiplication gate (one per gate).
4. Generate authenticated multiplication triples `{([x_i], [y_i], [z_i])}` for correctness checking.

**Online phase** (can be non-interactive in ROM):
1. **Input processing**: For each input wire `i`, P sends `Λ_i := w_i - λ_i` to V. Both compute `[w_i] := [λ_i] + Λ_i`.
2. **Circuit evaluation**: Process gates in topological order:
   - **Addition gate** `(α, β, γ, Add)`: Both locally compute `[w_γ] := [w_α] + [w_β]`. **Free — no communication.**
   - **Multiplication gate** `(α, β, γ, Mult)`: P sends `d := w_α · w_β - s_i` to V. Both compute `[w_γ] := [s_i] + d`. **Cost: 1 field element from P to V.**
3. **Output check**: P opens `[w_o]` and V checks it equals 1.
4. **Correctness verification**: V uses the authenticated triples to check that multiplication gates were evaluated honestly (see below).

### Correctness Checking (Three Approaches)

**Approach 0 (generic, any field)**: Use `B` triples per multiplication gate. V randomly permutes triples, assigns `B` to each gate, checks each using the Beaver-triple check: open `δ_α = w_α - x` and `δ_β = w_β - y`, then CheckZero on `z - w_γ + δ_β·x + δ_α·y + δ_α·δ_β`. Remaining `c` triples are checked directly. Soundness: `C(B+c choose B)^{-1}`. For ρ=40, need `C' ≥ 2^{ρ/B}` gates per batch.

**Approach 1 (large fields, log p ≥ ρ)**: Only 1 triple per gate. V sends random `η ∈ F_p`, P opens `η·[w_α] - [x]` and `[w_β] - [y]`, then checks. Soundness: `1/p`. **Cost: 4 field elements per gate** (2 opens + 1 triple). All gates checked in parallel with a single `η`.

**Approach 2 (large fields, polynomial check)**: Reduces to **2 field elements per gate**. Define polynomials `F, G, H` interpolating input/output values at the multiplication gates. Check `H = F·G` at a random point `ν ∈ F_{p^r}`. Soundness: `(2C-1)/p^r`. **Trades communication for computation** (requires polynomial interpolation over the shares).

### Batch Opening

Multiple authenticated values can be opened efficiently:
- **Hash-based (ROM)**: P sends all values plus `h = H(M[x_1], ..., M[x_ℓ])`. V checks. Overhead: 2κ bits total, independent of ℓ.
- **Information-theoretic**: V sends random `χ_1, ..., χ_ℓ ∈ F_{p^r}`. P sends `M[x] = Σ χ_i · M[x_i]`. V checks `M[x] = K[x] + Δ · x` where `x = Σ χ_i · x_i`. Soundness: `2/p^r`.

Both can be made non-interactive via Fiat-Shamir.

## Cost Model

| Setting | Communication per mult gate | Prover computation | Prover memory |
|---------|---------------------------|-------------------|---------------|
| Boolean (p=2) | ~9 bits/gate (ρ=40) | O(C) | O(M) where M = memory to evaluate circuit |
| Arithmetic (large field) | 2–4 field elements/gate | O(C) | O(M) |

- Addition gates and XOR gates are **free** (no communication).
- Online phase is **constant round** (or non-interactive in ROM).
- Memory is **streaming**: circuit evaluated on-the-fly, memory proportional to what's needed to evaluate the circuit in the clear.

## Concrete Performance (ρ=40, κ=128)

| Metric | Boolean | Arithmetic (F_{2^61-1}) |
|--------|---------|------------------------|
| Rate (1 thread, 200 Mbps) | 2M AND gates/sec | 600K mult gates/sec |
| Rate (5 threads, 200 Mbps) | ~2.2M AND gates/sec | ~960K mult gates/sec |
| Cost per gate | 0.45 μs (boolean) | 1.6 μs (arithmetic) |
| Memory | ~400 MB (constant) | ~350 MB (constant) |

Scales to circuits with hundreds of billions of gates.

## sVOLE Protocol (Offline Phase)

The paper also presents an efficient sVOLE protocol with three layers:

1. **Base sVOLE** (`Π_{base-sVOLE}`): Built from COPEe (correlated OT). Produces initial short VOLE correlations.
2. **Single-point sVOLE** (`Π_{spsVOLE}`): Generates a vector of authenticated values with exactly one nonzero entry. Uses GGM-tree construction + OT. Includes a consistency check using random linear combination.
3. **sVOLE extension** (`Π_{sVOLE}`): Extends short VOLE to long VOLE using LPN assumption. Public matrix `A` (10-local linear code). Each extend call produces `n - k` new VOLE correlations from `k` seed correlations and `t` single-point sVOLEs.

Performance: 85 ns/VOLE correlation at ≥100 Mbps, 0.42 bits communication per VOLE.

> For the zkVM design, the sVOLE construction details are out of scope — we assume black-box access to VOLE correlations (e.g., via Ferret). The key takeaway is the online protocol structure and cost model.

## Key Definitions

- **`[x]`** — Authenticated value: P holds `(x, M[x])`, V holds `K[x]`, with `M[x] = K[x] + Δ·x`.
- **`Open([x])`** — P sends `x` and `M[x]` to V, who checks `M[x] = K[x] + Δ·x`.
- **`CheckZero([x])`** — Special case of Open where `x` should be 0 (P need not send `x`).
- **Authenticated multiplication triple** — `([x], [y], [z])` with `z = x·y`.
- **sVOLE correlation** — P gets `(x, M[x])`, V gets `K[x]`, for random `x ∈ F_p`, with `M[x] = K[x] + Δ·x`.

## Relevance to VOLE-Based zkVM

1. **Establishes the IT-MAC + VOLE paradigm** that all subsequent papers (QuickSilver, Mac'n'Cheese, LPZK, Mozzarella) build on.
2. **Streaming evaluation model** — circuit processed gate-by-gate with constant memory — is directly applicable to Wasm instruction-by-instruction execution.
3. **Free additions** — only multiplication gates cost communication. For a Wasm VM, this means the circuit representation should minimize multiplications.
4. **The online/offline split** maps to the VC spec's preprocessing model: VOLE correlations are generated before the program is known, then consumed during execution.
5. **The polynomial checking approach** (Approach 2) achieves 2 elements/gate and is the basis for further optimizations in QuickSilver and LPZK.
