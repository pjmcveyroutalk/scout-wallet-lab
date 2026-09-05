#![forbid(unsafe_code)]

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

const VAULT_VERSION: u8 = 1;
const AAD: &[u8] = b"scout-wallet-v1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const SEED_LEN: usize = 32;
const KEY_LEN: usize = 32;

const ARGON2_MEMORY_KIB: u32 = 65_536;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_LANES: u32 = 1;

/// Returns the fixed identity of the isolated wallet engine crate.
#[must_use]
pub const fn engine_name() -> &'static str {
    "scout-wallet-lab"
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

impl LockedVault {
    pub fn generate(passphrase: &str) -> Result<Self, VaultError> {
        let mut seed = Zeroizing::new([0_u8; SEED_LEN]);
        getrandom::getrandom(seed.as_mut()).map_err(|_| VaultError::RandomnessUnavailable)?;

        Self::seal_seed(passphrase, seed.as_ref())
    }

    pub fn unlock(&self, passphrase: &str) -> Result<UnlockedWallet, VaultError> {
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
        let signing_key = SigningKey::from_bytes(seed.as_ref());
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

    pub fn to_json(&self) -> Result<String, VaultError> {
        serde_json::to_string_pretty(self).map_err(|_| VaultError::SerializationFailed)
    }

    pub fn from_json(encoded: &str) -> Result<Self, VaultError> {
        let vault: Self =
            serde_json::from_str(encoded).map_err(|_| VaultError::SerializationFailed)?;
        vault.validate_metadata()?;
        Ok(vault)
    }

    fn seal_seed(passphrase: &str, seed: &[u8; SEED_LEN]) -> Result<Self, VaultError> {
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
                Payload { msg: seed, aad: AAD },
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
        if BASE64.decode(&self.ciphertext_b64).is_err() {
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
    passphrase: &str,
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
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
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
    use super::{LockedVault, VaultError, VAULT_VERSION};

    #[test]
    fn engine_identity_is_stable() {
        assert_eq!(super::engine_name(), "scout-wallet-lab");
    }

    #[test]
    fn generated_vault_round_trips() -> Result<(), VaultError> {
        let vault = LockedVault::generate("correct horse battery staple")?;
        let expected_public_key = vault.public_key()?;
        let json = vault.to_json()?;
        let parsed = LockedVault::from_json(&json)?;
        let unlocked = parsed.unlock("correct horse battery staple")?;

        assert_eq!(unlocked.public_key(), expected_public_key);
        Ok(())
    }

    #[test]
    fn wrong_passphrase_is_rejected() -> Result<(), VaultError> {
        let vault = LockedVault::generate("correct passphrase")?;
        let result = vault.unlock("wrong passphrase");

        assert!(matches!(result, Err(VaultError::DecryptionFailed)));
        Ok(())
    }

    #[test]
    fn ciphertext_tampering_is_rejected() -> Result<(), VaultError> {
        let vault = LockedVault::generate("tamper test")?;
        let mut json = vault.to_json()?;
        let replacement = if json.contains("\"ciphertext_b64\": \"A") {
            "\"ciphertext_b64\": \"B"
        } else {
            "\"ciphertext_b64\": \"A"
        };
        json = json.replacen("\"ciphertext_b64\": \"", replacement, 1);

        let parsed = LockedVault::from_json(&json)?;
        let result = parsed.unlock("tamper test");

        assert!(matches!(
            result,
            Err(VaultError::DecryptionFailed | VaultError::InvalidFormat)
        ));
        Ok(())
    }

    #[test]
    fn unsupported_version_is_rejected() -> Result<(), VaultError> {
        let vault = LockedVault::generate("version test")?;
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
    fn serialized_vault_has_no_plaintext_passphrase() -> Result<(), VaultError> {
        let passphrase = "never serialize this phrase";
        let vault = LockedVault::generate(passphrase)?;
        let json = vault.to_json()?;

        assert!(!json.contains(passphrase));
        Ok(())
    }
}
