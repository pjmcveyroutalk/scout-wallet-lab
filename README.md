# Scout Wallet Lab

Isolated security lab for Scout-native wallet, signing, policy, ledger, Devnet access, observability, and execution controls.

This repository is separate from the main Solana ARB Scout repository and must not modify Scout core systems without an explicit future integration decision.

## Current state

Wallet Lab is Devnet-only.

Implemented: encrypted vault storage, key generation/import, lock/unlock, zeroized secret handling, public address derivation, canonical Solana message signing, read-only Devnet RPC, transaction lifecycle ledger, ambiguous-transaction quarantine, fresh blockhash resolution, prepared transaction binding, emergency signer lock, execution-policy authorization, safe observability, dashboard, Vercel static hosting, Rust 1.80 CI, and security tripwires.

Disabled: mainnet, production funds, transaction submission, browser signing, remote signing, and treasury movement.

## Security invariants

Scout owns transaction authorization. Strategies never own private keys.

UnlockedWallet is the signing authority. Production signing must use policy-authorized canonical transaction messages.

The browser and Vercel are outside the signing trust boundary. They must never receive seed material, private keys, decrypted keypairs, signing secrets, or wallet passphrases.

TX-LIFE-001: never rebuild or re-sign an unresolved execution while its original blockhash remains valid.

TX-RETRY-001: retry delivery before retrying economic intent. A second signature is forbidden until the first transaction is provably terminal or safely resolved.

Submitted or ambiguous capital remains reserved until outcome certainty exists.

Observability exposes only safe public metadata. Generated wallet-observability snapshots are not committed.

Rust is pinned to 1.80.0. Formatting, Clippy with warnings denied, tests, and security tripwires must pass before progression.

Mainnet promotion is forbidden until the complete wallet, policy, execution, recovery, and CI chain is explicitly accepted.

Documentation status: complete.
