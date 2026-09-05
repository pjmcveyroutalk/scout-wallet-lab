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
use std::{fmt, time::Duration};
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
pub enum RpcError {
    TlsInitializationFailed,
    ClientInitializationFailed,
    TransportFailed,
    HttpStatusFailed,
    InvalidResponse,
    RpcRejected,
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
pub enum SignerError {
    EmptyMessage,
}

impl fmt::Display for SignerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage => formatter.write_str("signing message must not be empty"),
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
    pub fn reserve(
        reserved_lamports: u64,
        recent_blockhash: Hash,
        last_valid_block_height: u64,
    ) -> Result<Self, LedgerError> {
        if reserved_lamports == 0 {
            return Err(LedgerError::ZeroReservation);
        }

        Ok(Self {
            reserved_lamports,
            recent_blockhash,
            last_valid_block_height,
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

    pub fn quarantine_if_expired(
        &mut self,
        current_block_height: u64,
    ) -> Result<(), LedgerError> {
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

        Ok(UnlockedWallet { signing_key })
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

        let kdf = KdfPa
