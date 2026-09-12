use crate::{
    BlockhashLease, ExecutionPolicy, LedgerError, LockedVault, PolicyError, PreparedTransaction,
    SecretPassphrase, SignatureBytes, SignerError, TransactionState, UnlockedWallet, VaultError,
};
use solana_hash::Hash;
use solana_instruction::Instruction;
use solana_message::Message;
use solana_pubkey::Pubkey;
use std::{fmt, str::FromStr};

const STAGE_C_PROOF_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const STAGE_C_PROOF_PAYLOAD: &[u8] = b"scout-stage-c-devnet-signing-proof-v1";
const STAGE_C_PROOF_RESERVED_LAMPORTS: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevnetSigningCoordinatorError {
    Vault(VaultError),
    Policy(PolicyError),
    Signer(SignerError),
    Ledger(LedgerError),
    InvalidCanonicalMessage,
    WalletIdentityMismatch,
}

impl fmt::Display for DevnetSigningCoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Vault(_) => "Devnet signing vault gate failed",
            Self::Policy(_) => "Devnet signing policy gate failed",
            Self::Signer(_) => "Devnet signer gate failed",
            Self::Ledger(_) => "Devnet transaction lifecycle gate failed",
            Self::InvalidCanonicalMessage => "Devnet canonical transaction message is invalid",
            Self::WalletIdentityMismatch => "Devnet transaction does not belong to this wallet",
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for DevnetSigningCoordinatorError {}

impl From<VaultError> for DevnetSigningCoordinatorError {
    fn from(error: VaultError) -> Self {
        Self::Vault(error)
    }
}

impl From<PolicyError> for DevnetSigningCoordinatorError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
    }
}

impl From<SignerError> for DevnetSigningCoordinatorError {
    fn from(error: SignerError) -> Self {
        Self::Signer(error)
    }
}

impl From<LedgerError> for DevnetSigningCoordinatorError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SignedDevnetTransactionMetadata {
    public_key: Pubkey,
    signature: SignatureBytes,
    recent_blockhash: Hash,
    reserved_lamports: u64,
}

impl SignedDevnetTransactionMetadata {
    #[must_use]
    pub const fn public_key(&self) -> Pubkey {
        self.public_key
    }

    #[must_use]
    pub const fn signature(&self) -> SignatureBytes {
        self.signature
    }

    #[must_use]
    pub const fn recent_blockhash(&self) -> Hash {
        self.recent_blockhash
    }

    #[must_use]
    pub const fn reserved_lamports(&self) -> u64 {
        self.reserved_lamports
    }
}

pub struct DevnetSigningCoordinator {
    wallet: UnlockedWallet,
    public_key: Pubkey,
}

impl DevnetSigningCoordinator {
    pub fn unlock_vault_json(
        vault_json: &str,
        passphrase: SecretPassphrase,
    ) -> Result<Self, DevnetSigningCoordinatorError> {
        let vault = LockedVault::from_json(vault_json)?;
        let expected_public_key = Pubkey::new_from_array(vault.public_key()?);
        let wallet = vault.unlock(&passphrase)?;
        let actual_public_key = Pubkey::new_from_array(wallet.public_key());

        if actual_public_key != expected_public_key {
            return Err(DevnetSigningCoordinatorError::WalletIdentityMismatch);
        }

        Ok(Self {
            wallet,
            public_key: actual_public_key,
        })
    }

    #[must_use]
    pub const fn public_key(&self) -> Pubkey {
        self.public_key
    }

    #[must_use]
    pub const fn signing_is_locked(&self) -> bool {
        self.wallet.signing_is_locked()
    }

    pub fn emergency_lock(&mut self) {
        self.wallet.emergency_lock();
    }

    pub fn sign_stage_c_proof(
        &self,
        lease: BlockhashLease,
    ) -> Result<SignedDevnetTransactionMetadata, DevnetSigningCoordinatorError> {
        let current_block_height = lease.observed_block_height();
        let instruction = stage_c_proof_instruction()?;
        let program_id = instruction.program_id;
        let policy = ExecutionPolicy::new(STAGE_C_PROOF_RESERVED_LAMPORTS, &[program_id])?;
        let mut transaction = PreparedTransaction::reserve(
            &[instruction],
            self.public_key,
            STAGE_C_PROOF_RESERVED_LAMPORTS,
            lease,
        )
        .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;

        self.sign_prepared_transaction(&mut transaction, &policy, current_block_height)
    }

    pub fn sign_prepared_transaction(
        &self,
        transaction: &mut PreparedTransaction,
        policy: &ExecutionPolicy,
        current_block_height: u64,
    ) -> Result<SignedDevnetTransactionMetadata, DevnetSigningCoordinatorError> {
        let message: Message = bincode::deserialize(transaction.message().bytes())
            .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;
        let payer = message
            .account_keys
            .first()
            .copied()
            .ok_or(DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;

        if payer != self.public_key {
            return Err(DevnetSigningCoordinatorError::WalletIdentityMismatch);
        }

        if message.recent_blockhash != transaction.ledger().recent_blockhash() {
            return Err(DevnetSigningCoordinatorError::InvalidCanonicalMessage);
        }

        let signature = {
            let authorized = policy.authorize(transaction, current_block_height)?;
            self.wallet.sign_transaction_message(&authorized)?
        };

        transaction.ledger_mut().mark_signed(signature)?;

        if transaction.ledger().state() != TransactionState::Signed {
            return Err(DevnetSigningCoordinatorError::Ledger(
                LedgerError::InvalidTransition,
            ));
        }

        Ok(SignedDevnetTransactionMetadata {
            public_key: self.public_key,
            signature,
            recent_blockhash: transaction.ledger().recent_blockhash(),
            reserved_lamports: transaction.ledger().reserved_lamports(),
        })
    }
}

fn stage_c_proof_instruction() -> Result<Instruction, DevnetSigningCoordinatorError> {
    let program_id = Pubkey::from_str(STAGE_C_PROOF_PROGRAM_ID)
        .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;

    Ok(Instruction {
        program_id,
        accounts: Vec::new(),
        data: STAGE_C_PROOF_PAYLOAD.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        stage_c_proof_instruction, DevnetSigningCoordinator, DevnetSigningCoordinatorError,
        STAGE_C_PROOF_PAYLOAD, STAGE_C_PROOF_PROGRAM_ID, STAGE_C_PROOF_RESERVED_LAMPORTS,
    };
    use crate::{
        BlockhashLease, CanonicalTransactionMessage, ExecutionPolicy, LockedVault, PolicyError,
        PreparedTransaction, SecretPassphrase, SecretSeed, SignerError, TransactionState,
        VaultError,
    };
    use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
    use solana_hash::Hash;
    use solana_instruction::Instruction;
    use solana_pubkey::Pubkey;
    use std::str::FromStr;

    fn prepared_transaction(
        payer: Pubkey,
        program_id: Pubkey,
        reserved_lamports: u64,
        last_valid_block_height: u64,
    ) -> Result<PreparedTransaction, DevnetSigningCoordinatorError> {
        let lease = BlockhashLease::new_for_test(
            Hash::new_from_array([0x71_u8; 32]),
            last_valid_block_height,
            last_valid_block_height.saturating_sub(1),
        );
        let instruction = Instruction {
            program_id,
            accounts: Vec::new(),
            data: vec![0x01],
        };

        PreparedTransaction::reserve(&[instruction], payer, reserved_lamports, lease)
            .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)
    }

    fn coordinator_fixture(
        seed_byte: u8,
    ) -> Result<(DevnetSigningCoordinator, Pubkey), DevnetSigningCoordinatorError> {
        let passphrase = SecretPassphrase::new("stage-b-devnet-signing".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([seed_byte; 32]))?;
        let vault_json = vault.to_json()?;
        let coordinator = DevnetSigningCoordinator::unlock_vault_json(
            &vault_json,
            SecretPassphrase::new("stage-b-devnet-signing".to_owned()),
        )?;
        let public_key = coordinator.public_key();

        Ok((coordinator, public_key))
    }

    #[test]
    fn successful_policy_authorized_signing_moves_ledger_once(
    ) -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, payer) = coordinator_fixture(0x41)?;
        let program_id = Pubkey::new_from_array([0x51_u8; 32]);
        let policy = ExecutionPolicy::new(10_000, &[program_id])?;
        let mut transaction = prepared_transaction(payer, program_id, 4_000, 500)?;
        let message_bytes = transaction.message().bytes().to_vec();

        let signed = coordinator.sign_prepared_transaction(&mut transaction, &policy, 499)?;

        assert_eq!(transaction.ledger().state(), TransactionState::Signed);
        assert_eq!(signed.public_key(), payer);
        assert_eq!(signed.reserved_lamports(), 4_000);
        assert_eq!(
            signed.recent_blockhash(),
            transaction.ledger().recent_blockhash()
        );

        let verifying_key = VerifyingKey::from_bytes(&payer.to_bytes())
            .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;
        let signature = Signature::from_bytes(&signed.signature().to_bytes());
        assert!(verifying_key.verify(&message_bytes, &signature).is_ok());

        assert!(matches!(
            coordinator.sign_prepared_transaction(&mut transaction, &policy, 499),
            Err(DevnetSigningCoordinatorError::Policy(
                PolicyError::TransactionNotReserved
            ))
        ));

        Ok(())
    }

    #[test]
    fn stage_c_proof_request_is_fixed_and_signature_verifies(
    ) -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, payer) = coordinator_fixture(0x49)?;
        let blockhash = Hash::new_from_array([0x72_u8; 32]);
        let lease = BlockhashLease::new_for_test(blockhash, 700, 699);

        let signed = coordinator.sign_stage_c_proof(lease)?;
        let instruction = stage_c_proof_instruction()?;
        let canonical =
            CanonicalTransactionMessage::new(std::slice::from_ref(&instruction), payer, blockhash)
                .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;

        assert_eq!(instruction.program_id.to_string(), STAGE_C_PROOF_PROGRAM_ID);
        assert_eq!(instruction.data, STAGE_C_PROOF_PAYLOAD);
        assert!(instruction.accounts.is_empty());
        assert_eq!(signed.public_key(), payer);
        assert_eq!(signed.recent_blockhash(), blockhash);
        assert_eq!(signed.reserved_lamports(), STAGE_C_PROOF_RESERVED_LAMPORTS);

        let verifying_key = VerifyingKey::from_bytes(&payer.to_bytes())
            .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;
        let signature = Signature::from_bytes(&signed.signature().to_bytes());
        assert!(verifying_key.verify(canonical.bytes(), &signature).is_ok());

        Ok(())
    }

    #[test]
    fn stage_c_proof_rejects_expired_blockhash() -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, _) = coordinator_fixture(0x50)?;
        let lease = BlockhashLease::new_for_test(Hash::new_from_array([0x73_u8; 32]), 800, 801);

        assert!(matches!(
            coordinator.sign_stage_c_proof(lease),
            Err(DevnetSigningCoordinatorError::Policy(
                PolicyError::BlockhashExpired
            ))
        ));
        Ok(())
    }

    #[test]
    fn stage_c_proof_program_id_is_canonical() -> Result<(), DevnetSigningCoordinatorError> {
        let expected = Pubkey::from_str(STAGE_C_PROOF_PROGRAM_ID)
            .map_err(|_| DevnetSigningCoordinatorError::InvalidCanonicalMessage)?;
        let instruction = stage_c_proof_instruction()?;

        assert_eq!(instruction.program_id, expected);
        Ok(())
    }

    #[test]
    fn wrong_passphrase_fails_before_signing() -> Result<(), DevnetSigningCoordinatorError> {
        let passphrase = SecretPassphrase::new("correct-stage-b-passphrase".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([0x42_u8; 32]))?;
        let vault_json = vault.to_json()?;
        let result = DevnetSigningCoordinator::unlock_vault_json(
            &vault_json,
            SecretPassphrase::new("wrong-stage-b-passphrase".to_owned()),
        );

        assert!(matches!(
            result,
            Err(DevnetSigningCoordinatorError::Vault(
                VaultError::DecryptionFailed
            ))
        ));
        Ok(())
    }

    #[test]
    fn malformed_vault_fails_before_signing() {
        let result = DevnetSigningCoordinator::unlock_vault_json(
            "{not-valid-json",
            SecretPassphrase::new("irrelevant".to_owned()),
        );

        assert!(matches!(
            result,
            Err(DevnetSigningCoordinatorError::Vault(
                VaultError::SerializationFailed
            ))
        ));
    }

    #[test]
    fn transaction_payer_must_match_unlocked_wallet() -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, _) = coordinator_fixture(0x43)?;
        let other_payer = Pubkey::new_from_array([0x44_u8; 32]);
        let program_id = Pubkey::new_from_array([0x52_u8; 32]);
        let policy = ExecutionPolicy::new(10_000, &[program_id])?;
        let mut transaction = prepared_transaction(other_payer, program_id, 4_000, 500)?;

        assert!(matches!(
            coordinator.sign_prepared_transaction(&mut transaction, &policy, 499),
            Err(DevnetSigningCoordinatorError::WalletIdentityMismatch)
        ));
        assert_eq!(transaction.ledger().state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn exposure_limit_is_enforced_before_signing() -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, payer) = coordinator_fixture(0x45)?;
        let program_id = Pubkey::new_from_array([0x53_u8; 32]);
        let policy = ExecutionPolicy::new(1_000, &[program_id])?;
        let mut transaction = prepared_transaction(payer, program_id, 1_001, 500)?;

        assert!(matches!(
            coordinator.sign_prepared_transaction(&mut transaction, &policy, 499),
            Err(DevnetSigningCoordinatorError::Policy(
                PolicyError::ExposureExceeded
            ))
        ));
        assert_eq!(transaction.ledger().state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn program_allowlist_is_enforced_before_signing() -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, payer) = coordinator_fixture(0x46)?;
        let allowed_program = Pubkey::new_from_array([0x54_u8; 32]);
        let disallowed_program = Pubkey::new_from_array([0x55_u8; 32]);
        let policy = ExecutionPolicy::new(10_000, &[allowed_program])?;
        let mut transaction = prepared_transaction(payer, disallowed_program, 1_000, 500)?;

        assert!(matches!(
            coordinator.sign_prepared_transaction(&mut transaction, &policy, 499),
            Err(DevnetSigningCoordinatorError::Policy(
                PolicyError::ProgramNotAllowed
            ))
        ));
        assert_eq!(transaction.ledger().state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn expired_blockhash_is_rejected_before_signing() -> Result<(), DevnetSigningCoordinatorError> {
        let (coordinator, payer) = coordinator_fixture(0x47)?;
        let program_id = Pubkey::new_from_array([0x56_u8; 32]);
        let policy = ExecutionPolicy::new(10_000, &[program_id])?;
        let mut transaction = prepared_transaction(payer, program_id, 1_000, 500)?;

        assert!(matches!(
            coordinator.sign_prepared_transaction(&mut transaction, &policy, 501),
            Err(DevnetSigningCoordinatorError::Policy(
                PolicyError::BlockhashExpired
            ))
        ));
        assert_eq!(transaction.ledger().state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn emergency_lock_refuses_policy_authorized_signing(
    ) -> Result<(), DevnetSigningCoordinatorError> {
        let (mut coordinator, payer) = coordinator_fixture(0x48)?;
        let program_id = Pubkey::new_from_array([0x57_u8; 32]);
        let policy = ExecutionPolicy::new(10_000, &[program_id])?;
        let mut transaction = prepared_transaction(payer, program_id, 1_000, 500)?;

        coordinator.emergency_lock();
        assert!(coordinator.signing_is_locked());

        assert!(matches!(
            coordinator.sign_prepared_transaction(&mut transaction, &policy, 499),
            Err(DevnetSigningCoordinatorError::Signer(SignerError::Locked))
        ));
        assert_eq!(transaction.ledger().state(), TransactionState::Reserved);
        Ok(())
    }

    #[test]
    fn invalid_policies_are_rejected_before_a_coordinator_can_sign() {
        let program_id = Pubkey::new_from_array([0x58_u8; 32]);

        assert!(matches!(
            ExecutionPolicy::new(0, &[program_id]),
            Err(PolicyError::ZeroExposureLimit)
        ));
        assert!(matches!(
            ExecutionPolicy::new(1, &[]),
            Err(PolicyError::NoAllowedPrograms)
        ));
    }
}
