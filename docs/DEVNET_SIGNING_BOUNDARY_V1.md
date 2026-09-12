# Scout Wallet Lab — Devnet Signing Boundary v1

Status: Stage C merged and certified; Stage D physical-proof harness prepared for operator review

This document defines the Scout Wallet Lab signing boundary without opening transaction submission, Mainnet, remote signing, browser signing, or production-funds movement.

## Locked boundary

- Wallet Lab remains Devnet-only.
- `sendTransaction` remains forbidden.
- Mainnet RPC remains forbidden.
- Raw seed/private-key/keypair export remains forbidden.
- Browser/Vercel remains outside the signing trust boundary.
- Arbitrary-message, arbitrary-byte, and generic transaction-signing APIs remain forbidden.
- Passphrases are accepted only for explicit local vault operations and must not be stored as plaintext.
- Recovery, re-key, signing, and submission remain separate operator gates.

## Existing trusted primitives

The Rust wallet engine contains the internal pieces required for a controlled signer:

- encrypted `LockedVault` storage;
- passphrase-gated `LockedVault::unlock`;
- canonical Solana transaction-message construction;
- fresh Devnet blockhash leasing;
- transaction reservation and lifecycle state;
- `ExecutionPolicy` authorization;
- `AuthorizedTransactionMessage`;
- `UnlockedWallet::sign_transaction_message`;
- emergency signer lock;
- transaction-state rules that prevent unsafe retry/re-sign behavior;
- Rust-only `DevnetSigningCoordinator` composing those gates.

These primitives do not authorize arbitrary signing or transaction submission.

## v1 trust boundary

A Devnet signing request must pass all gates below before the native signer may be called:

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

Devnet Signing Boundary v1 may produce a signature for a policy-authorized canonical transaction message. It must not submit that transaction to Solana.

`sendTransaction` remains a security-tripwire failure until a separate Devnet submission gate is explicitly designed, reviewed, approved, implemented, and tested.

## Android/JNI staging sequence

### Stage A — design lock

Completed.

- no Android signing method;
- no JNI signing export;
- no transaction submission;
- security tripwires explicitly enforced those absences.

### Stage B — native signing coordinator

Completed and certified.

Rust-only coordinator composes the existing vault, prepared-transaction, execution-policy, blockhash, signer-lock, and ledger primitives without exposing arbitrary signing.

### Stage C — Android request boundary

Completed, merged, and double-green certified.

The Android/JNI boundary exposes only one fixed Devnet proof request:

- `signStageCDevnetProof(vaultJson, passphraseBytes)`;
- no arbitrary message bytes;
- no generic `signTransaction(bytes)` surface;
- no raw seed, private key, keypair, decrypted vault contents, or signer handle crosses JNI;
- no transaction submission path is introduced;
- Mainnet remains structurally unavailable.

The fixed proof payload is:

`scout-stage-c-devnet-signing-proof-v1`

The fixed reserved exposure is one lamport.

### Credential recovery and re-key safety work

Completed as separate local-only tooling before Stage D physical execution.

- candidate passphrase verification is read-only;
- emergency recovery-word backup is explicitly gated;
- passphrase re-key preserves the existing signing seed and public identity;
- the replacement vault is verified before and after storage;
- rollback is attempted if post-write verification fails;
- re-key performs no signing, transaction creation, transaction submission, or Mainnet operation.

Availability of the re-key tooling does not prove that the operator has physically completed a re-key. That remains a local-device fact.

### Stage D — physical Devnet signing proof

Harness prepared for review. Physical execution remains an explicit operator gate.

The Stage D harness must:

- remain `android:exported="false"`;
- be reached only through an explicit Scout operator navigation surface;
- display `DEVNET ONLY`, transaction-submission-disabled, Mainnet-disabled, and arbitrary-signing-disabled state;
- require an explicit local authorization acknowledgement;
- require current wallet passphrase entry and confirmation;
- use `PassphrasePolicy.encodeCandidateForVerification` so a valid legacy passphrase is not incorrectly rejected by wallet-creation minimum-length rules;
- clear UI passphrase fields before the native signing call;
- wipe passphrase bytes after the native call;
- invoke only `NativeBridge.signStageCDevnetProof`;
- verify the returned wallet address equals the stored wallet identity;
- require a 128-character lowercase hexadecimal signature;
- require a non-empty fresh recent blockhash;
- require the fixed reserved-lamports value of `1`;
- display only public proof metadata;
- provide no clipboard path for proof output;
- never create, replace, re-key, restore, or export the wallet;
- never submit a transaction.

For this temporary physical-proof harness, normal Scout startup remains the credential-recovery launcher so the existing automatic update behavior is preserved. A separate `Scout Stage D` launcher opens a neutral operator hub, which then uses an explicit same-app intent to the non-exported Stage D proof activity. The proof activity itself is not externally launchable.

The first physical signature is not authorized by merely merging the harness. The operator must still explicitly authorize the proof locally on the device.

### Stage E — Devnet submission design

Separate future milestone. Not authorized by this document.

## Required tests and tripwires

CI must continue to prove:

- exact Rust 1.80 formatting;
- Clippy with warnings denied;
- Rust tests;
- security tripwires;
- Android build and APK signature verification when Android code changes;
- no Mainnet RPC endpoint;
- no `sendTransaction` source path;
- no generic Android/JNI transaction signing;
- no arbitrary-message or arbitrary-byte signing;
- Stage D calls only the fixed Stage C proof request;
- Stage D remains non-exported;
- the normal credential-recovery launcher remains intact for updater compatibility.

No merge is allowed on a red or ambiguous run.

## Manual control gates

PJ remains the final authority at the following points:

- merging signing-boundary PRs into `main`;
- performing the first physical Devnet signature;
- enabling any transaction submission capability;
- introducing Mainnet capability;
- handling real funds.

## Current next exact action

Certify the modernized Stage D physical-proof harness under Rust 1.80 and Android CI. If both lanes are green and the diff preserves every fail-closed boundary, present the PR for PJ's manual merge. After merge and app update, the next operator action is the explicit local physical Devnet proof; transaction submission remains disabled.
