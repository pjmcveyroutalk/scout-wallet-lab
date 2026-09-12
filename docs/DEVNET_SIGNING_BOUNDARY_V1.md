# Scout Wallet Lab — Devnet Signing Boundary v1

Status: design-stage only

This document defines the next Scout Wallet Lab signing milestone without opening transaction submission, Mainnet, remote signing, browser signing, or production-funds movement.

## Locked starting point

- Wallet Lab remains Devnet-only.
- `bridgeStatus` remains `wallet-operations-locked`.
- Android/JNI exposes no signing operation.
- `sendTransaction` remains forbidden.
- Mainnet RPC remains forbidden.
- Raw seed/private-key/keypair export remains forbidden.
- Browser/Vercel remains outside the signing trust boundary.
- Recovery-word Android credential crossing remains deferred and is not part of this phase.

## Existing trusted primitives

The Rust wallet engine already contains the internal pieces required to design a controlled signer:

- encrypted `LockedVault` storage;
- passphrase-gated `LockedVault::unlock`;
- canonical Solana transaction-message construction;
- fresh Devnet blockhash leasing;
- transaction reservation and lifecycle state;
- `ExecutionPolicy` authorization;
- `AuthorizedTransactionMessage`;
- `UnlockedWallet::sign_transaction_message`;
- emergency signer lock;
- transaction-state rules that prevent unsafe retry/re-sign behavior.

These primitives do not, by themselves, authorize Android or any external caller to request a signature.

## v1 trust boundary

A future Devnet signing request must pass all gates below before the native signer may be called:

1. The stored encrypted vault must parse and identify the expected wallet.
2. The operator must supply the wallet passphrase locally.
3. The wallet must unlock inside native Rust.
4. The transaction must already be a canonical `PreparedTransaction`.
5. The transaction ledger state must be `Reserved`.
6. A fresh blockhash lease must still be valid at authorization time.
7. Reserved lamports must be at or below the explicit Devnet exposure limit.
8. Every transaction program must be on the explicit allowlist.
9. `ExecutionPolicy::authorize` must produce the only signable transaction-message capability.
10. The wallet signer must not be emergency-locked.
11. The signature must bind only to that exact authorized canonical message.
12. The transaction lifecycle must transition from `Reserved` to `Signed` exactly once.

No API may accept arbitrary bytes for signing.

## Separation of signing and submission

Signing and transaction submission are separate milestones.

Devnet Signing Boundary v1 may eventually produce a signature for a policy-authorized canonical transaction message. It must not submit that transaction to Solana.

`sendTransaction` remains a security-tripwire failure until a separate Devnet submission gate is explicitly designed, reviewed, approved, implemented, and tested.

## Android/JNI staging sequence

The Android boundary must be introduced in stages:

### Stage A — design lock

Current stage.

- no Android signing method;
- no JNI signing export;
- no transaction submission;
- security tripwires explicitly enforce those absences.

### Stage B — native signing coordinator

Rust-only coordinator. It must compose the existing vault, prepared-transaction, execution-policy, blockhash, signer-lock, and ledger primitives without exposing arbitrary signing.

Required properties:

- Devnet-only by type/API design;
- passphrase consumed locally and zeroized where possible;
- no raw seed/private key returned;
- no generic message-signing endpoint;
- no network submission;
- structured non-secret result metadata only;
- deterministic failure categories that never contain secrets.

### Stage C — Android request boundary

Only after Stage B is double-green and explicitly accepted.

A narrow JNI/Kotlin method may request signing of a fully specified Devnet transaction intent. Android must not provide raw arbitrary message bytes for signing.

Crossing into Stage C is a manual security gate.

### Stage D — physical Devnet signing proof

Only after Stage C is double-green.

Physical proof should use a deliberately bounded Devnet-only transaction and should verify the produced signature/address relationship without enabling network submission in the same milestone.

### Stage E — Devnet submission design

Separate future milestone. Not authorized by this document.

## Required tests before Android exposure

Rust tests must prove:

- wrong passphrase fails before signing;
- malformed vault fails before signing;
- wallet/public-key mismatch fails;
- zero exposure policy is rejected;
- empty allowlist is rejected;
- over-limit reservation is rejected;
- disallowed program is rejected;
- non-reserved transaction is rejected;
- expired blockhash is rejected;
- emergency-locked signer refuses signing;
- only the exact policy-authorized canonical message can be signed;
- successful signing moves the ledger to `Signed` exactly once;
- re-signing the same ledger entry is rejected;
- no submission path is reachable;
- no Mainnet path exists;
- no raw key or arbitrary-signing API is introduced.

## CI contract

Every implementation increment must pass:

- exact Rust 1.80 formatting;
- Clippy with warnings denied;
- Rust tests;
- security tripwires;
- Android CI when Android code is touched.

No merge is allowed on a red or ambiguous run.

## Manual control gates

PJ remains the final authority at the following points:

- merging any signing-boundary PR into `main`;
- exposing signing across JNI/Android;
- performing the first physical Devnet signature;
- enabling any transaction submission capability;
- introducing Mainnet capability;
- handling real funds.

## Current next exact action

Implement Stage B as a Rust-only Devnet signing coordinator on a scoped development branch while the Android/JNI signing tripwires remain locked.
