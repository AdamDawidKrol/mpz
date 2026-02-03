# A 2.1 KHz Zero-Knowledge Processor with BubbleRAM

**Authors**: David Heath, Vladimir Kolesnikov
**Venue**: ACM CCS 2020 | **ePrint**: [2020/809](https://eprint.iacr.org/2020/809)

## Role in the VOLE-ZK Design Space

This paper is the first concretely efficient ZK processor -- a complete von Neumann machine executing arbitrary programs and proving correctness in zero knowledge. The central contribution is BubbleRAM, a ZK-specific ORAM that costs only `(1/2) log^2 n` OTs per access, beating linear scan for arrays larger than 3 elements and outperforming generic ORAMs (Floram, Lookahead) by 1-2 orders of magnitude in communication. The system is garbled-circuit-based (cast as a verifiable garbling scheme in the JKO13 framework), not VOLE-based, but it establishes the foundational architecture that all subsequent ZK CPUs build on: a fixed ISA with register file, ROM for instructions, RAM for data, and a cycle-by-cycle proof of correct execution. The key architectural insight -- that the prover knows all future access patterns and can pre-sort memory accordingly -- directly influenced later constant-overhead ZK RAM designs. For a VOLE-based zkVM, BubbleRAM's approach is superseded by permutation-check-based RAMs with O(1) gates per access, but its processor architecture, arithmetic MAC representation, and input-expansion technique remain relevant design references.

## Key Ideas

### Verifiable Garbling Scheme Framework

The system is formalized as a verifiable garbling scheme `(ev, Gb, En, Ev, De, Ve)` plugged into the JKO13 ZK protocol. V acts as GC generator, P acts as evaluator. The protocol is:

1. P evaluates the garbled circuit and commits to the output encryption
2. V sends all randomness used to encrypt the circuit
3. P re-encrypts to verify V's encryption is valid
4. P opens commitment to the output

This yields malicious-verifier ZK in constant rounds. All OT inputs are precomputed, so round complexity equals that of the underlying JKO protocol.

### Arithmetic MAC Representation

For a global secret `Delta in Z_q` (held by V) and per-wire mask `A in Z_q` (held by V), a value `a` is authenticated as:

```
[[a]] = <A, a * Delta - A>    (V holds A, P holds a * Delta - A)
```

Properties:
- **Additive homomorphism**: `[[a]] + [[b]] = [[a + b]]` -- free (no communication)
- **Subtraction, public-constant multiplication**: free
- **Public constant encoding**: `[[c]] = <c * Delta, 0>`
- **Unforgeability**: P cannot find `a' * Delta - A` for `a' != a` without knowing `A` and `Delta`; success probability `1/q`

The prime `q = 2^64 - 59` (largest 64-bit prime), giving `sigma = 64` bits of statistical security and ensuring `q > (2^32 - 1)^2` (no overflow during 32-bit multiplication).

### Non-Homomorphic Operations via OT

**Boolean multiplication (`mul1`)** -- multiply `[[a]]` by `[[b]]` where `a in {0,1}`. Cost: **1 OT**.

1. P locally computes `a * (b * Delta - B) = a*b*Delta - a*B`
2. Problem: V doesn't know mask `a*B`. Fix via OT: V offers `(0 - C, B - C)`, P selects based on `a`, gets `a*B - C`
3. Result: `<C, a*b*Delta - C>` -- a valid MAC of `a*b`
4. To enforce honest OT selection, V also offers `(0 - A', Delta - A')`. P gets a second MAC `a*Delta - A'` and proves consistency by computing `(a*Delta - A) - (a*Delta - A') = A' - A`, a MAC of 0

**32-bit multiplication (`mul32`)** -- generalizes `mul1` to `a in Z_{2^32}`. P decomposes `a` into 32 bits, performs 32 OTs (one per bit), reconstructs MACs homomorphically. Cost: **32 OTs**.

**Projection (`project`)** -- decompose `n`-bit value into `n` authenticated bits. P uses bits of `a` as `n` OT selections, receives individual bit MACs, reconstructs combined MAC and proves equality with original. Cost: **n OTs**.

**mod 2^32** -- project to 64 bits, reconstruct high 32 bits homomorphically, subtract. Cost: **64 OTs** (64-bit projection).

**Reading P's input (`readInput`)** -- 32 OTs to read one 32-bit input value, gated by an authenticated flag (using `mul1`). Cost: **32 OTs + 1 OT**.

### Proof Mechanism

P accumulates "proofs of zero" (`zerosActual`): MACs that should encode 0 if P followed the protocol. V independently accumulates `zerosExpected`. At protocol end:

```
P sends H(zerosActual)
V checks H(zerosActual) == H(zerosExpected)
```

ZK holds because each zero-MAC is value-independent (always encodes 0 regardless of P's input).

### Processor Architecture (ZKM)

State machine with 32-bit values (`nu = Z_{2^32}`):
- **32 registers** (stored in a BubbleRAM of size 32)
- **Main memory** (BubbleRAM of parameterized size `space`)
- **Program ROM** (read-only memory holding instructions)
- **Program counter** (authenticated value)
- **P's input** (read-only stack)

Instructions are 4-tuples `(op, src0, src1, tar)`. Op-codes: `add`, `mul`, `lt`, `beqz`, `load`, `store`, `input`, and other arithmetic/comparisons.

**Authenticated step** (`step_hat`): Unlike cleartext `step` which conditionally dispatches, the authenticated step performs ALL operations every cycle and uses algebra to select the correct result:

```
val = inp
val = val + ((op == add) * (arg0 + arg1))
val = val + ((op == mul) * (arg0 * arg1))
val = val + ((op == lt)  * (arg0 < arg1))
val = val + ((op == load) * m)
...
val = mod32(val)
```

Each `(op == X)` comparison produces an authenticated bit; multiplication by that bit selects the result. V learns nothing about which operation was performed.

**Op-code decoding optimization**: Instead of repeated comparisons against `op`, a binary decoder circuit generates a one-hot bitmap. This is much cheaper than individual equality tests.

### 2 log n ROM

Based on oblivious permutation (Waksman networks) and selection blocks from KS08:

1. P predicts instruction access order (by running program in cleartext)
2. Every `n` reads, apply a **selection block**:
   - Determine which of `n` instructions are needed in next `n` accesses
   - Permute ROM so each needed element with `i` copies is followed by `i-1` unneeded elements
   - Linear scan replacing unneeded elements with copies (1 OT per swap gate)
   - Permute copies into access order
3. Each element stored alongside its index; on access, P proves provided index equals stored index

Cost: 2 permutations (`2 * (n log n - n/2)` OTs) + 1 linear scan (`n` OTs) per `n` reads = **amortized 2 log n OTs per read**.

### BubbleRAM: (1/2) log^2 n RAM Access

Core idea: P knows all future accesses (by pre-executing in cleartext) and uses this to keep "hot" elements (needed soon) near the front of the array.

**Invariant**: At timestep `t`, for all `i in [0..log n]`, the next `2^i - (t mod 2^i)` memory accesses are located in the first `2^i` RAM slots.

**Oblivious partition**: Selects half of the first `2i` elements and moves them to the first `i` slots. Implemented via Waksman permutation network. Cost of partition on `2i` elements: `i log i + i/2` OTs.

**Rearrange** (before each access at timestep `t`):
```
for i in [log|array| - 1 .. 0]:
    if (2^i | t): partition(array[0..2^{i+1}], selection)
```

Example: At timestep 4, partition sizes 8, 4, 2. At timestep 5, partition size 2 only.

**Swap gate** for 32-bit values using arithmetic MACs:
```
delta = b * (x - y)     // 1 OT (mul1)
return (x - delta, y + delta)
```

Full Waksman permutation of `n` elements: `n log n - n/2` swap gates = `n log n - n/2` OTs.

**Amortized cost derivation**: A partition of size `2^i` costs `i log i + i/2` OTs, amortized across `i` subsequent accesses. Each element travels through `log n` partitions to reach slot 0. Total amortized cost per access:

```
sum_{i=0}^{log n - 1} (log(2^i) + 1/2) = (1/2) log^2 n
```

**Access protocol**:
1. `initRAM(size)`: Initialize array of `(index, value)` pairs, all values = 0, timestep `t = 0`
2. `accessRAM(ram, write_flag, index, write_value)`:
   - Apply `rearrange` to move hot items forward
   - Read element at slot 0
   - P proves `argument_index == stored_index` (appends difference to `zerosActual`)
   - If `write_flag` set, overwrite element at slot 0
   - Increment `t`
   - Return looked-up element

### Input Expansion

P's input must be binary (projective garbling scheme requirement for JKO13). P runs the processor in `INPUT` mode to expand her 32-bit integer inputs into all OT selection bits (including auxiliary bits for multiplications, projections, permutations). All OTs are then completed in constant rounds.

Three modes: `VERIFIER` (V sets up OT inputs), `PROVER` (P uses OT outputs), `INPUT` (P derives OT selection bits). A fourth `CHECK` mode handles `Ve` (malicious-verifier verification).

## Cost Model

### Per-Cycle OT Cost (No Main Memory)

| Component | OTs |
|-----------|-----|
| Decode instruction (binary decoder) | 86 |
| Registry RAM read (amortized, 32-entry BubbleRAM) | 26 |
| Read P input | 32 |
| Multiplication / project arg0 | 32 |
| Bitwise multiplication / project arg1 | 32 |
| Comparisons | 65 |
| Misc. Boolean multiplications | 25 |
| Registry RAM write (amortized) | 14 |
| mod 2^32 | 64 |
| **Total (amortized)** | **376** |

### Communication Per OT

Each OT communicates **48 bytes**: 16 bytes for random OT + 32 bytes for both secret pairs (via KOS15 malicious OT extension).

### Per-Cycle Communication (No Main Memory)

```
376 OTs * 48 bytes/OT = 18,048 bytes ~ 17.6 KB per cycle
```

### BubbleRAM Cost Per Access

| RAM size `n` | Amortized OTs/access | Communication/access (KB) |
|--------------|---------------------|--------------------------|
| 2^1 (2) | ~0.5 | ~0.02 |
| 2^5 (32) | ~12.5 | ~0.59 |
| 2^9 (512) | ~40.5 | ~1.90 |
| 2^13 (8K) | ~84.5 | ~3.96 |
| 2^17 (128K) | ~144.5 | ~6.77 |

Formula: `(1/2) log^2 n` OTs per access; each OT = 48 bytes.

### Total Cycle Cost With Main Memory

For a 512KB RAM (`n = 2^17 = 131072` cells of 32 bits):

```
Cycle core:                    376 OTs  (17.6 KB)
Main memory access (amortized): ~144 OTs (~6.8 KB)
Total:                         ~520 OTs (~24.4 KB)
```

Paper reports ~24 KB per cycle at 512KB main memory.

### Comparison: BubbleRAM vs Generic ORAMs (Communication per access)

| `n` | BubbleRAM (KB) | Floram (KB) | Speedup | Lookahead (KB) | Speedup |
|-----|---------------|-------------|---------|----------------|---------|
| 2^5 | 0.61 | ~600 | 980x | ~5 | 8x |
| 2^7 | 1.05 | ~610 | 580x | ~25 | 23x |
| 2^9 | 1.77 | ~620 | 350x | ~50 | 28x |
| 2^11 | 2.69 | ~640 | 240x | ~90 | 33x |
| 2^13 | 3.82 | ~670 | 175x | ~160 | 42x |
| 2^15 | 5.13 | ~700 | 135x | -- | -- |
| 2^17 | 6.63 | ~730 | 110x | -- | -- |

### ROM Cost

| Operation | OTs | Notes |
|-----------|-----|-------|
| Selection block (per `n` reads) | `2(n log n - n/2) + n` | 2 permutations + 1 linear scan |
| **Per read (amortized)** | **2 log n** | |

### Operation Costs

| Operation | OTs | Notes |
|-----------|-----|-------|
| Addition / subtraction | 0 | Homomorphic |
| Multiply by public constant | 0 | Homomorphic |
| Boolean multiply (`mul1`) | 1 | `a in {0,1}` |
| 32-bit multiply (`mul32`) | 32 | General `a in Z_{2^32}` |
| Projection (`project`, `n` bits) | `n` | Bit decomposition |
| mod 2^32 | 64 | 64-bit projection + arithmetic |
| ROM read (amortized) | `2 log n` | |
| RAM access (amortized) | `(1/2) log^2 n` | BubbleRAM |
| Waksman permutation (`n` elements) | `n log n - n/2` | Swap gates |
| Partition (`2i` elements) | `i log i + i/2` | Special-case permutation |

## Concrete Performance

### Benchmark Environment

- Hardware: MacBook Pro, Intel Dual-Core i5 3.1 GHz, 8 GB RAM
- Network: Simulated 1 Gbps LAN, 2 ms latency
- Implementation: 1900 lines of C++
- OT instantiation: KOS15 malicious OT extension (EMP-toolkit)
- One-time base OT cost: 150 KB

### Processor Clock Rate

| Configuration | Clock Rate | Comm/Cycle |
|---------------|-----------|------------|
| No main memory | ~2.1 KHz | ~17.6 KB |
| 512 KB main memory (2^17 cells) | ~2.1 KHz | ~24 KB |

Peak performance of **2.1 KHz** achieved on sorting benchmarks (100-300 element arrays) at 1 Gbps.

### ZK Bugs Benchmark (from HK20)

| System | Time | Notes |
|--------|------|-------|
| HK20 (stacked garbling) | 0.1 s | Bounded loops, no general control flow |
| **ZKM (this work)** | **0.42 s** | Unbounded loops, general control flow |

### Quick Sort + Bug Detection

| Array size | Cycles | Wall-clock time | Comm |
|------------|--------|----------------|------|
| 100 | ~15K | ~7 s | ~360 MB |
| 200 | ~40K | ~19 s | ~960 MB |
| 300 | ~65K | ~31 s | ~1.6 GB |
| 500 | 101K | ~48 s | ~2 GB |

Main memory: 2^12 cells (16 KB). Times averaged over 5 runs.

### Comparison: ZK Processors

| System | Clock Rate | Memory | Interactive | Setting |
|--------|-----------|--------|-------------|---------|
| **ZKM (this work)** | **2.1 KHz** | **512 KB** | Yes | Commodity laptop |
| BCTV14 (SNARK processor) | < 10 Hz | ~100s of bits | No | Powerful hardware |
| BCG+13 (SNARK for C) | < 10 Hz | ~100s of bits | No | Powerful hardware |

ZKM is ~200x faster than SNARK-based processors, with ~1000x more memory, at the cost of interactivity and proportional verifier work.

## Key Definitions

- **BubbleRAM** -- A ZK-specific ORAM where P pre-sorts memory so that elements needed soonest ("hot") are closest to index 0. Based on repeated oblivious partitions using Waksman networks. Amortized cost: `(1/2) log^2 n` OTs per access. Beats linear scan for `n > 3`.
- **Oblivious partition** -- Select half of the first `2i` elements in an array and move them to the first `i` slots, using a Waksman permutation network. P programs the partition based on future access knowledge. Cost: `i log i + i/2` OTs.
- **Waksman permutation network** -- Recursive construction of `n log n - n/2` conditional swap gates that realizes any permutation of `n` elements. Each swap gate costs 1 OT (via `mul1` on the arithmetic MAC representation).
- **Arithmetic MAC** -- Representation `[[a]] = <A, a*Delta - A>` where V holds mask `A` and global key `Delta`, P holds `a*Delta - A`. Supports free addition/subtraction and public-constant multiplication. Non-homomorphic operations (multiplication, projection) require OTs.
- **`mul1` (Boolean multiplication)** -- Multiply `[[a]] * [[b]]` where `a in {0,1}`. Cost: 1 OT. P selects based on `a`, receives product share and a second MAC of `a` to prove honest selection.
- **Verifiable garbling scheme** -- A 6-tuple `(ev, Gb, En, Ev, De, Ve)` satisfying correctness, soundness, and verifiability. When plugged into the JKO13 protocol, yields malicious-verifier ZK. The system is privacy-free (not suited for 2PC).
- **ZKM.ev** -- Cleartext processor specification. Executes `T` cycles of the ISA; outputs whether register 0 contains 1.
- **Proofs of zero (`zerosActual` / `zerosExpected`)** -- MACs that should encode 0 if P followed the protocol. Accumulated during execution, then compared via collision-resistant hash `H`. ZK holds because zero-MACs are value-independent.
- **Input expansion** -- Process by which P converts her 32-bit integer inputs into all binary OT selection bits (including auxiliary bits for multiplications, projections, and permutation programming). Runs in `INPUT` mode before the main protocol. Enables all OTs in constant rounds.
- **Temperature gradient (invariant)** -- At timestep `t`, for all `i`, the next `2^i - (t mod 2^i)` addresses to be accessed reside in the first `2^i` RAM slots. Maintained by `rearrange` applying partitions of decreasing size.

## Relevance to VOLE-Based zkVM

1. **Foundational ZK processor architecture.** ZKM establishes the template used by all subsequent ZK CPUs: a fixed ISA executed cycle-by-cycle, with ROM for instructions, RAM for data, and a register file. A VOLE-based zkVM will use the same high-level architecture even though the underlying proof mechanism changes from GC/OT to VOLE correlations.

2. **Prover-knows-the-future RAM paradigm.** BubbleRAM's core insight -- that in ZK the prover can precompute the entire access trace and sort memory accordingly -- is the conceptual ancestor of all subsequent ZK RAM constructions (timestamp-based, permutation-check-based). The "two shuffles" RAM (Yang-Heath, USENIX '24) replaces BubbleRAM's `O(log^2 n)` with `O(1)` gates per access, but the underlying principle is the same.

3. **Arithmetic MAC representation is a precursor to VOLE-ZK MACs.** The MAC structure `[[a]] = <A, a*Delta - A>` over `Z_q` is structurally identical to the IT-MAC used in QuickSilver and Wolverine (where VOLE provides the correlation `a*Delta + B`). BubbleRAM instantiates MACs via OT extension; a VOLE-based system gets them directly from subfield VOLE, eliminating per-operation OT overhead.

4. **Cost baseline for RAM-heavy programs.** At `(1/2) log^2 n` OTs per access and ~6.6 KB per access for 128K-entry memory, BubbleRAM quantifies the cost of the ORAM-based approach. Constant-overhead VOLE-based RAMs (~50 bytes/access at `n = 2^20`) are ~100x more communication-efficient, providing a concrete improvement target.

5. **Cycle cost breakdown informs ISA design.** The per-cycle cost analysis (Figure 6) reveals that instruction decoding (86 OTs), comparisons (65 OTs), and mod 2^32 (64 OTs) dominate. A VOLE-based zkVM should minimize these costs: native field arithmetic avoids mod clamping, range checks replace bit-decomposition-based comparisons, and lookup-table-based decoding replaces Boolean decoder circuits.

6. **All-operations-every-cycle overhead.** The authenticated step executes all ISA operations and selects the result via multiplication by a one-hot opcode indicator. This `O(|ISA|)` multiplicative overhead per cycle is a fundamental cost of the processor model. Later work (e.g., Stacked Garbling for disjunctive ZK, segment-based execution) can amortize this; a VOLE-based zkVM should consider similar techniques.

7. **Round complexity model.** BubbleRAM achieves constant rounds by having P precompute all OT inputs (input expansion) before protocol execution. A VOLE-based zkVM similarly benefits from preprocessing all VOLE correlations upfront, enabling streaming proof generation without additional rounds for memory operations.

8. **Historical performance anchor.** At 2.1 KHz on a 2020 commodity laptop with GC-based proofs, this sets the baseline for ZK processor throughput. Modern VOLE-based approaches (QuickSilver achieving MHz-range gate throughput) should target 100-1000x improvement in effective ISA cycle rate, which the shift from `O(log^2 n)` RAM to `O(1)` RAM and from OT-per-operation to VOLE-per-operation enables.
