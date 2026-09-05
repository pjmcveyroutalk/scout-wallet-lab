# Scout Wallet Lab

Isolated development and security lab for Scout-native wallet, signing, policy,
settlement, and execution controls.

## Isolation contract

This repository is independent from the main Solana ARB Scout repository.
Nothing in this lab may modify, depend on, or deploy into the main Scout build
without an explicit future integration decision.

## Toolchain

Rust is pinned to exactly 1.80.0.

Expected verification commands:

```text
cargo +1.80.0 fmt --all -- --check
cargo +1.80.0 clippy --workspace --all-targets -- -D warnings
cargo +1.80.0 test --workspace --all-targets
```

## Security baseline

- No production or mainnet funds.
- No secrets committed to Git.
- No private-key material in logs, debug output, telemetry, or dashboard state.
- `unsafe` is forbidden.
- `unwrap`, `expect`, and `panic` are denied by workspace Clippy configuration.
- The Vercel dashboard is a control and observability surface, never a key vault.

## Current phase

Foundation only. Cryptographic dependencies and wallet functionality are added
only after the Rust 1.80 baseline is verified green.
