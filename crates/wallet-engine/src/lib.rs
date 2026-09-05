#![forbid(unsafe_code)]

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::{Signer as _, SigningKey};
use rustls::crypto::CryptoProvider;
use serde::{Deserialize, Serialize};
use solana_hash::Hash;
use solana_instruction::Instruction;
use solana_message::Message;
use solana_pubkey::Pubkey;
use std::{fmt, str::FromStr, time::Duration};
use zeroize::{Zeroize, Zeroizing};

const VAULT_VERSION: u8 = 1;
const AAD: &[u8] = b"scout-wallet-v1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const SEED_LEN: usize = 32;
const KEY_LEN: usize = 32;
const SIGNATURE_LEN: usize = 64;
const AEAD_TAG_LEN: usize = 16;
const CIPHERTEXT_LEN: usize = SEED_LEN + AEAD_TAG_LEN;

const ARGON2_MEMORY_KIB: u32 = 65_536;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_LANES: u32 = 1;

const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";
const RPC_TIMEOUT_SECONDS: u64 = 10;
const RPC_REQUEST_ID: u64 = 1;

#[must_use]
pub const fn engine_name() -> &'static str {
    "scout-wallet-lab"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cluster {
    Devnet,
}

impl Cluster {
    #[must_use]
    pub const fn rpc_name(self) -> &'static str {
        match self {
            Self::Devnet => "devnet",
        }
    }

    #[must_use]
    pub const fn rpc_url(self) -> &'static str {
        match self {
            Self::Devnet => DEVNET_RPC_URL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevnetAccount {
    address: Pubkey,
    lamports: Option<u64>,
}

impl DevnetAccount {
    #[must_use]
    pub const fn new(address: Pubkey) -> Self {
        Self {
            address,
            lamports: None,
        }
    }

    #[must_use]
    pub const fn cluster(&self) -> Cluster {
        Cluster::Devnet
    }

    #[must_use]
    pub const fn address(&self) -> Pubkey {
        self.address
    }

    #[must_use]
    pub const fn lamports(&self) -> Option<u64> {
        self.lamports
    }

    pub fn record_balance(&mut self, lamports: u64) {
        self.lamports = Some(lamports);
    }

    pub fn clear_balance(&mut self) {
        self.lamports = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockhashLease {
    recent_blockhash: Hash,
    last_valid_block_height: u64,
    observed_block_height: u64,
}

impl BlockhashLease {
    #[cfg(test)]
    fn new_for_test(
        recent_blockhash: Hash,
        last_valid_block_height: u64,
        observed_block_height: u64,
    ) -> Self {
        Self {
            recent_blockhash,
            last_valid_block_height,
            observed_block_height,
        }
    }

    #[must_use]
    pub const fn recent_blockhash(&self) -> Hash {
        self.recent_blockhash
    }

    #[must_use]
    pub const fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    #[must_use]
    pub const fn observed_block_height(&self) -> u64 {
        self.observed_block_height
    }

    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.observed_block_height <= self.last_valid_block_height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcError {
    TlsInitializationFailed,
    ClientInitializationFailed,
    TransportFailed,
    HttpStatusFailed,
    InvalidResponse,
    RpcRejected,
    BlockhashExpired,
}

impl fmt::Display for RpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TlsInitializationFailed => "RPC TLS initialization failed",
            Self::ClientInitializationFailed => "RPC client initialization failed",
            Self::TransportFailed => "RPC transport failed",
            Self::HttpStatusFailed => "RPC HTTP status rejected",
            Self::InvalidResponse => "RPC response was invalid",
            Self::RpcRejected => "RPC request was rejected",
            Self::BlockhashExpired => "RPC returned an already-expired blockhash",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RpcError {}

pub struct DevnetRpc {
    client: reqwest::Client,
}

#[derive(Serialize)]
struct GetBalanceRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: (String, RpcCommitment),
}

#[derive(Serialize)]
struct GetLatestBlockhashRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: (RpcCommitment,),
}

#[derive(Serialize)]
struct GetBlockHeightRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: (RpcCommitment,),
}

#[derive(Serialize)]
struct RpcCommitment {
    commitment: &'static str,
}

#[derive(Deserialize)]
struct GetBalanceResponse {
    result: Option<GetBalanceResult>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetBalanceResult {
    value: u64,
}

#[derive(Deserialize)]
struct GetLatestBlockhashResponse {
    result: Option<GetLatestBlockhashResult>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetLatestBlockhashResult {
    value: GetLatestBlockhashValue,
}

#[derive(Deserialize)]
struct GetLatestBlockhashValue {
    blockhash: String,
    #[serde(rename = "lastValidBlockHeight")]
    last_valid_block_height: u64,
}

#[derive(Deserialize)]
struct GetBlockHeightResponse {
    result: Option<u64>,
    error: Option<serde_json::Value>,
}

impl DevnetRpc {
    pub fn new() -> Result<Self, RpcError> {
        ensure_tls_provider()?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(RPC_TIMEOUT_SECONDS))
            .build()
            .map_err(|_| RpcError::ClientInitializationFailed)?;

        Ok(Self { client })
    }

    pub async fn get_balance(&self, address: Pubkey) -> Result<u64, RpcError> {
        let request = build_get_balance_request(address);

        let response = self
            .client
            .post(Cluster::Devnet.rpc_url())
            .json(&request)
            .send()
            .await
            .map_err(|_| RpcError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(RpcError::HttpStatusFailed);
        }

        let response = response
            .json::<GetBalanceResponse>()
            .await
            .map_err(|_| RpcError::InvalidResponse)?;

        if response.error.is_some() {
            return Err(RpcError::RpcRejected);
        }

        response
            .result
            .map(|result| result.value)
            .ok_or(RpcError::InvalidResponse)
    }

    pub async fn refresh_balance(&self, account: &mut DevnetAccount) -> Result<u64, RpcError> {
        account.clear_balance();

        let result = self.get_balance(account.address()).await;

        match result {
            Ok(lamports) => {
                account.record_balance(lamports);
                Ok(lamports)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn get_latest_blockhash(&self) -> Result<(Hash, u64), RpcError> {
        let request = build_get_latest_blockhash_request();

        let response = self
            .client
            .post(Cluster::Devnet.rpc_url())
            .json(&request)
            .send()
            .await
            .map_err(|_| RpcError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(RpcError::HttpStatusFailed);
        }

        let response = response
            .json::<GetLatestBlockhashResponse>()
            .await
            .map_err(|_| RpcError::InvalidResponse)?;

        parse_latest_blockhash_response(response)
    }

    pub async fn get_block_height(&self) -> Result<u64, RpcError> {
        let request = build_get_block_height_request();

        let response = self
            .client
            .post(Cluster::Devnet.rpc_url())
            .json(&request)
            .send()
            .await
            .map_err(|_| RpcError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(RpcError::HttpStatusFailed);
        }

        let response = response
            .json::<GetBlockHeightResponse>()
            .await
            .map_err(|_| RpcError::InvalidResponse)?;

        parse_block_height_response(response)
    }

    pub async fn resolve_fresh_blockhash(&self) -> Result<BlockhashLease, RpcError> {
        let (recent_blockhash, last_valid_block_height) = self.get_latest_blockhash().await?;
        let observed_block_height = self.get_block_height().await?;

        build_blockhash_lease(
            recent_blockhash,
            last_valid_block_height,
            observed_block_height,
        )
    }
}

fn ensure_tls_provider() -> Result<(), RpcError> {
    if CryptoProvider::get_default().is_some() {
        return Ok(());
    }

    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| RpcError::TlsInitializationFailed)
}

fn build_get_balance_request(address: Pubkey) -> GetBalanceRequest {
    GetBalanceRequest {
        jsonrpc: "2.0",
        id: RPC_REQUEST_ID,
        method: "getBalance",
        params: (
            address.to_string(),
            RpcCommitment {
                commitment: "confirmed",
            },
        ),
    }
}

fn build_get_latest_blockhash_request() -> GetLatestBlockhashRequest {
    GetLatestBlockhashRequest {
        jsonrpc: "2.0",
        id: RPC_REQUEST_ID,
        method: "getLatestBlockhash",
        params: (RpcCommitment {
            commitment: "confirmed",
        },),
    }
}

fn build_get_block_height_request() -> GetBlockHeightRequest {
    GetBlockHeightRequest {
        jsonrpc: "2.0",
        id: RPC_REQUEST_ID,
        method: "getBlockHeight",
        params: (RpcCommitment {
            commitment: "confirmed",
        },),
    }
}

fn parse_latest_blockhash_response(
    response: GetLatestBlockhashResponse,
) -> Result<(Hash, u64), RpcError> {
    if response.error.is_some() {
        return Err(RpcError::RpcRejected);
    }

    let value = response
        .result
        .map(|result| result.value)
        .ok_or(RpcError::InvalidResponse)?;

    let recent_blockhash =
        Hash::from_str(&value.blockhash).map_err(|_| RpcError::InvalidResponse)?;

    Ok((recent_blockhash, value.last_valid_block_height))
}

fn parse_block_height_response(response: GetBlockHeightResponse) -> Result<u64, RpcError> {
    if response.error.is_some() {
        return Err(RpcError::RpcRejected);
    }

    response.result.ok_or(RpcError::InvalidResponse)
}

fn build_blockhash_lease(
    recent_blockhash: Hash,
    last_valid_block_height: u64,
    observed_block_height: u64,
) -> Result<BlockhashLease, RpcError> {
    if observed_block_height > last_valid_block_height {
        return Err(RpcError::BlockhashExpired);
    }

    Ok(BlockhashLease {
        recent_blockhash,
        last_valid_block_height,
        observed_block_height,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMessageError {
    EmptyInstructions,
    SerializationFailed,
}

impl fmt::Display for TransactionMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyInstructions => "transaction message must contain at least one instruction",
            Self::SerializationFailed => "transaction message serialization failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TransactionMessageError {}

pub struct CanonicalTransactionMessage {
    bytes: Vec<u8>,
}

impl CanonicalTransactionMessage {
    pub fn new(
        instructions: &[Instruction],
        payer: Pubkey,
        recent_blockhash: Hash,
    ) -> Result<Self, TransactionMessageError> {
        if instructions.is_empty() {
            return Err(TransactionMessageError::EmptyInstructions);
        }

        let message = Message::new_with_blockhash(instructions, Some(&payer), &recent_blockhash);
        let bytes = bincode::serialize(&message)
            .map_err(|_| TransactionMessageError::SerializationFailed)?;

        Ok(Self { bytes })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

pub struct AuthorizedTransactionMessage<'a> {
    bytes: &'a [u8],
}

impl<'a> AuthorizedTransactionMessage<'a> {
    #[cfg(test)]
    fn new(message: &'a CanonicalTransactionMessage) -> Self {
        Self {
            bytes: message.bytes(),
        }
    }

    fn as_bytes(&self) -> &[u8] {
        self.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    ZeroExposureLimit,
    NoAllowedPrograms,
    ExposureExceeded,
    ProgramNotAllowed,
    TransactionNotReserved,
    BlockhashExpired,
    InvalidMessage,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroExposureLimit => "execution policy exposure limit must be greater than zero",
            Self::NoAllowedPrograms => "execution policy requires at least one allowed program",
            Self::ExposureExceeded => "transaction reservation exceeds execution policy",
            Self::ProgramNotAllowed => "transaction contains a program not allowed by policy",
            Self::TransactionNotReserved => "only reserved transactions may be authorized",
            Self::BlockhashExpired => "transaction blockhash has expired",
            Self::InvalidMessage => "transaction message could not be validated by policy",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PolicyError {}

pub struct ExecutionPolicy {
    max_reserved_lamports: u64,
    allowed_programs: Vec<Pubkey>,
}

impl ExecutionPolicy {
    pub fn new(
        max_reserved_lamports: u64,
        allowed_programs: &[Pubkey],
    ) -> Result<Self, PolicyError> {
        if max_reserved_lamports == 0 {
            return Err(PolicyError::ZeroExposureLimit);
        }

        if allowed_programs.is_empty() {
            return Err(PolicyError::NoAllowedPrograms);
        }

        Ok(Self {
            max_reserved_lamports,
            allowed_programs: allowed_programs.to_vec(),
        })
    }

    #[must_use]
    pub const fn max_reserved_lamports(&self) -> u64 {
        self.max_reserved_lamports
    }

    #[must_use]
    pub fn allowed_programs(&self) -> &[Pubkey] {
        self.allowed_programs.as_slice()
    }

    pub fn authorize<'a>(
        &self,
        transaction: &'a PreparedTransaction,
        current_block_height: u64,
    ) -> Result<AuthorizedTransactionMessage<'a>, PolicyError> {
        if transaction.ledger().state() != TransactionState::Reserved {
            return Err(PolicyError::TransactionNotReserved);
        }

        if transaction.ledger().reserved_lamports() > self.max_reserved_lamports {
            return Err(PolicyError::ExposureExceeded);
        }

        if current_block_height > transaction.ledger().last_valid_block_height() {
            return Err(PolicyError::BlockhashExpired);
        }

        let message: Message = bincode::deserialize(transaction.message().bytes())
            .map_err(|_| PolicyError::InvalidMessage)?;

        for instruction in &message.instructions {
            let program_index = usize::from(instruction.program_id_index);
            let program_id = message
                .account_keys
                .get(program_index)
                .ok_or(PolicyError::InvalidMessage)?;

            if !self.allowed_programs.contains(program_id) {
                return Err(PolicyError::ProgramNotAllowed);
            }
        }

        Ok(AuthorizedTransactionMessage {
            bytes: transaction.message().bytes(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedTransactionError {
    Message(TransactionMessageError),
    Ledger(LedgerError),
}

impl fmt::Display for PreparedTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(error) => error.fmt(formatter),
            Self::Ledger(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PreparedTransactionError {}

pub struct PreparedTransaction {
    message: CanonicalTransactionMessage,
    ledger: TransactionLedgerEntry,
}

impl PreparedTransaction {
    pub fn reserve(
        instructions: &[Instruction],
        payer: Pubkey,
        reserved_lamports: u64,
        lease: BlockhashLease,
    ) -> Result<Self, PreparedTransactionError> {
        let recent_blockhash = lease.recent_blockhash();
        let message = CanonicalTransactionMessage::new(instructions, payer, recent_blockhash)
            .map_err(PreparedTransactionError::Message)?;
        let ledger = TransactionLedgerEntry::reserve(reserved_lamports, lease)
            .map_err(PreparedTransactionError::Ledger)?;

        Ok(Self { message, ledger })
    }

    #[must_use]
    pub fn message(&self) -> &CanonicalTransactionMessage {
        &self.message
    }

    #[must_use]
    pub const fn ledger(&self) -> &TransactionLedgerEntry {
        &self.ledger
    }

    #[must_use]
    pub fn ledger_mut(&mut self) -> &mut TransactionLedgerEntry {
        &mut self.ledger
    }
}

pub struct SecretPassphrase {
    value: Zeroizing<String>,
}

impl SecretPassphrase {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self {
            value: Zeroizing::new(value),
        }
    }

    fn expose(&self) -> &str {
        self.value.as_str()
    }
}

pub struct SecretSeed {
    value: Zeroizing<[u8; SEED_LEN]>,
}

impl SecretSeed {
    #[must_use]
    pub fn new(value: [u8; SEED_LEN]) -> Self {
        Self {
            value: Zeroizing::new(value),
        }
    }

    fn expose(&self) -> &[u8; SEED_LEN] {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    InvalidFormat,
    UnsupportedVersion,
    RandomnessUnavailable,
    KeyDerivationFailed,
    EncryptionFailed,
    DecryptionFailed,
    PublicKeyMismatch,
    SerializationFailed,
}

impl fmt::Display for VaultError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidFormat => "invalid vault format",
            Self::UnsupportedVersion => "unsupported vault version",
            Self::RandomnessUnavailable => "operating-system randomness unavailable",
            Self::KeyDerivationFailed => "vault key derivation failed",
            Self::EncryptionFailed => "vault encryption failed",
            Self::DecryptionFailed => "vault decryption failed",
            Self::PublicKeyMismatch => "vault public key does not match decrypted signing key",
            Self::SerializationFailed => "vault serialization failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for VaultError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerState {
    Running,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerError {
    EmptyMessage,
    Locked,
}

impl fmt::Display for SignerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage => formatter.write_str("signing message must not be empty"),
            Self::Locked => formatter.write_str("signer is emergency locked"),
        }
    }
}

impl std::error::Error for SignerError {}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedVault {
    version: u8,
    public_key_b64: String,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
    kdf: KdfParameters,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct KdfParameters {
    algorithm: String,
    memory_kib: u32,
    iterations: u32,
    lanes: u32,
    output_len: usize,
}

pub struct UnlockedWallet {
    signing_key: SigningKey,
    signer_state: SignerState,
}

pub struct AuthorizedMessage<'a> {
    bytes: &'a [u8],
}

impl<'a> AuthorizedMessage<'a> {
    #[cfg(test)]
    fn new(bytes: &'a [u8]) -> Result<Self, SignerError> {
        if bytes.is_empty() {
            return Err(SignerError::EmptyMessage);
        }

        Ok(Self { bytes })
    }

    fn as_bytes(&self) -> &[u8] {
        self.bytes
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SignatureBytes {
    value: [u8; SIGNATURE_LEN],
}

impl SignatureBytes {
    #[must_use]
    pub const fn to_bytes(self) -> [u8; SIGNATURE_LEN] {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    Reserved,
    Signed,
    Submitted,
    Confirmed,
    Failed,
    Ambiguous,
    Quarantined,
    Settled,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    ZeroReservation,
    InvalidTransition,
    BlockhashStillValid,
    OutcomeUnresolved,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroReservation => "transaction reservation must be greater than zero",
            Self::InvalidTransition => "transaction lifecycle transition is invalid",
            Self::BlockhashStillValid => "transaction blockhash is still valid",
            Self::OutcomeUnresolved => "submitted transaction outcome remains unresolved",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LedgerError {}

pub struct TransactionLedgerEntry {
    reserved_lamports: u64,
    recent_blockhash: Hash,
    last_valid_block_height: u64,
    signature: Option<SignatureBytes>,
    state: TransactionState,
}

impl TransactionLedgerEntry {
    pub fn reserve(reserved_lamports: u64, lease: BlockhashLease) -> Result<Self, LedgerError> {
        if reserved_lamports == 0 {
            return Err(LedgerError::ZeroReservation);
        }

        Ok(Self {
            reserved_lamports,
            recent_blockhash: lease.recent_blockhash(),
            last_valid_block_height: lease.last_valid_block_height(),
            signature: None,
            state: TransactionState::Reserved,
        })
    }

    #[must_use]
    pub const fn reserved_lamports(&self) -> u64 {
        self.reserved_lamports
    }

    #[must_use]
    pub const fn recent_blockhash(&self) -> Hash {
        self.recent_blockhash
    }

    #[must_use]
    pub const fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    #[must_use]
    pub const fn signature(&self) -> Option<SignatureBytes> {
        self.signature
    }

    #[must_use]
    pub const fn state(&self) -> TransactionState {
        self.state
    }

    #[must_use]
    pub const fn capital_is_reserved(&self) -> bool {
        !matches!(
            self.state,
            TransactionState::Settled | TransactionState::Released
        )
    }

    #[must_use]
    pub const fn can_retry_delivery(&self, current_block_height: u64) -> bool {
        matches!(
            self.state,
            TransactionState::Submitted | TransactionState::Ambiguous
        ) && current_block_height <= self.last_valid_block_height
    }

    #[must_use]
    pub const fn can_rebuild_economic_intent(&self) -> bool {
        matches!(self.state, TransactionState::Released)
    }

    pub fn mark_signed(&mut self, signature: SignatureBytes) -> Result<(), LedgerError> {
        if self.state != TransactionState::Reserved {
            return Err(LedgerError::InvalidTransition);
        }

        self.signature = Some(signature);
        self.state = TransactionState::Signed;
        Ok(())
    }

    pub fn mark_submitted(&mut self) -> Result<(), LedgerError> {
        if self.state != TransactionState::Signed {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Submitted;
        Ok(())
    }

    pub fn mark_ambiguous(&mut self) -> Result<(), LedgerError> {
        if self.state != TransactionState::Submitted {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Ambiguous;
        Ok(())
    }

    pub fn mark_confirmed(&mut self) -> Result<(), LedgerError> {
        if !matches!(
            self.state,
            TransactionState::Submitted
                | TransactionState::Ambiguous
                | TransactionState::Quarantined
        ) {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Confirmed;
        Ok(())
    }

    pub fn mark_failed(&mut self) -> Result<(), LedgerError> {
        if !matches!(
            self.state,
            TransactionState::Submitted
                | TransactionState::Ambiguous
                | TransactionState::Quarantined
        ) {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Failed;
        Ok(())
    }

    pub fn settle(&mut self) -> Result<(), LedgerError> {
        if self.state != TransactionState::Confirmed {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Settled;
        Ok(())
    }

    pub fn release_terminal_failure(&mut self) -> Result<(), LedgerError> {
        if self.state != TransactionState::Failed {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Released;
        Ok(())
    }

    pub fn release_if_expired(&mut self, current_block_height: u64) -> Result<(), LedgerError> {
        if current_block_height <= self.last_valid_block_height {
            return Err(LedgerError::BlockhashStillValid);
        }

        if matches!(
            self.state,
            TransactionState::Submitted
                | TransactionState::Ambiguous
                | TransactionState::Quarantined
        ) {
            return Err(LedgerError::OutcomeUnresolved);
        }

        if !matches!(
            self.state,
            TransactionState::Reserved | TransactionState::Signed
        ) {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Released;
        Ok(())
    }

    pub fn quarantine_if_expired(&mut self, current_block_height: u64) -> Result<(), LedgerError> {
        if current_block_height <= self.last_valid_block_height {
            return Err(LedgerError::BlockhashStillValid);
        }

        if !matches!(
            self.state,
            TransactionState::Submitted | TransactionState::Ambiguous
        ) {
            return Err(LedgerError::InvalidTransition);
        }

        self.state = TransactionState::Quarantined;
        Ok(())
    }
}

impl LockedVault {
    pub fn generate(passphrase: &SecretPassphrase) -> Result<Self, VaultError> {
        let mut seed = [0_u8; SEED_LEN];
        getrandom::getrandom(&mut seed).map_err(|_| VaultError::RandomnessUnavailable)?;

        let result = Self::seal_seed(passphrase, &seed);
        seed.zeroize();
        result
    }

    pub fn import_seed(
        passphrase: &SecretPassphrase,
        seed: SecretSeed,
    ) -> Result<Self, VaultError> {
        Self::seal_seed(passphrase, seed.expose())
    }

    pub fn unlock(&self, passphrase: &SecretPassphrase) -> Result<UnlockedWallet, VaultError> {
        self.validate_metadata()?;

        let salt = decode_array::<SALT_LEN>(&self.salt_b64)?;
        let nonce = decode_array::<NONCE_LEN>(&self.nonce_b64)?;
        let expected_public_key = decode_array::<32>(&self.public_key_b64)?;
        let ciphertext = BASE64
            .decode(&self.ciphertext_b64)
            .map_err(|_| VaultError::InvalidFormat)?;

        let key = derive_key(passphrase, &salt, &self.kdf)?;
        let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| VaultError::DecryptionFailed)?;
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: ciphertext.as_slice(),
                        aad: AAD,
                    },
                )
                .map_err(|_| VaultError::DecryptionFailed)?,
        );

        if plaintext.len() != SEED_LEN {
            return Err(VaultError::InvalidFormat);
        }

        let mut seed = Zeroizing::new([0_u8; SEED_LEN]);
        seed.as_mut().copy_from_slice(plaintext.as_slice());
        let signing_key = SigningKey::from_bytes(&seed);
        let actual_public_key = signing_key.verifying_key().to_bytes();

        if actual_public_key != expected_public_key {
            return Err(VaultError::PublicKeyMismatch);
        }

        Ok(UnlockedWallet {
            signing_key,
            signer_state: SignerState::Running,
        })
    }

    pub fn public_key(&self) -> Result<[u8; 32], VaultError> {
        self.validate_metadata()?;
        decode_array::<32>(&self.public_key_b64)
    }

    pub fn devnet_account(&self) -> Result<DevnetAccount, VaultError> {
        let address = Pubkey::new_from_array(self.public_key()?);
        Ok(DevnetAccount::new(address))
    }

    pub fn to_json(&self) -> Result<String, VaultError> {
        serde_json::to_string_pretty(self).map_err(|_| VaultError::SerializationFailed)
    }

    pub fn from_json(encoded: &str) -> Result<Self, VaultError> {
        let vault: Self =
            serde_json::from_str(encoded).map_err(|_| VaultError::SerializationFailed)?;
        vault.validate_metadata()?;
        Ok(vault)
    }

    fn seal_seed(passphrase: &SecretPassphrase, seed: &[u8; SEED_LEN]) -> Result<Self, VaultError> {
        let signing_key = SigningKey::from_bytes(seed);
        let public_key = signing_key.verifying_key().to_bytes();

        let mut salt = [0_u8; SALT_LEN];
        getrandom::getrandom(&mut salt).map_err(|_| VaultError::RandomnessUnavailable)?;

        let mut nonce = [0_u8; NONCE_LEN];
        getrandom::getrandom(&mut nonce).map_err(|_| VaultError::RandomnessUnavailable)?;

        let kdf = KdfParameters::current();
        let key = derive_key(passphrase, &salt, &kdf)?;
        let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| VaultError::EncryptionFailed)?;
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: seed,
                    aad: AAD,
                },
            )
            .map_err(|_| VaultError::EncryptionFailed)?;

        Ok(Self {
            version: VAULT_VERSION,
            public_key_b64: BASE64.encode(public_key),
            salt_b64: BASE64.encode(salt),
            nonce_b64: BASE64.encode(nonce),
            ciphertext_b64: BASE64.encode(ciphertext),
            kdf,
        })
    }

    fn validate_metadata(&self) -> Result<(), VaultError> {
        if self.version != VAULT_VERSION {
            return Err(VaultError::UnsupportedVersion);
        }
        if self.kdf != KdfParameters::current() {
            return Err(VaultError::InvalidFormat);
        }

        decode_array::<32>(&self.public_key_b64)?;
        decode_array::<SALT_LEN>(&self.salt_b64)?;
        decode_array::<NONCE_LEN>(&self.nonce_b64)?;

        let ciphertext = BASE64
            .decode(&self.ciphertext_b64)
            .map_err(|_| VaultError::InvalidFormat)?;
        if ciphertext.len() != CIPHERTEXT_LEN {
            return Err(VaultError::InvalidFormat);
        }

        Ok(())
    }
}

impl UnlockedWallet {
    #[must_use]
    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    #[must_use]
    pub fn devnet_account(&self) -> DevnetAccount {
        DevnetAccount::new(Pubkey::new_from_array(self.public_key()))
    }

    #[must_use]
    pub const fn signer_state(&self) -> SignerState {
        self.signer_state
    }

    #[must_use]
    pub const fn signing_is_locked(&self) -> bool {
        matches!(self.signer_state, SignerState::Locked)
    }

    pub fn emergency_lock(&mut self) {
        self.signer_state = SignerState::Locked;
    }

    pub fn sign_authorized(
        &self,
        message: &AuthorizedMessage<'_>,
    ) -> Result<SignatureBytes, SignerError> {
        self.ensure_signing_enabled()?;
        let signature = self.signing_key.sign(message.as_bytes());

        Ok(SignatureBytes {
            value: signature.to_bytes(),
        })
    }

    pub fn sign_transaction_message(
        &self,
        message: &AuthorizedTransactionMessage<'_>,
    ) -> Result<SignatureBytes, SignerError> {
        self.ensure_signing_enabled()?;
        let signature = self.signing_key.sign(message.as_bytes());

        Ok(SignatureBytes {
            value: signature.to_bytes(),
        })
    }

    fn ensure_signing_enabled(&self) -> Result<(), SignerError> {
        if self.signing_is_locked() {
            return Err(SignerError::Locked);
        }

        Ok(())
    }
}

impl KdfParameters {
    fn current() -> Self {
        Self {
            algorithm: "argon2id-v19".to_owned(),
            memory_kib: ARGON2_MEMORY_KIB,
            iterations: ARGON2_ITERATIONS,
            lanes: ARGON2_LANES,
            output_len: KEY_LEN,
        }
    }
}

fn derive_key(
    passphrase: &SecretPassphrase,
    salt: &[u8; SALT_LEN],
    parameters: &KdfParameters,
) -> Result<Zeroizing<[u8; KEY_LEN]>, VaultError> {
    let params = Params::new(
        parameters.memory_kib,
        parameters.iterations,
        parameters.lanes,
        Some(parameters.output_len),
    )
    .map_err(|_| VaultError::KeyDerivationFailed)?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    argon2
        .hash_password_into(passphrase.expose().as_bytes(), salt, key.as_mut())
        .map_err(|_| VaultError::KeyDerivationFailed)?;
    Ok(key)
}

fn decode_array<const N: usize>(encoded: &str) -> Result<[u8; N], VaultError> {
    let decoded = BASE64
        .decode(encoded)
        .map_err(|_| VaultError::InvalidFormat)?;
    decoded.try_into().map_err(|_| VaultError::InvalidFormat)
}

#[cfg(test)]
mod tests {
    use super::{
        build_blockhash_lease, build_get_balance_request, build_get_block_height_request,
        build_get_latest_blockhash_request, parse_block_height_response,
        parse_latest_blockhash_response, AuthorizedMessage, AuthorizedTransactionMessage,
        BlockhashLease, CanonicalTransactionMessage, Cluster, ExecutionPolicy,
        GetBlockHeightResponse, GetLatestBlockhashResponse, LedgerError, LockedVault, PolicyError,
        PreparedTransaction, PreparedTransactionError, RpcError, SecretPassphrase, SecretSeed,
        SignatureBytes, SignerError, SignerState, TransactionLedgerEntry, TransactionMessageError,
        TransactionState, VaultError, CIPHERTEXT_LEN, VAULT_VERSION,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
    use solana_hash::Hash;
    use solana_instruction::Instruction;
    use solana_message::Message;
    use solana_pubkey::Pubkey;

    fn test_lease(
        blockhash: Hash,
        last_valid_block_height: u64,
        observed_block_height: u64,
    ) -> BlockhashLease {
        BlockhashLease::new_for_test(blockhash, last_valid_block_height, observed_block_height)
    }

    #[test]
    fn engine_identity_is_stable() {
        assert_eq!(super::engine_name(), "scout-wallet-lab");
    }

    #[test]
    fn generated_vault_round_trips() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("correct horse battery staple".to_owned());
        let vault = LockedVault::generate(&passphrase)?;
        let expected_public_key = vault.public_key()?;
        let json = vault.to_json()?;
        let parsed = LockedVault::from_json(&json)?;
        let unlocked = parsed.unlock(&passphrase)?;

        assert_eq!(unlocked.public_key(), expected_public_key);
        assert_eq!(unlocked.signer_state(), SignerState::Running);
        Ok(())
    }

    #[test]
    fn imported_seed_is_stable() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("import test".to_owned());
        let first = LockedVault::import_seed(&passphrase, SecretSeed::new([7_u8; 32]))?;
        let second = LockedVault::import_seed(&passphrase, SecretSeed::new([7_u8; 32]))?;

        assert_eq!(first.public_key()?, second.public_key()?);
        Ok(())
    }

    #[test]
    fn wrong_passphrase_is_rejected() -> Result<(), VaultError> {
        let correct = SecretPassphrase::new("correct passphrase".to_owned());
        let wrong = SecretPassphrase::new("wrong passphrase".to_owned());
        let vault = LockedVault::generate(&correct)?;
        let result = vault.unlock(&wrong);

        assert!(matches!(result, Err(VaultError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn unsupported_version_is_rejected() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("version test".to_owned());
        let vault = LockedVault::generate(&passphrase)?;
        let mut json = vault.to_json()?;
        json = json.replacen(
            &format!("\"version\": {VAULT_VERSION}"),
            "\"version\": 99",
            1,
        );

        let result = LockedVault::from_json(&json);
        assert!(matches!(result, Err(VaultError::UnsupportedVersion)));
        Ok(())
    }

    #[test]
    fn serialized_vault_has_no_plaintext_passphrase_or_seed() -> Result<(), VaultError> {
        let passphrase_text = "never serialize this phrase";
        let passphrase = SecretPassphrase::new(passphrase_text.to_owned());
        let seed = [203_u8; 32];
        let seed_b64 = BASE64.encode(seed);
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new(seed))?;
        let json = vault.to_json()?;

        assert!(!json.contains(passphrase_text));
        assert!(!json.contains(&seed_b64));
        Ok(())
    }

    #[test]
    fn ciphertext_tampering_is_rejected() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("tamper test".to_owned());
        let mut vault = LockedVault::generate(&passphrase)?;
        let mut ciphertext = BASE64
            .decode(&vault.ciphertext_b64)
            .map_err(|_| VaultError::InvalidFormat)?;

        let first_byte = ciphertext.first_mut().ok_or(VaultError::InvalidFormat)?;
        *first_byte ^= 1;
        vault.ciphertext_b64 = BASE64.encode(ciphertext);

        let result = vault.unlock(&passphrase);
        assert!(matches!(result, Err(VaultError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn malformed_ciphertext_length_is_rejected() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("length test".to_owned());
        let vault = LockedVault::generate(&passphrase)?;
        let json = vault.to_json()?;
        let encoded = BASE64.encode(vec![0_u8; CIPHERTEXT_LEN - 1]);

        let marker = "\"ciphertext_b64\": \"";
        let start = json.find(marker).ok_or(VaultError::SerializationFailed)? + marker.len();
        let tail = &json[start..];
        let end = tail.find('"').ok_or(VaultError::SerializationFailed)? + start;

        let mut malformed = String::with_capacity(json.len());
        malformed.push_str(&json[..start]);
        malformed.push_str(&encoded);
        malformed.push_str(&json[end..]);

        let result = LockedVault::from_json(&malformed);
        assert!(matches!(result, Err(VaultError::InvalidFormat)));
        Ok(())
    }

    #[test]
    fn empty_authorized_message_is_rejected() {
        let result = AuthorizedMessage::new(&[]);
        assert!(matches!(result, Err(SignerError::EmptyMessage)));
    }

    #[test]
    fn authorized_message_signature_verifies() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("signing test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([19_u8; 32]))?;
        let unlocked = vault.unlock(&passphrase)?;
        let public_key = unlocked.public_key();
        let message_bytes = b"scout authorized execution message";
        let message =
            AuthorizedMessage::new(message_bytes).map_err(|_| VaultError::SerializationFailed)?;
        let signature = unlocked
            .sign_authorized(&message)
            .map_err(|_| VaultError::SerializationFailed)?;

        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| VaultError::InvalidFormat)?;
        let signature = Signature::from_bytes(&signature.to_bytes());

        assert!(verifying_key.verify(message_bytes, &signature).is_ok());
        Ok(())
    }

    #[test]
    fn emergency_lock_blocks_authorized_message_signing() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("emergency lock message test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([23_u8; 32]))?;
        let mut unlocked = vault.unlock(&passphrase)?;
        let message = AuthorizedMessage::new(b"authorized before lock")
            .map_err(|_| VaultError::SerializationFailed)?;

        unlocked.emergency_lock();

        assert_eq!(unlocked.signer_state(), SignerState::Locked);
        assert!(unlocked.signing_is_locked());
        assert!(matches!(
            unlocked.sign_authorized(&message),
            Err(SignerError::Locked)
        ));
        Ok(())
    }

    #[test]
    fn locked_vault_exposes_canonical_devnet_account() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("account test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([31_u8; 32]))?;
        let expected = Pubkey::new_from_array(vault.public_key()?);
        let account = vault.devnet_account()?;

        assert_eq!(account.address(), expected);
        assert_eq!(account.cluster(), Cluster::Devnet);
        assert_eq!(account.cluster().rpc_name(), "devnet");
        assert_eq!(account.cluster().rpc_url(), "https://api.devnet.solana.com");
        assert_eq!(account.lamports(), None);
        Ok(())
    }

    #[test]
    fn unlocked_wallet_exposes_same_devnet_account() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("unlocked account test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([41_u8; 32]))?;
        let locked_account = vault.devnet_account()?;
        let unlocked = vault.unlock(&passphrase)?;
        let unlocked_account = unlocked.devnet_account();

        assert_eq!(locked_account.address(), unlocked_account.address());
        Ok(())
    }

    #[test]
    fn devnet_balance_state_is_explicit() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("balance state test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([53_u8; 32]))?;
        let mut account = vault.devnet_account()?;

        assert_eq!(account.lamports(), None);

        account.record_balance(1_500_000_000);
        assert_eq!(account.lamports(), Some(1_500_000_000));

        account.clear_balance();
        assert_eq!(account.lamports(), None);
        Ok(())
    }

    #[test]
    fn devnet_rpc_request_is_read_only_and_fixed() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("rpc request test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([61_u8; 32]))?;
        let account = vault.devnet_account()?;
        let request = build_get_balance_request(account.address());
        let encoded = serde_json::to_value(request).map_err(|_| VaultError::SerializationFailed)?;

        assert_eq!(encoded["jsonrpc"], "2.0");
        assert_eq!(encoded["id"], 1);
        assert_eq!(encoded["method"], "getBalance");
        assert_eq!(encoded["params"][0], account.address().to_string());
        assert_eq!(encoded["params"][1]["commitment"], "confirmed");
        Ok(())
    }

    #[test]
    fn blockhash_rpc_requests_are_read_only_and_fixed() -> Result<(), VaultError> {
        let latest = serde_json::to_value(build_get_latest_blockhash_request())
            .map_err(|_| VaultError::SerializationFailed)?;
        let height = serde_json::to_value(build_get_block_height_request())
            .map_err(|_| VaultError::SerializationFailed)?;

        assert_eq!(latest["jsonrpc"], "2.0");
        assert_eq!(latest["id"], 1);
        assert_eq!(latest["method"], "getLatestBlockhash");
        assert_eq!(latest["params"][0]["commitment"], "confirmed");

        assert_eq!(height["jsonrpc"], "2.0");
        assert_eq!(height["id"], 1);
        assert_eq!(height["method"], "getBlockHeight");
        assert_eq!(height["params"][0]["commitment"], "confirmed");
        Ok(())
    }

    #[test]
    fn latest_blockhash_response_parses_canonical_hash() -> Result<(), RpcError> {
        let expected = Hash::new_from_array([67_u8; 32]);
        let encoded = format!(
            r#"{{"result":{{"context":{{"slot":1}},"value":{{"blockhash":"{}","lastValidBlockHeight":500}}}},"error":null}}"#,
            expected
        );
        let response: GetLatestBlockhashResponse =
            serde_json::from_str(&encoded).map_err(|_| RpcError::InvalidResponse)?;
        let (blockhash, last_valid_block_height) = parse_latest_blockhash_response(response)?;

        assert_eq!(blockhash, expected);
        assert_eq!(last_valid_block_height, 500);
        Ok(())
    }

    #[test]
    fn block_height_response_parses_height() -> Result<(), RpcError> {
        let response: GetBlockHeightResponse =
            serde_json::from_str(r#"{"result":490,"error":null}"#)
                .map_err(|_| RpcError::InvalidResponse)?;

        assert_eq!(parse_block_height_response(response)?, 490);
        Ok(())
    }

    #[test]
    fn expired_blockhash_lease_is_rejected() {
        let blockhash = Hash::new_from_array([69_u8; 32]);
        let result = build_blockhash_lease(blockhash, 500, 501);

        assert!(matches!(result, Err(RpcError::BlockhashExpired)));
    }

    #[test]
    fn fresh_blockhash_lease_preserves_validity_metadata() -> Result<(), RpcError> {
        let blockhash = Hash::new_from_array([70_u8; 32]);
        let lease = build_blockhash_lease(blockhash, 500, 490)?;

        assert_eq!(lease.recent_blockhash(), blockhash);
        assert_eq!(lease.last_valid_block_height(), 500);
        assert_eq!(lease.observed_block_height(), 490);
        assert!(lease.is_valid());
        Ok(())
    }

    #[test]
    fn empty_transaction_message_is_rejected() {
        let payer = Pubkey::new_from_array([71_u8; 32]);
        let blockhash = Hash::new_from_array([73_u8; 32]);
        let result = CanonicalTransactionMessage::new(&[], payer, blockhash);

        assert!(matches!(
            result,
            Err(TransactionMessageError::EmptyInstructions)
        ));
    }

    #[test]
    fn canonical_transaction_message_is_deterministic() -> Result<(), TransactionMessageError> {
        let payer = Pubkey::new_from_array([79_u8; 32]);
        let program_id = Pubkey::new_from_array([83_u8; 32]);
        let blockhash = Hash::new_from_array([89_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![1_u8, 2_u8, 3_u8, 4_u8],
        };

        let first = CanonicalTransactionMessage::new(&[instruction.clone()], payer, blockhash)?;
        let second = CanonicalTransactionMessage::new(&[instruction], payer, blockhash)?;

        assert_eq!(first.bytes(), second.bytes());
        assert!(!first.bytes().is_empty());
        Ok(())
    }

    #[test]
    fn canonical_transaction_message_signature_verifies() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("transaction signing test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([97_u8; 32]))?;
        let unlocked = vault.unlock(&passphrase)?;
        let payer = Pubkey::new_from_array(unlocked.public_key());
        let program_id = Pubkey::new_from_array([101_u8; 32]);
        let blockhash = Hash::new_from_array([103_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![9_u8, 8_u8, 7_u8],
        };
        let canonical = CanonicalTransactionMessage::new(&[instruction], payer, blockhash)
            .map_err(|_| VaultError::SerializationFailed)?;
        let authorized = AuthorizedTransactionMessage::new(&canonical);
        let signature = unlocked
            .sign_transaction_message(&authorized)
            .map_err(|_| VaultError::SerializationFailed)?;

        let verifying_key = VerifyingKey::from_bytes(&unlocked.public_key())
            .map_err(|_| VaultError::InvalidFormat)?;
        let signature = Signature::from_bytes(&signature.to_bytes());

        assert!(verifying_key.verify(canonical.bytes(), &signature).is_ok());
        Ok(())
    }

    #[test]
    fn emergency_lock_blocks_transaction_signing() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("emergency transaction lock test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([99_u8; 32]))?;
        let mut unlocked = vault.unlock(&passphrase)?;
        let payer = Pubkey::new_from_array(unlocked.public_key());
        let instruction = Instruction {
            program_id: Pubkey::new_from_array([100_u8; 32]),
            accounts: Vec::new(),
            data: vec![1_u8],
        };
        let canonical = CanonicalTransactionMessage::new(
            &[instruction],
            payer,
            Hash::new_from_array([102; 32]),
        )
        .map_err(|_| VaultError::SerializationFailed)?;
        let authorized = AuthorizedTransactionMessage::new(&canonical);

        unlocked.emergency_lock();

        assert!(matches!(
            unlocked.sign_transaction_message(&authorized),
            Err(SignerError::Locked)
        ));
        Ok(())
    }

    #[test]
    fn emergency_lock_is_irreversible_for_unlocked_wallet() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("irreversible lock test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([103_u8; 32]))?;
        let mut unlocked = vault.unlock(&passphrase)?;

        unlocked.emergency_lock();
        unlocked.emergency_lock();

        assert_eq!(unlocked.signer_state(), SignerState::Locked);
        assert!(unlocked.signing_is_locked());
        Ok(())
    }

    #[test]
    fn policy_rejects_zero_exposure_limit() {
        let program_id = Pubkey::new_from_array([151_u8; 32]);
        let result = ExecutionPolicy::new(0, &[program_id]);

        assert!(matches!(result, Err(PolicyError::ZeroExposureLimit)));
    }

    #[test]
    fn policy_rejects_empty_program_allowlist() {
        let result = ExecutionPolicy::new(500_000_000, &[]);

        assert!(matches!(result, Err(PolicyError::NoAllowedPrograms)));
    }

    #[test]
    fn policy_rejects_exposure_above_limit() -> Result<(), VaultError> {
        let payer = Pubkey::new_from_array([152_u8; 32]);
        let program_id = Pubkey::new_from_array([153_u8; 32]);
        let blockhash = Hash::new_from_array([154_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![1_u8],
        };
        let lease = test_lease(blockhash, 700, 690);
        let prepared = PreparedTransaction::reserve(&[instruction], payer, 600_000_000, lease)
            .map_err(|_| VaultError::SerializationFailed)?;
        let policy = ExecutionPolicy::new(500_000_000, &[program_id])
            .map_err(|_| VaultError::SerializationFailed)?;

        let result = policy.authorize(&prepared, 690);

        assert!(matches!(result, Err(PolicyError::ExposureExceeded)));
        Ok(())
    }

    #[test]
    fn policy_rejects_unapproved_program() -> Result<(), VaultError> {
        let payer = Pubkey::new_from_array([155_u8; 32]);
        let program_id = Pubkey::new_from_array([156_u8; 32]);
        let allowed_program = Pubkey::new_from_array([157_u8; 32]);
        let blockhash = Hash::new_from_array([158_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![2_u8],
        };
        let lease = test_lease(blockhash, 800, 790);
        let prepared = PreparedTransaction::reserve(&[instruction], payer, 400_000_000, lease)
            .map_err(|_| VaultError::SerializationFailed)?;
        let policy = ExecutionPolicy::new(500_000_000, &[allowed_program])
            .map_err(|_| VaultError::SerializationFailed)?;

        let result = policy.authorize(&prepared, 790);

        assert!(matches!(result, Err(PolicyError::ProgramNotAllowed)));
        Ok(())
    }

    #[test]
    fn policy_rejects_expired_reservation() -> Result<(), VaultError> {
        let payer = Pubkey::new_from_array([159_u8; 32]);
        let program_id = Pubkey::new_from_array([160_u8; 32]);
        let blockhash = Hash::new_from_array([161_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![3_u8],
        };
        let lease = test_lease(blockhash, 900, 890);
        let prepared = PreparedTransaction::reserve(&[instruction], payer, 400_000_000, lease)
            .map_err(|_| VaultError::SerializationFailed)?;
        let policy = ExecutionPolicy::new(500_000_000, &[program_id])
            .map_err(|_| VaultError::SerializationFailed)?;

        let result = policy.authorize(&prepared, 901);

        assert!(matches!(result, Err(PolicyError::BlockhashExpired)));
        Ok(())
    }

    #[test]
    fn policy_rejects_transaction_after_reserved_state() -> Result<(), VaultError> {
        let payer = Pubkey::new_from_array([162_u8; 32]);
        let program_id = Pubkey::new_from_array([163_u8; 32]);
        let blockhash = Hash::new_from_array([164_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![4_u8],
        };
        let lease = test_lease(blockhash, 1_000, 990);
        let mut prepared = PreparedTransaction::reserve(&[instruction], payer, 400_000_000, lease)
            .map_err(|_| VaultError::SerializationFailed)?;
        let policy = ExecutionPolicy::new(500_000_000, &[program_id])
            .map_err(|_| VaultError::SerializationFailed)?;
        let signature = SignatureBytes {
            value: [165_u8; 64],
        };

        prepared
            .ledger_mut()
            .mark_signed(signature)
            .map_err(|_| VaultError::SerializationFailed)?;

        let result = policy.authorize(&prepared, 990);

        assert!(matches!(result, Err(PolicyError::TransactionNotReserved)));
        Ok(())
    }

    #[test]
    fn policy_mints_authorization_for_reserved_allowed_transaction() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("policy signing test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([166_u8; 32]))?;
        let unlocked = vault.unlock(&passphrase)?;
        let payer = Pubkey::new_from_array(unlocked.public_key());
        let program_id = Pubkey::new_from_array([167_u8; 32]);
        let blockhash = Hash::new_from_array([168_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![5_u8, 6_u8],
        };
        let lease = test_lease(blockhash, 1_100, 1_090);
        let mut prepared = PreparedTransaction::reserve(&[instruction], payer, 400_000_000, lease)
            .map_err(|_| VaultError::SerializationFailed)?;
        let policy = ExecutionPolicy::new(500_000_000, &[program_id])
            .map_err(|_| VaultError::SerializationFailed)?;

        assert_eq!(policy.max_reserved_lamports(), 500_000_000);
        assert_eq!(policy.allowed_programs(), &[program_id]);

        let signature = {
            let authorized = policy
                .authorize(&prepared, 1_090)
                .map_err(|_| VaultError::SerializationFailed)?;

            unlocked
                .sign_transaction_message(&authorized)
                .map_err(|_| VaultError::SerializationFailed)?
        };

        let verifying_key = VerifyingKey::from_bytes(&unlocked.public_key())
            .map_err(|_| VaultError::InvalidFormat)?;
        let verification_signature = Signature::from_bytes(&signature.to_bytes());

        assert!(verifying_key
            .verify(prepared.message().bytes(), &verification_signature)
            .is_ok());

        prepared
            .ledger_mut()
            .mark_signed(signature)
            .map_err(|_| VaultError::SerializationFailed)?;

        assert_eq!(prepared.ledger().state(), TransactionState::Signed);
        assert!(prepared.ledger().signature() == Some(signature));
        Ok(())
    }

    #[test]
    fn prepared_transaction_binds_message_and_ledger_to_same_blockhash(
    ) -> Result<(), PreparedTransactionError> {
        let payer = Pubkey::new_from_array([104_u8; 32]);
        let program_id = Pubkey::new_from_array([105_u8; 32]);
        let blockhash = Hash::new_from_array([106_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![1_u8, 3_u8, 5_u8],
        };
        let lease = test_lease(blockhash, 600, 590);
        let prepared = PreparedTransaction::reserve(&[instruction], payer, 500_000_000, lease)?;

        let decoded: Message = bincode::deserialize(prepared.message().bytes()).map_err(|_| {
            PreparedTransactionError::Message(TransactionMessageError::SerializationFailed)
        })?;

        assert_eq!(decoded.recent_blockhash, blockhash);
        assert_eq!(prepared.ledger().recent_blockhash(), blockhash);
        assert_eq!(prepared.ledger().last_valid_block_height(), 600);
        assert_eq!(prepared.ledger().reserved_lamports(), 500_000_000);
        assert_eq!(prepared.ledger().state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn prepared_transaction_rejects_empty_instructions_before_reservation() {
        let payer = Pubkey::new_from_array([106_u8; 32]);
        let blockhash = Hash::new_from_array([107_u8; 32]);
        let lease = test_lease(blockhash, 600, 590);
        let result = PreparedTransaction::reserve(&[], payer, 500_000_000, lease);

        assert!(matches!(
            result,
            Err(PreparedTransactionError::Message(
                TransactionMessageError::EmptyInstructions
            ))
        ));
    }

    #[test]
    fn prepared_transaction_rejects_zero_reservation() {
        let payer = Pubkey::new_from_array([107_u8; 32]);
        let program_id = Pubkey::new_from_array([108_u8; 32]);
        let blockhash = Hash::new_from_array([109_u8; 32]);
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![2_u8, 4_u8, 6_u8],
        };
        let lease = test_lease(blockhash, 600, 590);
        let result = PreparedTransaction::reserve(&[instruction], payer, 0, lease);

        assert!(matches!(
            result,
            Err(PreparedTransactionError::Ledger(
                LedgerError::ZeroReservation
            ))
        ));
    }

    #[test]
    fn zero_transaction_reservation_is_rejected() {
        let blockhash = Hash::new_from_array([107_u8; 32]);
        let lease = test_lease(blockhash, 500, 490);
        let result = TransactionLedgerEntry::reserve(0, lease);

        assert!(matches!(result, Err(LedgerError::ZeroReservation)));
    }

    #[test]
    fn ledger_reservation_consumes_blockhash_lease() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([108_u8; 32]);
        let lease = test_lease(blockhash, 500, 490);
        let entry = TransactionLedgerEntry::reserve(900_000_000, lease)?;

        assert_eq!(entry.recent_blockhash(), blockhash);
        assert_eq!(entry.last_valid_block_height(), 500);
        assert_eq!(entry.state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn ambiguous_submission_is_quarantined_after_expiry() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([109_u8; 32]);
        let lease = test_lease(blockhash, 500, 490);
        let mut entry = TransactionLedgerEntry::reserve(900_000_000, lease)?;
        let signature = SignatureBytes {
            value: [113_u8; 64],
        };

        entry.mark_signed(signature)?;
        entry.mark_submitted()?;
        entry.mark_ambiguous()?;

        assert_eq!(entry.state(), TransactionState::Ambiguous);
        assert!(entry.capital_is_reserved());
        assert!(entry.can_retry_delivery(500));
        assert!(!entry.can_rebuild_economic_intent());

        let early_release = entry.release_if_expired(500);
        assert!(matches!(
            early_release,
            Err(LedgerError::BlockhashStillValid)
        ));

        let unresolved_release = entry.release_if_expired(501);
        assert!(matches!(
            unresolved_release,
            Err(LedgerError::OutcomeUnresolved)
        ));

        entry.quarantine_if_expired(501)?;

        assert_eq!(entry.state(), TransactionState::Quarantined);
        assert!(entry.capital_is_reserved());
        assert!(!entry.can_retry_delivery(501));
        assert!(!entry.can_rebuild_economic_intent());
        Ok(())
    }

    #[test]
    fn quarantined_failure_requires_resolution_before_release() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([119_u8; 32]);
        let lease = test_lease(blockhash, 700, 690);
        let mut entry = TransactionLedgerEntry::reserve(800_000_000, lease)?;
        let signature = SignatureBytes {
            value: [121_u8; 64],
        };

        entry.mark_signed(signature)?;
        entry.mark_submitted()?;
        entry.mark_ambiguous()?;
        entry.quarantine_if_expired(701)?;

        assert_eq!(entry.state(), TransactionState::Quarantined);
        assert!(entry.capital_is_reserved());

        entry.mark_failed()?;
        entry.release_terminal_failure()?;

        assert_eq!(entry.state(), TransactionState::Released);
        assert!(!entry.capital_is_reserved());
        assert!(entry.can_rebuild_economic_intent());
        Ok(())
    }

    #[test]
    fn unsubmitted_transaction_can_release_after_expiry() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([123_u8; 32]);
        let lease = test_lease(blockhash, 800, 790);
        let mut entry = TransactionLedgerEntry::reserve(600_000_000, lease)?;
        let signature = SignatureBytes {
            value: [125_u8; 64],
        };

        entry.mark_signed(signature)?;
        entry.release_if_expired(801)?;

        assert_eq!(entry.state(), TransactionState::Released);
        assert!(!entry.capital_is_reserved());
        assert!(entry.can_rebuild_economic_intent());
        Ok(())
    }

    #[test]
    fn confirmed_submission_must_settle_before_capital_releases() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([127_u8; 32]);
        let lease = test_lease(blockhash, 900, 890);
        let mut entry = TransactionLedgerEntry::reserve(700_000_000, lease)?;
        let signature = SignatureBytes {
            value: [131_u8; 64],
        };

        entry.mark_signed(signature)?;
        entry.mark_submitted()?;
        entry.mark_confirmed()?;

        assert_eq!(entry.state(), TransactionState::Confirmed);
        assert!(entry.capital_is_reserved());
        assert!(!entry.can_rebuild_economic_intent());

        entry.settle()?;

        assert_eq!(entry.state(), TransactionState::Settled);
        assert!(!entry.capital_is_reserved());
        assert!(!entry.can_rebuild_economic_intent());
        Ok(())
    }

    #[test]
    fn terminal_failure_requires_explicit_release() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([137_u8; 32]);
        let lease = test_lease(blockhash, 1_200, 1_190);
        let mut entry = TransactionLedgerEntry::reserve(400_000_000, lease)?;
        let signature = SignatureBytes {
            value: [139_u8; 64],
        };

        entry.mark_signed(signature)?;
        entry.mark_submitted()?;
        entry.mark_failed()?;

        assert_eq!(entry.state(), TransactionState::Failed);
        assert!(entry.capital_is_reserved());
        assert!(!entry.can_rebuild_economic_intent());

        entry.release_terminal_failure()?;

        assert_eq!(entry.state(), TransactionState::Released);
        assert!(!entry.capital_is_reserved());
        assert!(entry.can_rebuild_economic_intent());
        Ok(())
    }

    #[test]
    fn invalid_lifecycle_transition_is_rejected() -> Result<(), LedgerError> {
        let blockhash = Hash::new_from_array([149_u8; 32]);
        let lease = test_lease(blockhash, 1_500, 1_490);
        let mut entry = TransactionLedgerEntry::reserve(200_000_000, lease)?;
        let result = entry.mark_submitted();

        assert!(matches!(result, Err(LedgerError::InvalidTransition)));
        assert_eq!(entry.state(), TransactionState::Reserved);
        Ok(())
    }
}
