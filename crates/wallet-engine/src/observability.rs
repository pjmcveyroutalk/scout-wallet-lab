use crate::{DevnetAccount, SignerState, TransactionLedgerEntry, TransactionState, UnlockedWallet};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WalletObservabilitySnapshot {
    engine: &'static str,
    cluster: &'static str,
    address: String,
    lamports: Option<u64>,
    signer: SignerObservation,
    transaction: Option<TransactionObservation>,
}

impl WalletObservabilitySnapshot {
    #[must_use]
    pub fn capture(
        wallet: &UnlockedWallet,
        account: &DevnetAccount,
        transaction: Option<&TransactionLedgerEntry>,
    ) -> Self {
        Self {
            engine: crate::engine_name(),
            cluster: account.cluster().rpc_name(),
            address: account.address().to_string(),
            lamports: account.lamports(),
            signer: SignerObservation::from_state(wallet.signer_state()),
            transaction: transaction.map(TransactionObservation::capture),
        }
    }

    #[must_use]
    pub const fn engine(&self) -> &'static str {
        self.engine
    }

    #[must_use]
    pub const fn cluster(&self) -> &'static str {
        self.cluster
    }

    #[must_use]
    pub fn address(&self) -> &str {
        self.address.as_str()
    }

    #[must_use]
    pub const fn lamports(&self) -> Option<u64> {
        self.lamports
    }

    #[must_use]
    pub const fn signer(&self) -> &SignerObservation {
        &self.signer
    }

    #[must_use]
    pub const fn transaction(&self) -> Option<&TransactionObservation> {
        self.transaction.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SignerObservation {
    state: &'static str,
    signing_locked: bool,
}

impl SignerObservation {
    const fn from_state(state: SignerState) -> Self {
        match state {
            SignerState::Running => Self {
                state: "running",
                signing_locked: false,
            },
            SignerState::Locked => Self {
                state: "locked",
                signing_locked: true,
            },
        }
    }

    #[must_use]
    pub const fn state(&self) -> &'static str {
        self.state
    }

    #[must_use]
    pub const fn signing_locked(&self) -> bool {
        self.signing_locked
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransactionObservation {
    state: &'static str,
    reserved_lamports: u64,
    last_valid_block_height: u64,
    signature_present: bool,
    capital_reserved: bool,
    retry_delivery_allowed_at_last_valid_height: bool,
    rebuild_economic_intent_allowed: bool,
}

impl TransactionObservation {
    fn capture(entry: &TransactionLedgerEntry) -> Self {
        Self {
            state: transaction_state_name(entry.state()),
            reserved_lamports: entry.reserved_lamports(),
            last_valid_block_height: entry.last_valid_block_height(),
            signature_present: entry.signature().is_some(),
            capital_reserved: entry.capital_is_reserved(),
            retry_delivery_allowed_at_last_valid_height: entry
                .can_retry_delivery(entry.last_valid_block_height()),
            rebuild_economic_intent_allowed: entry.can_rebuild_economic_intent(),
        }
    }

    #[must_use]
    pub const fn state(&self) -> &'static str {
        self.state
    }

    #[must_use]
    pub const fn reserved_lamports(&self) -> u64 {
        self.reserved_lamports
    }

    #[must_use]
    pub const fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    #[must_use]
    pub const fn signature_present(&self) -> bool {
        self.signature_present
    }

    #[must_use]
    pub const fn capital_reserved(&self) -> bool {
        self.capital_reserved
    }

    #[must_use]
    pub const fn retry_delivery_allowed_at_last_valid_height(&self) -> bool {
        self.retry_delivery_allowed_at_last_valid_height
    }

    #[must_use]
    pub const fn rebuild_economic_intent_allowed(&self) -> bool {
        self.rebuild_economic_intent_allowed
    }
}

const fn transaction_state_name(state: TransactionState) -> &'static str {
    match state {
        TransactionState::Reserved => "reserved",
        TransactionState::Signed => "signed",
        TransactionState::Submitted => "submitted",
        TransactionState::Confirmed => "confirmed",
        TransactionState::Failed => "failed",
        TransactionState::Ambiguous => "ambiguous",
        TransactionState::Quarantined => "quarantined",
        TransactionState::Settled => "settled",
        TransactionState::Released => "released",
    }
}

#[cfg(test)]
mod tests {
    use super::WalletObservabilitySnapshot;
    use crate::{
        BlockhashLease, Cluster, LockedVault, SecretPassphrase, SecretSeed, TransactionLedgerEntry,
        VaultError,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use solana_hash::Hash;
    use solana_pubkey::Pubkey;

    fn test_lease(
        blockhash: Hash,
        last_valid_block_height: u64,
        observed_block_height: u64,
    ) -> BlockhashLease {
        BlockhashLease::new_for_test(blockhash, last_valid_block_height, observed_block_height)
    }

    #[test]
    fn snapshot_exposes_only_safe_wallet_metadata() -> Result<(), VaultError> {
        let passphrase_text = "observability secret passphrase";
        let passphrase = SecretPassphrase::new(passphrase_text.to_owned());
        let seed = [171_u8; 32];

        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new(seed))?;
        let wallet = vault.unlock(&passphrase)?;
        let mut account = wallet.devnet_account();

        account.record_balance(1_250_000_000);

        let blockhash = Hash::new_from_array([172_u8; 32]);
        let lease = test_lease(blockhash, 2_000, 1_990);

        let transaction = TransactionLedgerEntry::reserve(400_000_000, lease)
            .map_err(|_| VaultError::SerializationFailed)?;

        let snapshot = WalletObservabilitySnapshot::capture(&wallet, &account, Some(&transaction));

        let encoded =
            serde_json::to_string(&snapshot).map_err(|_| VaultError::SerializationFailed)?;

        assert_eq!(snapshot.engine(), "scout-wallet-lab");
        assert_eq!(snapshot.cluster(), "devnet");
        assert_eq!(
            snapshot.address(),
            Pubkey::new_from_array(wallet.public_key()).to_string()
        );
        assert_eq!(snapshot.lamports(), Some(1_250_000_000));

        assert_eq!(snapshot.signer().state(), "running");
        assert!(!snapshot.signer().signing_locked());

        let observed_transaction = snapshot
            .transaction()
            .ok_or(VaultError::SerializationFailed)?;

        assert_eq!(observed_transaction.state(), "reserved");
        assert_eq!(observed_transaction.reserved_lamports(), 400_000_000);
        assert_eq!(observed_transaction.last_valid_block_height(), 2_000);
        assert!(!observed_transaction.signature_present());
        assert!(observed_transaction.capital_reserved());
        assert!(!observed_transaction.retry_delivery_allowed_at_last_valid_height());
        assert!(!observed_transaction.rebuild_economic_intent_allowed());

        assert!(!encoded.contains(passphrase_text));
        assert!(!encoded.contains("ciphertext"));
        assert!(!encoded.contains("seed"));
        assert!(!encoded.contains("private"));
        assert!(!encoded.contains(&BASE64.encode(seed)));

        Ok(())
    }

    #[test]
    fn emergency_lock_is_visible_without_exposing_signing_material() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("observability lock test".to_owned());

        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([174_u8; 32]))?;

        let mut wallet = vault.unlock(&passphrase)?;
        let account = wallet.devnet_account();

        wallet.emergency_lock();

        let snapshot = WalletObservabilitySnapshot::capture(&wallet, &account, None);

        assert_eq!(snapshot.signer().state(), "locked");
        assert!(snapshot.signer().signing_locked());
        assert!(snapshot.transaction().is_none());

        let encoded =
            serde_json::to_string(&snapshot).map_err(|_| VaultError::SerializationFailed)?;

        assert!(!encoded.contains("observability lock test"));
        assert!(!encoded.contains("ciphertext"));
        assert!(!encoded.contains("seed"));
        assert!(!encoded.contains("private"));

        Ok(())
    }

    #[test]
    fn snapshot_uses_devnet_identity_only() -> Result<(), VaultError> {
        let passphrase = SecretPassphrase::new("observability cluster test".to_owned());

        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([175_u8; 32]))?;

        let wallet = vault.unlock(&passphrase)?;
        let account = wallet.devnet_account();

        let snapshot = WalletObservabilitySnapshot::capture(&wallet, &account, None);

        assert_eq!(snapshot.cluster(), Cluster::Devnet.rpc_name());
        assert_eq!(snapshot.cluster(), "devnet");

        Ok(())
    }
}
