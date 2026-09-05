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

    pub fn sign_authorized(
        &self,
        message: &AuthorizedMessage<'_>,
    ) -> Result<SignatureBytes, SignerError> {
        let signature = self.signing_key.sign(message.as_bytes());

        Ok(SignatureBytes {
            value: signature.to_bytes(),
        })
    }

    #[must_use]
    pub fn sign_transaction_message(
        &self,
        message: &AuthorizedTransactionMessage<'_>,
    ) -> SignatureBytes {
        let signature = self.signing_key.sign(message.as_bytes());

        SignatureBytes {
            value: signature.to_bytes(),
        }
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
        build_get_balance_request, AuthorizedMessage, AuthorizedTransactionMessage,
        CanonicalTransactionMessage, Cluster, LockedVault, SecretPassphrase, SecretSeed,
        SignerError, TransactionMessageError, VaultError, CIPHERTEXT_LEN, VAULT_VERSION,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
    use solana_hash::Hash;
    use solana_instruction::Instruction;
    use solana_pubkey::Pubkey;

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
        let signature = unlocked.sign_transaction_message(&authorized);

        let verifying_key = VerifyingKey::from_bytes(&unlocked.public_key())
            .map_err(|_| VaultError::InvalidFormat)?;
        let signature = Signature::from_bytes(&signature.to_bytes());

        assert!(verifying_key.verify(canonical.bytes(), &signature).is_ok());
        Ok(())
    }
}
