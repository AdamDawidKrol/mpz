# Protocol Selection Questions

These questions must be answered before writing the protocol selection document.
Answers will be recorded inline after each question.

---

## 1. Security Model & Assumptions

**Q1.1: What security model does the zkVM target?**
- A) Semi-honest (both parties follow the protocol)
- **B) Malicious (either party may deviate arbitrarily)** ✓
- C) Covert (cheating is detected with some probability)

> **Answer:** Malicious security. The zkVM uses malicious-secure protocols (QuickSilver, Batchman, etc.) where either party may deviate arbitrarily from the protocol.

**Q1.2: Is a random oracle assumption acceptable?**
- **A) Yes — we expect to use Fiat-Shamir and hash-based constructions throughout** ✓
- B) Only where necessary — prefer information-theoretic where possible
- C) No — information-theoretic security only

> **Answer:** Yes. The random oracle assumption is acceptable throughout the design, including Fiat-Shamir transforms and hash-based batch checking (e.g., BLAKE3 in JesseQ).

**Q1.3: What computational security parameter (kappa)?**
- **A) 128** ✓
- B) 256
- C) Configurable

> **Answer:** κ = 128.

**Q1.4: What statistical security parameter (sigma)?**
- **A) 40** ✓
- B) 80
- C) Configurable

> **Answer:** σ = 40.

---

## 2. Algebraic Domain

**Q2.1: What algebraic domain should the zkVM operate in?**
- A) Prime field (F_{2^61-1}) — battle-tested, all protocols benchmarked here, but Wasm wrapping semantics need reduction circuits and range proofs
- B) Ring Z_{2^k} (Mozzarella/QuarkSilver) — native Wasm match (no reduction circuits), but ~3.7x slower raw throughput and key protocols (Batchman, Two Shuffles RAM, LogRobin++) have not been instantiated over rings
- **C) Mixed, with a preference for binary fields** ✓
- D) Not sure — recommend based on overall system analysis

> **Answer:** Mixed-domain. The primary domain is binary (F\_2 / F\_{2^k}), leveraging subfield VOLE (sVOLE) as the base correlation. VOLE over a large prime field can be generated via COPEe when needed (e.g., for protocols that require prime-field arithmetic like permutation checks). The preference for binary fields aligns with Wasm's bit-level operations and avoids the overhead of emulating wrapping arithmetic in a prime field.

**Q2.2: How should we weigh throughput vs implementation simplicity?**
- A) Prioritize throughput — we want the fastest system even if more complex
- B) Prioritize simplicity — fewer moving parts, easier to implement and reason about
- **C) Throughput-biased, but simplicity wins when complexity is high** ✓

> **Answer:** Bias toward throughput, but not at any cost. If a protocol or optimization introduces substantial implementation complexity, prefer the simpler alternative. The implementation should lean toward fewer lines of code. In practice: adopt optimizations that are straightforward to implement (e.g., JesseQ as a drop-in for QuickSilver), but avoid complex multi-mode systems unless the performance gain is compelling.

**Q2.3: Does per-byte taint tracking affect domain choice?**
- A) Yes, it pushes toward a specific choice (explain which)
- **B) No — moot. Taint is tracked externally by the embedder, not in the proof circuit or RAM.** ✓
- C) Not sure — revisit after RAM design

> **Answer:** Moot. Taint is a publicly computable data structure maintained by the embedder outside the proof circuit (Q6.1). It does not interact with the algebraic domain or the RAM protocol.

---

## 3. Gate Checking (Base Proof Layer)

**Q3.1: Which base proof protocol should the zkVM use?**
- **A) QuickSilver — 1 element/gate, 4.8M arith gates/sec, information-theoretic, polynomial mode for structured sub-computations** ✓
- B) JesseQ JQv1 — 1 element/gate, 23.3M arith gates/sec (3x faster prover), requires qsVOLE preprocessing and BLAKE3 (random oracle)
- C) JesseQ JQv2 — 1/2 element/gate for layered circuits, same assumptions as JQv1
- D) Not sure — recommend one

> **Answer:** QuickSilver. It is more battle-tested and better understood. JesseQ JQv1 is a known upgrade path (drop-in replacement, 3x faster prover) that can be adopted later during optimization without architectural changes.

**Q3.2: If JesseQ is selected, should JQv2's half-communication mode be included?**
- A) Yes — worth the complexity for 2x communication reduction on layered circuits
- B) No — single mode (JQv1) keeps things simple
- **C) Defer — note it as a future optimization** ✓

> **Answer:** Deferred. QuickSilver is the initial choice (Q3.1). JQv2's half-communication mode is a future optimization contingent on first upgrading to JQv1.

**Q3.3: Should the polynomial mode (QuickSilver's polynomial ZK) be included for structured sub-computations?**
- **A) Yes — needed for permutation arguments and structured sub-computations** ✓
- B) No — circuit mode only
- C) Defer — note as future optimization

> **Answer:** Yes. QuickSilver's polynomial mode is required for the RAM protocol's permutation checks (grand-product arguments in Two Shuffles RAM), where it compresses high fan-in multiplications from ~10 VOLE/access to ~5.33. It is also useful for structured host functions and lookup-like gadgets.

---

## 4. Instruction Dispatch (Branching / Disjunction)

**Q4.1: At what granularity should the zkVM dispatch instructions?**
- A) Individual Wasm opcodes — each opcode is one branch circuit
- B) Basic blocks / CFG blocks — group straight-line code into one "instruction"
- **C) Hybrid — adaptive granularity depending on execution context** ✓
- D) Not sure — recommend based on cost analysis

> **Answer:** Hybrid, with adaptive granularity. The dispatch strategy depends on the execution context:
> - **Straight-line segments** can be compiled as optimized circuits for an entire basic block, or dispatched as individual opcodes — the choice depends on the block's structure and cost.
> - **Control flow points** (branches, calls, indirect jumps) require CPU-style dispatch with disjunction protocols.
> - **Branch conditions** may themselves use disjunctions (e.g., proving one of two branch targets was taken).
>
> The granularity is not a single fixed choice but a compilation strategy that mixes block-level optimization with per-opcode dispatch where needed. The exact boundaries depend on other design decisions (RAM model, taint tracking, etc.).

**Q4.2: Is Tight ZK CPU's pay-per-instruction-size property essential?**
- A) Yes — Wasm opcodes vary too much in size to pad uniformly
- **B) No — padding is acceptable; Batchman's uniform-size model is sufficient** ✓
- C) Depends on the dispatch granularity and compilation strategy

> **Answer:** No. Tight ZK CPU's pay-per-use property is not essential for the initial design. Batchman's uniform-size branch model is sufficient. Wasm's structured control flow allows the compiler to choose dispatch granularity per-segment, mitigating the padding overhead. Tight ZK CPU is a potential future optimization if heterogeneous instruction sizes become a bottleneck, but its implementation complexity (UROM, boundary strings, Expand1/2) is too high for the initial system.

**Q4.3: Which dispatch protocol(s) should the zkVM use?**
- A) Batchman only — O(RB + R|C| + B|C|), simpler, well-benchmarked
- B) Tight ZK CPU only — pay for taken instruction size, 5-18x over Batchman for heterogeneous sizes
- **C) Batchman + LogRobin++ — Batchman for batched steps, LogRobin++ for isolated disjunctions** ✓
- D) Tight ZK CPU + LogRobin++ — tight dispatch for the main loop, LogRobin++ for edge cases
- E) Not sure — recommend one

> **Answer:** Batchman + LogRobin++. Both share the same topology-matrix foundation (MULLEFT, compressed topology vectors, IT-MAC commitments), so one set of primitives supports both protocols. Batchman handles the repeated CPU-step dispatch loop where amortization over R repetitions yields O(RB + R|C| + B|C|) total computation. LogRobin++ handles isolated branch points and non-batched disjunctions at O(B + |C|) per instance. Tight ZK CPU is excluded from the initial design due to high implementation complexity (UROM, boundary strings, Expand1/2), but noted as a future optimization path.

**Q4.4: Should the execution path (which opcodes were executed) be hidden from the verifier?**
- A) Yes — the verifier learns only the total computation size
- B) No — the execution path can be public (simpler, cheaper)
- **C) Follows from branch condition taint** ✓

> **Answer:** The execution path visibility is determined by the taint of branch conditions. When a branch condition is concrete (public), the taken path is visible to the verifier — no hiding is needed and no padding cost is incurred. When a branch condition is symbolic (private/blind), the taken path must be hidden, requiring padding to conceal both which branch was taken and the path length. This is not a global configuration but a per-branch-point property derived from the VC spec's taint system.

---

## 5. Memory Model

**Q5.1: At what granularity should the RAM for linear memory operate?**
- A) Byte-level — simplest for per-byte taint, no alignment issues
- **B) i32-word-level (4 bytes per word)** ✓
- C) Not sure — recommend based on cost analysis

> **Answer:** i32-word-level. Cost analysis (Phase 2, OQ1) showed that Mystique domain conversions (~35μs each) make byte-level RAM prohibitively expensive (4x overhead for i32.load). i32-word optimizes for the dominant wasm32 case: i32.load/store is 1 RAM access, i64.load/store is 2. Per-byte taint is tracked externally by the embedder as 4 taint bits per word. Unaligned accesses and sub-word stores (i8/i16) require read-modify-write, handled in the compiler.

**Q5.2: Which RAM protocol should the zkVM use for linear memory?**
- **A) Two Shuffles RAM — 4 input + 6 mult gates per access, ~600K accesses/sec, state of the art, composes externally with Batchman** ✓
- B) A simpler permutation-based RAM (e.g., Franzese et al.) — higher gate count but potentially simpler implementation
- C) Not sure — recommend one

> **Answer:** Two Shuffles RAM. Best known constant-overhead ZK RAM: 4 input + 6 mult gates per access, ~600K accesses/sec at 1 Gbps, ~50 bytes/access. Composes externally with Batchman — instruction circuits produce read/write records, and the permutation check runs as a separate proof phase. The grand-product permutation checks leverage QuickSilver's polynomial mode (Q3.3) for compression.

**Q5.3: Should the zkVM use specialized stack/queue primitives?**
- A) Yes — use cheaper stack (3.5 mults/access) for operand stack and call stack, saving ~25-40% on those accesses
- B) No — single RAM primitive for everything, simpler design
- **C) Not applicable in the original framing — Wasm is compiled to a register representation** ✓

> **Answer:** The Wasm operand stack is compiled away: the zkVM translates Wasm's stack-based IR to a register representation, so operand push/pop becomes register read/write. There is no runtime operand stack to track with a specialized stack primitive. Registers are stored in a separate RAM instance from linear memory. The call stack may still require a structure, but this is a design detail for the full design document. Specialized stack/queue primitives (ZK Stacks & Queues) remain available as a future optimization if a LIFO/FIFO access pattern emerges in the final design.

**Q5.4: How many distinct memory regions does the zkVM need?**
- A) One region (linear memory only) — operand stack and call stack are managed within the proof circuit's register-forwarding mechanism
- **B) Multiple regions — at least linear memory, registers, and globals** ✓
- C) Not sure — recommend based on Wasm execution model

> **Answer:** Multiple distinct memory regions:
> - **Linear memory** — byte-level Two Shuffles RAM for Wasm linear memory.
> - **Register file** — separate structure for registers (Wasm locals are compiled to registers; the operand stack is compiled away).
> - **Globals** — likely a separate RAM for Wasm globals.
> - **Call stack** — structure TBD; how call/return is handled is not yet decided.
>
> The exact number and nature of regions will be refined in the design document. The register file and globals may use simpler/cheaper RAM variants than linear memory depending on access patterns and size.

---

## 6. Taint Tracking

**Q6.1: How should taint be represented in the proof circuit?**
- A) As a separate 1-bit authenticated value per trackable unit (byte, local, global)
- B) Implicit in the structure — concrete values are known wire values, symbolic values are committed wire values
- **C) Not in the circuit — taint is a publicly computable abstract data structure** ✓

> **Answer:** Taint is not part of any proof circuit. It is an abstract data structure that is publicly computable by both parties from the program structure and the initial visibility annotations (the call configuration). Both parties track taint independently to determine which values are concrete vs symbolic, which in turn determines how values are handled (concrete values are plaintext, symbolic values are authenticated via IT-MACs). No proof is needed for taint correctness — it is a deterministic function of public information.

**Q6.2: How should taint propagation be enforced?**
- A) Eagerly — each instruction computes output taint from input taints as part of its constraint circuit
- **B) Outside the proof circuit — tracked by the embedder** ✓
- C) Not sure

> **Answer:** Taint propagation is tracked by the embedder outside the proof circuit. It follows deterministically from the VC spec's rules (default rule, annihilator exceptions, select, memory load/store) applied to publicly known program structure and initial visibility annotations. The embedder uses taint to decide how to handle each value (plaintext for concrete, IT-MAC for symbolic), but taint itself is never a wire in the proof.

**Q6.3: Are taint updates (symbolic → concrete via reveal) handled inside or outside the proof circuit?**
- A) Inside — the RAM or memory model tracks taint and supports updates mid-execution
- **B) Outside — reveal is an embedder/protocol-level operation** ✓
- C) Not sure

> **Answer:** Outside. Reveal (symbolic → concrete) is an embedder/protocol-level operation: the parties execute a sub-protocol to open the authenticated value, and the embedder updates its taint map. The proof circuit does not model taint transitions. From the circuit's perspective, post-reveal values are simply concrete — the circuit for subsequent instructions sees a plaintext value rather than an IT-MAC commitment.

---

## 7. Type Conversions (Arithmetic ↔ Boolean)

**Q7.1: Does the zkVM need arithmetic-to-boolean and boolean-to-arithmetic conversions?**
- **A) Yes — conversions are needed, but the mechanism and frequency depend on cost analysis** ✓
- B) No — the binary-field preference (Q2.1) means most operations stay in one domain; conversions to prime field only needed for specific sub-protocols (e.g., permutation checks)
- C) Yes, but only for specific sub-protocols — not general instruction-level conversions

> **Answer:** Yes. Conversions between domains are needed — Wasm freely mixes arithmetic and bitwise operations, and the RAM operates over a prime field (Q5.1, Q5.2) while the primary domain is binary (Q2.1). The specific conversion mechanism (Mystique zk-edaBits, COPEe-based conversion, or other approaches), the direction of conversion, and how aggressively to minimize domain crossings all depend on quantitative cost analysis that is not yet complete.

**Q7.2: If conversions are needed, how aggressively should they be optimized?**
- A) Full Mystique integration — preprocessed zk-edaBits, ~30-45μs per conversion
- B) Naive bit decomposition circuits — simpler but more expensive
- **C) Defer — note as optimization for later** ✓

> **Answer:** Deferred. The conversion mechanism is acknowledged as needed (Q7.1) but the specific optimization strategy (Mystique zk-edaBits, COPEe, naive circuits) is left for later cost analysis. The design document should identify where conversions occur and note that this is an optimization target.

---

## 8. Control Flow on Symbolic Values

**Q8.1: Should the zkVM support branching on symbolic values?**
- **A) Yes — with adaptive strategy per branch point** ✓
- B) No — require all branch conditions to be concrete; abort if program branches on symbolic value
- C) Defer — design for concrete-only initially, leave room for symbolic later

> **Answer:** Yes. The zkVM supports branching on symbolic values, with the compilation strategy selecting the cheapest approach per branch point:
> - **Simple cases** (e.g., if/else with small bodies): prove a disjunction over the two branches using LogRobin++ or Batchman.
> - **Complex cases** (e.g., loops with symbolic bounds, deeply nested symbolic control flow): fall back to full CPU emulation with padded execution to hide the taken path.
>
> The zkVM switches between these modes dynamically based on the branch structure, which is known at compile time from Wasm's structured control flow. This is consistent with the hybrid dispatch model (Q4.1) and path-visibility semantics (Q4.4).

**Q8.2: How should the annihilator exceptions (imul by 0, iand by 0, ior by all-ones) interact with the proof circuit?**
- **A) Emit a constant — the symbolic operand is unconstrained and discarded** ✓
- B) Not sure — needs further analysis

> **Answer:** Always emit a constant. When taint tracking (which is public, Q6.1) determines that an annihilator exception applies, the circuit simply outputs the known constant value (0 for `imul`/`iand`, all-ones for `ior`). The symbolic operand is unconstrained and not read by the circuit. No proof linking the symbolic input to the output is needed — the result is fully determined by the concrete annihilator operand.

---

## 9. Symbolic Addressing

**Q9.1: Should the zkVM support memory loads/stores with symbolic addresses?**
- **A) Yes — addresses can be symbolic (private)** ✓
- B) No — all addresses must be concrete; abort on symbolic address
- C) Defer — design for concrete-only, note as future extension

> **Answer:** Yes. Memory loads/stores can have symbolic (private) addresses. Two Shuffles RAM (Q5.2) natively supports this — the address is a private input from the prover in each RAM access, and the permutation check enforces correctness without revealing the address to the verifier.

---

## 10. VCI Integration (Reveal Mechanism)

**Q10.1: How should the VCI reveal operation interact with the proof circuit?**
- A) Reveal is a sub-protocol — the zkVM pauses, executes a reveal protocol between the parties, then resumes with the value now concrete
- **B) Reveal is batched — requests are collected and executed in bulk at synchronization points** ✓
- C) Not sure — this is a protocol design question

> **Answer:** Batched. Reveal requests are collected as they occur (via `reveal_<type>`) and executed in bulk when the guest calls `reveal_<type>_wait` or at other synchronization points. This aligns with the VCI spec's handle-based async model and allows the embedder to optimize network round-trips (e.g., opening multiple IT-MAC commitments in a single batch).

**Q10.2: Should reveal requests be provable (the verifier can confirm that the revealed value matches the symbolic value)?**
- **A) Yes — core correctness requirement** ✓
- B) No — reveal correctness is handled by the outer MPC protocol, not the ZK circuit
- C) Not sure

> **Answer:** Yes. The proof must link the revealed concrete value to the symbolic wire. This is a core correctness requirement — the verifier must be convinced that the opened value is the same value that was committed and operated on during execution. The IT-MAC structure provides this naturally: opening `[x]` means P sends `(x, M[x])` and V checks `K[x] = M[x] + Δ·x`.

---

## 11. Floating Point

**Q11.1: Should the initial zkVM support floating-point operations?**
- A) Yes — full f32/f64 support with NaN canonicalization
- **B) No — integer-only (i32, i64); floating point is out of scope** ✓
- C) Deferred — note as a future extension, design should not preclude it

> **Answer:** No. The initial zkVM supports i32 and i64 only. Floating-point (f32, f64) is out of scope for the initial design.

---

## 12. Wasm Feature Coverage

**Q12.1: What Wasm feature set should the initial zkVM target?**
- A) MVP only (Wasm 1.0) — the minimal feature set
- **B) MVP + bulk memory operations (memory.copy, memory.fill)** ✓
- C) MVP + bulk memory + multi-value returns + reference types
- D) Not sure — recommend a pragmatic starting set

> **Answer:** MVP + bulk memory operations. The bulk memory proposal (memory.copy, memory.fill) is widely used by compilers and the VC spec explicitly defines taint semantics for both operations.

**Q12.2: Should the design document enumerate which Wasm instructions are supported?**
- **A) Yes — with per-instruction cost estimates** ✓
- B) Yes — but without detailed cost estimates (just supported/unsupported)
- C) No — the design document covers architecture; instruction coverage is a separate document

> **Answer:** Yes, with per-instruction cost estimates. The design document should include a table of supported Wasm instructions with cost in terms of multiplication gates, VOLE correlations, RAM accesses, and domain conversions where applicable.

---

## 13. VOLE Correlation Generation

**Q13.1: Should the design document specify which VOLE generation protocol to use?**
- A) Yes — specify Ferret or another concrete protocol
- **B) No — treat VOLE as a black box** ✓
- C) Briefly note Ferret as the expected instantiation, but treat VOLE as a black box in the design

> **Answer:** No. The design assumes black-box access to sVOLE correlations. The VOLE generation protocol is an implementation detail outside the scope of the zkVM design.

---

## 14. Protocol Exclusions

**Q14.1: Confirm: exclude all protocols requiring additively homomorphic encryption (AHE)?**
- **A) Yes — exclude AntMan's sublinear mode and Justvengers entirely** ✓
- B) Exclude Justvengers but keep AntMan's sublinear mode as an option
- C) Revisit — maybe AHE is acceptable if the gains are large enough

> **Answer:** Yes. All protocols requiring AHE (BGV or similar) are excluded. This eliminates AntMan's sublinear communication mode and Justvengers entirely. The zkVM uses only primitives built on VOLE, IT-MACs, and hash functions.

**Q14.2: Include AntMan's SIMD mode (no HE, just batched QuickSilver) as an optimization path?**
- A) Yes — include for repeated sub-circuits (hash rounds, signature verification)
- B) No — keep the protocol set minimal
- **C) Mention as a future optimization only** ✓

> **Answer:** Future optimization only. AntMan's SIMD mode (batched QuickSilver, no HE) is noted as a potential optimization for repeated sub-circuits but is not part of the initial design.

**Q14.3: Include VOLE-in-the-Head (public verifiability compiler)?**
- A) Yes — include as a concrete design path
- B) Mention as a future path only
- **C) Out of scope entirely** ✓

> **Answer:** Out of scope. The zkVM is a designated-verifier interactive system. Public verifiability via VOLE-in-the-Head is not considered.

---

## 15. Document Scope & Framing

**Q15.1: How prescriptive should the protocol selection document be?**
- **A) Single concrete recommendation per role — "use X for Y"** ✓
- B) Present trade-offs with multiple options, highlight a preferred choice
- C) Exploratory — present options without strong recommendations

> **Answer:** Prescriptive. The document makes a single concrete recommendation per functional role, with brief justification. Trade-offs are acknowledged but a clear decision is stated.

**Q15.2: Who is the primary audience?**
- A) An implementer who will build this system
- B) A reviewer/designer who will refine the design further
- **C) Both** ✓

> **Answer:** Both. The document should be precise enough for an implementer to act on, and clear enough for a reviewer to evaluate and refine the design.

**Q15.3: What should the document be named?**
- **A) `decisions.md`** ✓
- B) `protocol-selection.md`
- C) Something else (suggest)

> **Answer:** `decisions.md`.
