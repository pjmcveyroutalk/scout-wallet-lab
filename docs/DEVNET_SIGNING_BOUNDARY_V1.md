# Scout Wallet Lab — Devnet Signing Boundary v1

Status: Stage D physical signing proof passed; Stage E physical simulation-only preflight passed; Stage F design merged; Stage F-A presubmit implementation is in review; ledger submission remains disabled and physically unauthorized

This document defines the Scout Wallet Lab signing and pre-submission boundary without opening Mainnet, remote signing, browser signing, arbitrary signing, or production-funds movement.

## Locked boundary

- Wallet Lab remains Devnet-only.
- Stage F implementation work was explicitly opened by PJ on 2026-09-12, but Stage F-A remains presubmit-only and contains no ledger-submission RPC.
- `sendTransaction` remains forbidden in executable source during Stage F-A.
- Stage F-B ledger submission remains a later, separate implementation and physical-authorization gate.
- Mainnet RPC remains forbidden.
- Raw seed/private-key/keypair export remains forbidden.
- Browser/Vercel remains outside the signing trust boundary.
- Arbitrary-message, arbitrary-byte, and generic transaction-signing APIs remain forbidden.
- Passphrases are accepted only for explicit local vault operations and must not be stored as plaintext.
- Recovery, re-key, signing, simulation, presubmit preparation, and ledger submission remain separate operator gates.
- A successful signature, simulation, or presubmit proof never authorizes ledger submission by itself.

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
- Rust-only `DevnetSigningCoordinator` composing those gates;
- fixed Stage E fee preflight and signature-verifying Devnet simulation;
- Stage F-A fixed-candidate fee, balance-floor, simulation, and in-memory candidate controls.

These primitives do not authorize arbitrary signing or ledger submission.

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

## Separation of signing, simulation, presubmit preparation, and submission

Signing, simulation, presubmit preparation, and ledger submission are separate milestones.

Devnet Signing Boundary v1 may produce a signature for a policy-authorized canonical transaction message. Stage E may send that exact signed transaction to Devnet `simulateTransaction` with signature verification enabled. Stage F-A may prepare one fixed signed candidate, run the same exact-bytes simulation discipline, and retain that candidate only in process memory behind a random one-time token until it is explicitly discarded or the process ends.

None of those actions authorizes broadcasting the transaction for inclusion in the ledger.

Stage F-A must not contain `sendTransaction`. Any future Stage F-B broadcast path requires a separate implementation gate and a separate physical operator authorization before the first Devnet ledger submission.

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

Completed as separate local-only tooling before physical signing execution.

- candidate passphrase verification is read-only;
- emergency recovery-word backup is explicitly gated;
- passphrase re-key preserves the existing signing seed and public identity;
- the replacement vault is verified before and after storage;
- rollback is attempted if post-write verification fails;
- re-key performs no signing, transaction creation, transaction submission, or Mainnet operation.

Availability of the re-key tooling does not prove that the operator physically completed every optional recovery action. Those remain local-device facts.

### Stage D — physical Devnet signing proof

Completed on the physical Scout Operator device. PJ reported the Stage D proof passed on 2026-09-12.

The Stage D harness remains constrained to:

- `android:exported="false"`;
- explicit Scout operator navigation only;
- `DEVNET ONLY`, transaction-submission-disabled, Mainnet-disabled, and arbitrary-signing-disabled state;
- explicit local authorization acknowledgement;
- current wallet passphrase entry and confirmation;
- `PassphrasePolicy.encodeCandidateForVerification` so a valid legacy passphrase is not incorrectly rejected by wallet-creation minimum-length rules;
- clearing UI passphrase fields before the native signing call;
- wiping passphrase bytes after the native call;
- invoking only `NativeBridge.signStageCDevnetProof`;
- verifying the returned wallet address equals the stored wallet identity;
- requiring a 128-character lowercase hexadecimal signature;
- requiring a non-empty fresh recent blockhash;
- requiring the fixed reserved-lamports value of `1`;
- displaying only public proof metadata;
- no clipboard path for proof output;
- no wallet creation, replacement, re-key, restore, or recovery export;
- no transaction submission.

### Stage E — physical Devnet simulation-only preflight

Completed on the physical Scout Operator device. PJ reported the Stage E preflight passed on 2026-09-12.

Stage E remains a fixed, non-submitting proof path:

- fixed Memo program only;
- fixed payload `scout-stage-e-devnet-simulation-proof-v1`;
- fresh Devnet blockhash required;
- `getFeeForMessage` used before signing;
- fee must be greater than zero and no more than 10,000 lamports;
- exactly one required signer and the stored Scout wallet must be the payer;
- the fixed candidate is signed locally through the native coordinator;
- the exact signed wire transaction is sent only to `simulateTransaction`;
- simulation uses `sigVerify: true`;
- simulation does not replace the recent blockhash;
- simulation must return no execution error and must return units consumed;
- Android verifies the returned public wallet identity and bounded public metadata;
- no transaction submission path exists.

A Stage E pass proves the fixed signed candidate can clear the local policy gates and Devnet simulation. It does not prove or authorize ledger submission.

### Stage F-A — Devnet presubmit proof

Implementation is in review. Physical Stage F-A execution has not yet occurred.

Stage F-A is intentionally narrower than a ledger-submission path:

1. Devnet only; Android verifies the native identity, cluster, and exact Devnet endpoint before enabling the gate.
2. Exactly one fixed Memo program instruction with payload `scout-stage-f-devnet-submission-proof-v1`.
3. No recipient account, token transfer, system transfer, swap, program-selected accounts, or operator-supplied transaction bytes.
4. Exactly one required signer and the stored Scout wallet as payer.
5. A fresh blockhash lease is resolved before fee calculation and signing.
6. `getFeeForMessage` obtains the exact fee, which must be greater than zero and no more than 10,000 lamports.
7. Current Devnet balance is fetched and the candidate is rejected unless at least 1,000,000 lamports remain after the fee.
8. The exact candidate is signed locally through the existing policy-authorized coordinator.
9. The exact signed wire bytes are sent only to `simulateTransaction` with `sigVerify: true` and `replaceRecentBlockhash: false`.
10. Simulation must complete without execution error and return units consumed.
11. The signed wire candidate is held only in zeroizing process memory behind a random 16-byte one-time token.
12. Only one prepared candidate may exist in the process at a time.
13. Discard requires the exact candidate token; leaving or destroying the Stage F-A screen attempts discard.
14. The token is not displayed to the operator and is not persisted, copied, shared, logged, placed in an intent, or sent to browser/Vercel code.
15. The Stage F-A activity is non-exported and reachable only through explicit Scout operator navigation.
16. Mainnet, generic signing, wallet creation/re-key/recovery export, and ledger submission remain unavailable from the Stage F-A surface.
17. `sendTransaction` remains absent from Stage F-A executable source.

A physical Stage F-A pass will prove only that Scout can prepare, sign, simulate, hold, and discard the exact bounded candidate. It will not submit anything to the ledger.

### Stage F-B — first Devnet ledger-submission gate

Not implemented and not physically authorized.

If separately opened after a successful Stage F-A physical proof, Stage F-B must preserve the exact signed candidate and fail closed across the complete delivery lifecycle. At minimum:

- a separate local post-simulation authorization is required before the first delivery attempt;
- the exact prepared candidate must be used without replacement or re-signing;
- submission is attempted at most once for that candidate;
- no automatic retry or rebroadcast occurs after an ambiguous transport outcome;
- stale blockhash, RPC rejection, returned-signature mismatch, confirmation timeout, ambiguous delivery, and confirmed success remain distinct states;
- ambiguous delivery is terminal for automatic action and requires observation/reconciliation rather than another send;
- only public transaction signature and confirmation metadata may be displayed;
- Mainnet remains structurally unavailable.

## Required tests and tripwires

CI must continue to prove:

- exact Rust 1.80 formatting;
- Clippy with warnings denied;
- Rust tests;
- security tripwires;
- Android build and APK signature verification when Android code changes;
- no Mainnet RPC endpoint;
- no Stage F-A `sendTransaction` executable path;
- no generic Android/JNI transaction signing;
- no arbitrary-message or arbitrary-byte signing;
- Stage D calls only the fixed Stage C proof request;
- Stage D remains non-exported;
- Stage E calls only its fixed simulation request;
- Stage E remains non-exported;
- Stage E simulation uses signature verification and never substitutes a fresh blockhash server-side;
- Stage F-A fixed payload, fee ceiling, balance floor, exact-bytes signature-verifying simulation, zeroizing in-memory candidate, one-time token binding, and discard path remain present;
- Stage F-A has only narrow prepare/discard JNI requests;
- Stage F-A remains non-exported;
- the normal credential-recovery launcher remains intact for updater compatibility.

No merge is allowed on a red or ambiguous run.

## Manual control gates

PJ remains the final authority at the following points:

- merging signing-boundary PRs into `main`;
- physically executing the Stage F-A presubmit proof;
- opening Stage F-B ledger-submission implementation;
- physically authorizing the first Devnet ledger submission;
- introducing Mainnet capability;
- handling real funds.

Stage D and Stage E physical passes, Stage F implementation authorization, and any later Stage F-A pass do not transfer or weaken those authorities.

## Current next exact action

Complete and certify the Stage F-A presubmit harness under the dedicated Stage F boundary audit, Rust 1.80 CI, and Android CI. Keep the PR draft until all three lanes are green and the final diff contains no ledger-submission or Mainnet path. Then present the PR for PJ's manual merge. After merge and app update, the next operator action is the explicit physical Stage F-A presubmit proof followed by explicit candidate discard. Stage F-B remains closed until a later separate operator decision.
