# Succinct Non-Interactive Arguments via Linear Interactive Proofs

**Authors**: Nir Bitansky, Alessandro Chiesa, Yuval Ishai, Rafail Ostrovsky, Omer Paneth
**Venue**: TCC 2013 | **ePrint**: [2012/718](https://eprint.iacr.org/2012/718)

## Role in the VOLE-ZK Design Space

This paper introduces the Linear Interactive Proof (LIP) model — interactive proofs where the prover is restricted to computing affine functions of the verifier's messages. LIPs are the theoretical foundation connecting linear PCPs (which underlie GGPR/Groth16-style SNARKs) to the VOLE-ZK paradigm (which also exploits linearity of the prover). LPZK explicitly identifies itself as an instantiation of LIPs.

## Key Ideas

### The LIP Model

A Linear Interactive Proof (LIP) is a two-message protocol where:
- The verifier sends a vector `q` of field elements (depending on the circuit C but not the input x)
- The prover applies an **affine function** `Pi = (Pi_0, b)` determined by the witness, computing `a = Pi_0 * q + b`
- The verifier checks the response using a decision algorithm of size O(|x|)

Both honest and malicious provers are restricted to affine functions — this is the "algebraically-bounded adversary" model. Soundness error is O(1/|F|).

### LIPs from Linear PCPs

A linear PCP (LPCP) is a PCP where the proof oracle computes a linear function `pi : F^m -> F`. The paper gives a transformation: any k-query LPCP of length m over F becomes an LIP with verifier message in F^{(k+1)m} and prover message in F^{k+1}. The key idea: the verifier sends all k queries plus a random linear combination; the prover responds to each, and the verifier checks consistency of the random combination to prevent the prover from using inconsistent linear functions.

Two LPCP instantiations yield concrete LIPs:
- **Hadamard-based** (from ALMSS): verifier message O(s^2), prover message F^4, degree-(2,2) verifier
- **QSP-based** (from GGPR): verifier message O(s), prover message F^4, degree-(O(s),2) verifier

### From LIPs to Preprocessing SNARKs

The cryptographic compiler is conceptually simple: encrypt each field element in the verifier's message with an additively-homomorphic encryption scheme that enforces "linear-only" operations. This forces the prover to "act linearly" even though it is computationally unbounded. Two flavors:
- **Designated-verifier**: LIP + linear-only encryption -> preprocessing SNARK
- **Publicly-verifiable**: algebraic LIP (low-degree verifier) + linear-only one-way encoding -> publicly-verifiable preprocessing SNARK

### Reinterpretation of Existing SNARKs

The paper reveals that Groth10, Lipmaa12, and GGPR13 — which appeared to bypass PCPs entirely — can all be reinterpreted as implicitly using linear PCPs compiled via linear-only encodings. This unifying perspective is the paper's main conceptual contribution.

### Zero Knowledge

HVZK (honest-verifier zero-knowledge) LIPs compile into ZK preprocessing SNARKs when the encryption/encoding supports rerandomization. For LPCPs with low-degree decision algorithms, a generic HVZK transformation is given.

## Relevance to VOLE-Based zkVM

The LIP model is the *exact* theoretical framework that explains why VOLE-based ZK works. In VOLE-ZK protocols:
- The verifier's "message" is the VOLE correlation: effectively a random vector encrypted under the global key Delta
- The prover's computation is **affine** in the VOLE correlations — it computes `M[x] = K[x] + Delta * x`, which is a linear function of (K[x], Delta)
- Soundness comes from the fact that the prover cannot deviate from this affine structure without being detected by the IT-MAC check

LPZK (the QuickSilver/Mac'n'Cheese line) makes this connection explicit by calling itself a "linear interactive proof over VOLE." The key insight is that VOLE correlations serve as the cryptographic enforcement mechanism (analogous to "linear-only encryption" in this paper) that forces the prover into the affine function space. The difference is that VOLE-based enforcement is information-theoretic (via IT-MACs) rather than computational (via encryption), which is why VOLE-ZK protocols can achieve unconditional soundness against an unbounded prover in the VOLE-hybrid model.

Understanding LIPs clarifies why VOLE-ZK protocols have the structure they do, and why the prover's computation is inherently linear-time: the prover's "proof" is an affine function of O(N) VOLE correlations, and evaluating an affine function on N inputs takes O(N) time.
