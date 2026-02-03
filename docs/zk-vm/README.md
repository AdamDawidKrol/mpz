# VOLE-Based zkVM Design

This directory contains the research artifacts and design documents for a
VOLE-based zero-knowledge virtual machine (zkVM). The zkVM implements the
[Verifiable Compute (VC) specification](https://sinui0.github.io/vc-spec/),
which defines a two-party verifiable computation model layered on WebAssembly
with a visibility and taint system.

## Context

The VC spec defines two parties — local and remote — executing a WebAssembly
program where values carry visibility annotations: **public** values are
concrete and known to both parties, while **private** and **blind** values are
symbolic and processed through zero-knowledge constraints. The zkVM is the
mechanism that enforces correctness of the symbolic execution. It assumes
black-box access to VOLE correlations (e.g. via the Ferret protocol).

## Process

The design of this zkVM follows a structured, multi-phase process. Each phase
produces artifacts in this directory.

### Phase 1: Literature Survey

We identified and collected 25 research papers spanning the topics required
to build a VOLE-based zkVM:

- **VOLE-ZK core protocols** — the foundational paradigm of IT-MAC-based
  zero-knowledge proofs over VOLE correlations (Wolverine, LPZK, QuickSilver,
  Mac'n'Cheese).
- **Arithmetic domain** — techniques for working in different algebraic
  settings, including boolean-to-arithmetic conversions and native computation
  over rings Z\_{2^k} (Mystique, Mozzarella).
- **Communication optimization** — protocols that reduce proof size, including
  sublinear-communication approaches (AntMan, VOLE-in-the-Head).
- **zkVM design patterns** — architectural ideas from existing zkVM designs,
  particularly lookup-based approaches (Jolt, Lasso).
- **RAM proofs** — authenticated memory access protocols suitable for
  VOLE-based ZK, from early generic compilers through to the current
  state-of-the-art (BubbleRAM, Spice, RAM-from-Circuit-ZK,
  Constant-Overhead-ZK-RAM, Two Shuffles RAM, ZK Stacks & Queues).
- **Disjunction / branching** — protocols for proving one-out-of-many branch
  selection, which is the core primitive for instruction dispatch in a zkVM
  (Mac'n'Cheese, LogRobin++, Batchman & Robin, Tight ZK CPU, JesSeQ,
  Justvengers).
- **Theoretical background** — foundational work on linear interactive proofs,
  sum-check protocols, and IOP constructions (Spartan, Linear Interactive
  Proofs, ZK-IOP Linear Prover).

The PDFs are stored in `papers/` and compressed summaries (`.md`) are in
`papers/summaries/`.

### Phase 2: Paper Summarization

Each paper was summarized into a structured markdown document optimized for
use by a design agent. The summaries contain:

- Full protocol mechanics (steps, equations, sub-procedures).
- Cost models (communication, computation, VOLE consumption).
- Concrete benchmark data where available.
- Key definitions and terminology.
- An assessment of relevance to the VOLE-based zkVM design.

The summaries are intentionally compressed: they omit security proofs and
focus on the information needed to make protocol selection and integration
decisions.

### Phase 3: Protocol Selection

Using the paper summaries as input, the next phase produces a **decisions
document** that explains which protocols are selected for each functional role
in the zkVM and why. This includes:

- Which primitives and assumptions the zkVM accepts (and which it rejects).
- Which protocol fills each role: gate checking, instruction dispatch,
  memory access, arithmetic domain, type conversions, etc.
- Trade-off analysis where multiple candidates exist.
- Identification of protocols that are excluded due to unacceptable
  dependencies (e.g. reliance on additively homomorphic encryption).

### Phase 4: zkVM Design

With protocol selections finalized, the final phase produces the zkVM design
document. This covers the end-to-end architecture: how WebAssembly execution
maps to proof constraints, how the VC spec's visibility system integrates
with the proof backend, and how the selected protocols compose into a
coherent system.

## Directory Structure

```
docs/zk-vm/
├── README.md              # This file
├── papers/
│   ├── <name>.pdf         # Original paper PDF
│   └── summaries/
│       └── <name>.md      # Compressed summary
```

