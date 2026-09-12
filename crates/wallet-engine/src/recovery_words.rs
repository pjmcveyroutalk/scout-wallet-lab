#[path = "devnet_signing_coordinator.rs"]
pub mod devnet_signing_coordinator;
#[path = "stage_e_preflight.rs"]
pub mod stage_e_preflight;

use bip39::{Language, Mnemonic};
use ed25519_dalek::SigningKey;
use solana_pubkey::Pubkey;
use std::fmt;
use zeroize::{Zeroize, Zeroizing};

const RECOVERY_WORD_COUNT: usize = 24;
const RECOVERY_ENTROPY_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryWordsError {
    InvalidMnemonic,
    InvalidWordCount,
    InvalidEntropyLength,
    PublicKeyMismatch,
    RoundTripMismatch,
}

impl fmt::Display for RecoveryWordsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMnemonic => "Scout emergency recovery words are invalid",
            Self::InvalidWordCount => {
                "Scout emergency recovery words must contain exactly 24 words"
            }
            Self::InvalidEntropyLength => {
                "Scout emergency recovery words did not decode to a 32-byte signing seed"
            }
            Self::PublicKeyMismatch => {
                "Scout emergency recovery words do not match the expected wallet identity"
            }
            Self::RoundTripMismatch => {
                "Scout emergency recovery words failed internal round-trip verification"
            }
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for RecoveryWordsError {}

pub struct RecoveryWords {
    words: Zeroizing<String>,
    public_key: Pubkey,
}

impl RecoveryWords {
    #[must_use]
    pub fn words(&self) -> &str {
        self.words.as_str()
    }

    #[must_use]
    pub const fn public_key(&self) -> Pubkey {
        self.public_key
    }
}

pub(crate) fn recovery_words_from_signing_seed(
    signing_seed: &[u8; RECOVERY_ENTROPY_LEN],
    expected_public_key: Pubkey,
) -> Result<RecoveryWords, RecoveryWordsError> {
    verify_seed_public_key(signing_seed, expected_public_key)?;

    let mnemonic =
        Mnemonic::from_entropy(signing_seed).map_err(|_| RecoveryWordsError::InvalidMnemonic)?;

    if mnemonic.word_count() != RECOVERY_WORD_COUNT {
        return Err(RecoveryWordsError::InvalidWordCount);
    }

    let words = Zeroizing::new(mnemonic.to_string());

    let recovered_seed = signing_seed_from_recovery_words(words.as_str())?;

    if recovered_seed.as_ref() != signing_seed {
        return Err(RecoveryWordsError::RoundTripMismatch);
    }

    verify_seed_public_key(&recovered_seed, expected_public_key)?;

    Ok(RecoveryWords {
        words,
        public_key: expected_public_key,
    })
}

pub(crate) fn signing_seed_from_recovery_words(
    words: &str,
) -> Result<Zeroizing<[u8; RECOVERY_ENTROPY_LEN]>, RecoveryWordsError> {
    let normalized_words = normalize_recovery_words(words);

    if normalized_words.split_whitespace().count() != RECOVERY_WORD_COUNT {
        return Err(RecoveryWordsError::InvalidWordCount);
    }

    let mnemonic = Mnemonic::parse_in_normalized(Language::English, normalized_words.as_str())
        .map_err(|_| RecoveryWordsError::InvalidMnemonic)?;

    if mnemonic.word_count() != RECOVERY_WORD_COUNT {
        return Err(RecoveryWordsError::InvalidWordCount);
    }

    let (mut entropy, entropy_len) = mnemonic.to_entropy_array();

    if entropy_len != RECOVERY_ENTROPY_LEN {
        entropy.zeroize();
        return Err(RecoveryWordsError::InvalidEntropyLength);
    }

    let mut signing_seed = Zeroizing::new([0_u8; RECOVERY_ENTROPY_LEN]);
    signing_seed.copy_from_slice(&entropy[..RECOVERY_ENTROPY_LEN]);
    entropy.zeroize();

    Ok(signing_seed)
}

#[cfg(test)]
pub(crate) fn verify_recovery_words_identity(
    words: &str,
    expected_public_key: Pubkey,
) -> Result<(), RecoveryWordsError> {
    let signing_seed = signing_seed_from_recovery_words(words)?;
    verify_seed_public_key(&signing_seed, expected_public_key)
}

fn normalize_recovery_words(words: &str) -> Zeroizing<String> {
    Zeroizing::new(words.split_whitespace().collect::<Vec<&str>>().join(" "))
}

fn verify_seed_public_key(
    signing_seed: &[u8; RECOVERY_ENTROPY_LEN],
    expected_public_key: Pubkey,
) -> Result<(), RecoveryWordsError> {
    let signing_key = SigningKey::from_bytes(signing_seed);
    let recovered_public_key = Pubkey::new_from_array(signing_key.verifying_key().to_bytes());

    if recovered_public_key != expected_public_key {
        return Err(RecoveryWordsError::PublicKeyMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        recovery_words_from_signing_seed, signing_seed_from_recovery_words,
        verify_recovery_words_identity, RecoveryWordsError, RECOVERY_ENTROPY_LEN,
        RECOVERY_WORD_COUNT,
    };
    use ed25519_dalek::SigningKey;
    use solana_pubkey::Pubkey;

    const ZERO_ENTROPY_24_WORD_VECTOR: &str = concat!(
        "abandon abandon abandon abandon abandon abandon ",
        "abandon abandon abandon abandon abandon abandon ",
        "abandon abandon abandon abandon abandon abandon ",
        "abandon abandon abandon abandon abandon art"
    );

    fn public_key_for_seed(seed: &[u8; RECOVERY_ENTROPY_LEN]) -> Pubkey {
        let signing_key = SigningKey::from_bytes(seed);
        Pubkey::new_from_array(signing_key.verifying_key().to_bytes())
    }

    #[test]
    fn zero_entropy_matches_known_24_word_vector() -> Result<(), RecoveryWordsError> {
        let seed = [0_u8; RECOVERY_ENTROPY_LEN];
        let public_key = public_key_for_seed(&seed);

        let recovery_words = recovery_words_from_signing_seed(&seed, public_key)?;

        assert_eq!(recovery_words.words(), ZERO_ENTROPY_24_WORD_VECTOR);
        assert_eq!(
            recovery_words.words().split_whitespace().count(),
            RECOVERY_WORD_COUNT
        );
        assert_eq!(recovery_words.public_key(), public_key);

        Ok(())
    }

    #[test]
    fn recovery_words_round_trip_exact_signing_seed() -> Result<(), RecoveryWordsError> {
        let seed = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let public_key = public_key_for_seed(&seed);

        let recovery_words = recovery_words_from_signing_seed(&seed, public_key)?;
        let recovered_seed = signing_seed_from_recovery_words(recovery_words.words())?;

        assert_eq!(recovered_seed.as_ref(), &seed);

        Ok(())
    }

    #[test]
    fn recovery_words_reconstruct_same_public_key() -> Result<(), RecoveryWordsError> {
        let seed = [0x5a_u8; RECOVERY_ENTROPY_LEN];
        let public_key = public_key_for_seed(&seed);

        let recovery_words = recovery_words_from_signing_seed(&seed, public_key)?;

        verify_recovery_words_identity(recovery_words.words(), public_key)
    }

    #[test]
    fn whitespace_is_normalized_before_validation() -> Result<(), RecoveryWordsError> {
        let seed = [0_u8; RECOVERY_ENTROPY_LEN];
        let public_key = public_key_for_seed(&seed);

        let spaced = ZERO_ENTROPY_24_WORD_VECTOR.replace(' ', "   \n\t");
        let recovered_seed = signing_seed_from_recovery_words(spaced.as_str())?;

        assert_eq!(recovered_seed.as_ref(), &seed);
        verify_recovery_words_identity(spaced.as_str(), public_key)
    }

    #[test]
    fn wrong_word_count_is_rejected() {
        let twelve_words = concat!(
            "abandon abandon abandon abandon abandon abandon ",
            "abandon abandon abandon abandon abandon about"
        );

        assert!(matches!(
            signing_seed_from_recovery_words(twelve_words),
            Err(RecoveryWordsError::InvalidWordCount)
        ));
    }

    #[test]
    fn missing_word_is_rejected() {
        let missing_word = concat!(
            "abandon abandon abandon abandon abandon abandon ",
            "abandon abandon abandon abandon abandon abandon ",
            "abandon abandon abandon abandon abandon abandon ",
            "abandon abandon abandon abandon art"
        );

        assert!(matches!(
            signing_seed_from_recovery_words(missing_word),
            Err(RecoveryWordsError::InvalidWordCount)
        ));
    }

    #[test]
    fn extra_word_is_rejected() {
        let extra_word = format!("{ZERO_ENTROPY_24_WORD_VECTOR} abandon");

        assert!(matches!(
            signing_seed_from_recovery_words(extra_word.as_str()),
            Err(RecoveryWordsError::InvalidWordCount)
        ));
    }

    #[test]
    fn unknown_word_is_rejected() {
        let unknown_word = ZERO_ENTROPY_24_WORD_VECTOR.replacen("art", "notaword", 1);

        assert!(matches!(
            signing_seed_from_recovery_words(unknown_word.as_str()),
            Err(RecoveryWordsError::InvalidMnemonic)
        ));
    }

    #[test]
    fn checksum_corruption_is_rejected() {
        let corrupted_checksum = ZERO_ENTROPY_24_WORD_VECTOR.replacen("art", "abandon", 1);

        assert!(matches!(
            signing_seed_from_recovery_words(corrupted_checksum.as_str()),
            Err(RecoveryWordsError::InvalidMnemonic)
        ));
    }

    #[test]
    fn wrong_public_key_is_rejected() {
        let seed = [0x33_u8; RECOVERY_ENTROPY_LEN];
        let other_seed = [0x44_u8; RECOVERY_ENTROPY_LEN];
        let wrong_public_key = public_key_for_seed(&other_seed);

        assert!(matches!(
            recovery_words_from_signing_seed(&seed, wrong_public_key),
            Err(RecoveryWordsError::PublicKeyMismatch)
        ));
    }

    #[test]
    fn words_for_one_seed_cannot_verify_as_another_wallet() -> Result<(), RecoveryWordsError> {
        let seed = [0x11_u8; RECOVERY_ENTROPY_LEN];
        let other_seed = [0x22_u8; RECOVERY_ENTROPY_LEN];

        let public_key = public_key_for_seed(&seed);
        let other_public_key = public_key_for_seed(&other_seed);

        let recovery_words = recovery_words_from_signing_seed(&seed, public_key)?;

        assert!(matches!(
            verify_recovery_words_identity(recovery_words.words(), other_public_key),
            Err(RecoveryWordsError::PublicKeyMismatch)
        ));

        Ok(())
    }
}
