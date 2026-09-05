# Scout Wallet Lab

Isolated development and security lab for Scout-native wallet, signing, policy,
ledger, Devnet access, observability, and execution controls.

## Isolation contract

This repository is independent from the main Solana ARB Scout repository.

Wallet Lab must not modify or deploy into Scout's workspace, CLI, CI, execution
engine, Orca/Meteora progression, or production infrastructure without an
explicit future integration decision.

## Current state

Scout Wallet Lab is **Devnet-only**.

Implemented:

- encrypted vault storage
- wallet generation/import
- lock/unlock boundaries
- zeroized secret handling
- public-key derivation
- canonical Solana message construction
- bounded transaction signing
- read-only Devnet balance inspection
- transaction lifecycle ledger
- ambiguous-transaction quarantine
- fresh blockhash resolution
- prepared transaction/ledger binding
- emergency signer lock
- execution-policy authorization
- safe observability model/exporter
- static dashboard
- Vercel static deployment
- exact Rust 1.80.0 CI

Not enabled:

- mainnet
- production funds
- transaction submission
- browser signing
- treasury movement
- remote signing controls

## Security architecture

Scout owns transaction authorization. Strategies never own private keys.

```text
SCOUT STRATEGIES
      |
      v
POLICY RING
      |
      v
SIGNER RING
      |
      v
VAULT RING
