# AGENTS.md - Coding Agent Guidelines for mpz

This document provides guidelines for AI coding agents working in this repository.

## Project Overview

mpz is a Rust workspace for multi-party computation (MPC) primitives. It contains ~30 crates
organized around core cryptographic protocols: oblivious transfer, garbled circuits, secret
sharing, zero-knowledge proofs, and supporting infrastructure.

**Key architectural pattern**: Core vs IO separation. Protocol logic lives in `*-core` crates
(no async, no I/O), while the main crate adds async I/O using `futures` Sink/Stream traits.

## Build Commands

```bash
# Build entire workspace
cargo build

# Build specific crate (preferred when working on one crate)
cargo build -p mpz-ot

# Build with all features
cargo build -p mpz-ot --all-features

# Check without building
cargo check -p mpz-ot
```

## Test Commands

```bash
# Run all workspace tests
cargo test

# Run tests for a specific crate (PREFERRED)
cargo test -p mpz-ot

# Run a single test by name
cargo test -p mpz-ot test_kos_rcot

# Run tests with specific features
cargo test -p mpz-ot --features "ideal,test-utils"

# Run tests with output visible
cargo test -p mpz-ot -- --nocapture
```

**Important**: When working on a specific crate, run commands at the crate level, not workspace.

## Linting and Formatting

```bash
# Format code (uses nightly for full feature support)
cargo +nightly fmt --all

# Check formatting
cargo +nightly fmt --check --all

# Run clippy (nightly, all features, treat warnings as errors)
cargo +nightly clippy --workspace --exclude mpz-wasm-bench --all-targets --all-features -- -D warnings

# Clippy for wasm-bench (requires wasm target)
cargo +nightly clippy --lib --target wasm32-unknown-unknown -p mpz-wasm-bench -- -D warnings

# Generate documentation
cargo doc --no-deps --workspace --lib --document-private-items --examples
```

## Code Style

### Formatting (rustfmt.toml)

- `imports_granularity = "Crate"` - Group imports by crate
- `wrap_comments = true` - Wrap long comments

### Imports

Group imports by crate, not individual items:
```rust
// Good
use std::{ops::Range, sync::Arc};
use mpz_core::{Block, aes::FixedKeyAes};

// Avoid
use std::ops::Range;
use std::sync::Arc;
```

When using a type, import it at the top of the module. Do NOT use fully qualified paths inline.

### Module Organization

Use `<module>.rs` and `<module>/<submodule>.rs` pattern. Do NOT use `mod.rs`:
```
src/
├── lib.rs
├── sender.rs       # Module file
├── receiver.rs
└── sender/         # Submodules for sender
    └── state.rs
```

### Error Handling

- Use `thiserror` for library errors (not `anyhow`)
- Prefer `Result` and `Option` over panicking; use `?` for propagation
- Define a crate-level `Result<T>` type alias when appropriate

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Expect Messages

State what should be true (the invariant), not what went wrong:
```rust
// Good
.expect("value should be present after initialization")

// Bad
.expect("value was missing")
```

### Documentation

- Write doc comments (`///`) for all public items
- Use `//!` for crate/module-level documentation
- Common crate-level lints:
```rust
#![deny(
    unsafe_code,
    missing_docs,
    unused_imports,
    unused_must_use,
    unreachable_pub,
    clippy::all
)]
```

### Visibility

Private-by-default. Minimize use of `pub` and `pub(crate)`. Only expose what's part of
the intentional API surface.

### Unsafe Code

Avoid `unsafe` unless necessary. If using unsafe:
- Document safety invariants with `// SAFETY:` comments
- Cover assumptions with `debug_assert!` where possible
- Some crates use `#![forbid(unsafe_code)]`

### Async Patterns

- Core crates should NOT contain async code (separation of concerns)
- Remain executor agnostic - no coupling to tokio runtime

### Type-States

The type-state pattern is encouraged for protocol implementations to make invalid states
unrepresentable at compile time.

### Naming Conventions

- Follow standard Rust naming: `snake_case` for functions/variables, `PascalCase` for types
- Crate names: `mpz-<name>` for main crates, `mpz-<name>-core` for core logic

## Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

For async tests with timeout:
```rust
#[tokio::test]
#[timeout(15000)]  // 15 second max
async fn test_async_operation() {
    // ...
}
```

## Working Guidelines

1. Make minimal, targeted changes - only modify what's asked
2. Don't add extra features, refactoring, or "improvements" unless requested
3. Verify changes compile with `cargo build` or `cargo test` at the crate level
4. Use `todo!()` for unimplemented sections
5. When encountering bugs, try to reproduce with a minimal unit test
6. Defer to user input when intent is unclear

## Architecture Notes

- Prefer RustCrypto crates for cryptographic primitives
- Transport agnostic - no coupling to TCP or specific transports
- Messages should be strongly typed with serde for serialization
