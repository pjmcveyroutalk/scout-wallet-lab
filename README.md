# Scout Wallet Lab

Isolated development and security lab for Scout-native wallet, signing, policy,
ledger, devnet account access, observability, and execution controls.

## Isolation contract

This repository is independent from the main Solana ARB Scout repository.

Nothing in this lab may modify, depend on, or deploy into the main Scout build
without an explicit future integration decision.

Wallet Lab development must not change the main Scout workspace, CLI, CI,
Orca/Meteora progression, execution engine, or deployment pipeline.

## Current safety state

Scout Wallet Lab is still **Devnet-only**.

Mainnet funds, mainnet execution, transaction submission, browser signing, and
production treasury integration remain disabled.

The current system includes:

- encrypted wallet vault storage
- wallet generation and import
- lock and unlock boundaries
- zeroized secret wrappers
- public-key derivation
- bounded transaction-message signing
- read-only Devnet account balance inspection
- canonical Solana message construction
- transaction lifecycle ledger
- ambiguous-submission quarantine rules
- fresh blockhash resolution
- prepared transaction and ledger binding
- emergency signer lock
- execution policy authorization
- Devnet-only observability model
- safe observability exporter
- static dashboard
- Vercel static deployment
- exact Rust 1.80.0 CI verification

## Security architecture

Scout owns transaction authorization.

Strategies never own private keys.

```text
SCOUT STRATEGIES
      |
      | propose execution intent
      v
POLICY RING
      |
      | authorized transaction message
      v
SIGNER RING
      |
      | signature
      v
VAULT RING
