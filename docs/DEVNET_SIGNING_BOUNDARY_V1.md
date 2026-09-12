# Scout Wallet Lab — Devnet Signing Boundary v1

Status: Stage D physical signing proof passed; Stage E physical simulation-only preflight passed; Stage F submission remains design-only and not authorized

This document defines the Scout Wallet Lab signing and pre-submission boundary without opening Mainnet, remote signing, browser signing, arbitrary signing, or production-funds movement.

## Locked boundary

- Wallet Lab remains Devnet-only.
- `sendTransaction` remains forbidden in executable source until PJ explicitly opens the separate Stage F implementation gate.
- Mainnet RPC remains forbidden.
- Raw seed/private-key/keypair export remains forbidden.
- Browser/Vercel remains outside the signing trust boundary.
- Arbitrary-message, arbitrary-byte, and generic transaction-signing APIs remain forbidden.
- Passphrases are accepted only for explicit local vault operations and must not be stored as plaintext.
- Recovery, re-key, signing, simulation, and submission remain separate operator gates.
- A successful simulation never authorizes submission by itself.

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
- fixed Stage E fee preflight and signature-verifying Devnet simulation.

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

## Separation of signing, simulation, and submission

Signing, simulation, and transaction submission are separate milestones.

Devnet Signing Boundary v1 may produce a signature for a policy-authorized canonical transaction message. Stage E may also send that exact signed transaction to Devnet `simulateTransaction` with signature verification enabled.

Neither action authorizes broadcasting the transaction for inclusion in the ledger.

`sendTransaction` remains a security-tripwire failure in executable source until a separate Devnet submission gate is explicitly designed, reviewed, approved, implemented, and physically authorized.

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

### Stage F — Devnet submission gate design

Design phase may proceed. Implementation and physical submission are not authorized by this document.

The first possible Stage F transaction, if separately approved by PJ, must remain narrower than a general wallet send path. The implementation contract is:

1. Devnet only; the RPC cluster and endpoint must be structurally verified before authorization.
2. Exactly one fixed Memo program instruction with a fixed Stage F proof payload defined in source.
3. No recipient account, token transfer, system transfer, swap, program-selected accounts, or operator-supplied transaction bytes.
4. Exactly one required signer and the stored Scout wallet must be the payer.
5. Fresh blockhash lease resolved immediately before fee calculation and authorization.
6. Exact fee obtained from Devnet and bounded by a small fixed cap in source.
7. Sufficient Devnet balance verified before signing, with a conservative remaining-balance floor.
8. The exact candidate must pass signature-verifying simulation before any submission can become eligible.
9. Simulation result must be clean and tied to the same signed bytes intended for submission.
10. A second, separate local authorization acknowledgement must be required after simulation and before submission.
11. Current wallet passphrase must be entered locally for the Stage F operation and wiped after use.
12. The signed wire transaction must not cross into browser/Vercel code, logs, analytics, clipboard, files, intents, or persistent app storage.
13. Submission must occur at most once for that signed candidate. No automatic retry, rebroadcast, replacement, or re-sign loop is allowed.
14. Only the resulting public signature and public confirmation metadata may be displayed.
15. Mainnet remains structurally unavailable.
16. Generic signing, arbitrary transaction submission, token transfer, and account-selectable send surfaces remain forbidden.
17. The global executable-source tripwire against `sendTransaction` must remain active until PJ explicitly authorizes Stage F implementation in a separate operator decision.

Before implementation is opened, Stage F must also define a fail-closed lifecycle for at least these outcomes:

- preflight fee outside boundary;
- insufficient Devnet balance;
- stale blockhash;
- simulation failure;
- operator cancellation;
- submission transport failure with ambiguous delivery;
- RPC rejection;
- returned signature mismatch or malformed response;
- confirmation timeout;
- confirmed success.

Ambiguous submission transport failure is terminal for the candidate: Scout must not automatically submit the same signed bytes again and must not automatically create a replacement transaction.

## Required tests and tripwires

CI must continue to prove:

- exact Rust 1.80 formatting;
- Clippy with warnings denied;
- Rust tests;
- security tripwires;
- Android build and APK signature verification when Android code changes;
- no Mainnet RPC endpoint;
- no `sendTransaction` executable source path before explicit Stage F implementation authorization;
- no generic Android/JNI transaction signing;
- no arbitrary-message or arbitrary-byte signing;
- Stage D calls only the fixed Stage C proof request;
- Stage D remains non-exported;
- Stage E calls only its fixed simulation request;
- Stage E remains non-exported;
- Stage E simulation uses signature verification and never substitutes a fresh blockhash server-side;
- the normal credential-recovery launcher remains intact for updater compatibility.

No merge is allowed on a red or ambiguous run.

## Manual control gates

PJ remains the final authority at the following points:

- merging signing-boundary PRs into `main`;
- enabling any transaction submission capability;
- physically authorizing the first Devnet ledger submission;
- introducing Mainnet capability;
- handling real funds.

Stage D and Stage E physical passes do not transfer or weaken those authorities.

## Current next exact action

Review and certify this Stage F design-only boundary update. It intentionally adds no submission code and leaves the executable-source `sendTransaction` tripwire intact. After a green design PR is merged, Stage F implementation must remain blocked until PJ separately and explicitly authorizes opening the Devnet submission capability.
