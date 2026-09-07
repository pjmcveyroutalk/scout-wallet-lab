use serde::{Deserialize, Serialize};
use std::fmt;
use wallet_engine::{Cluster, LockedVault};

const BACKUP_FORMAT_VERSION: u8 = 1;
const BACKUP_KIND: &str = "scout-locked-vault-backup";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedVaultBackupPackage {
    format_version: u8,
    kind: String,
    cluster: String,
    public_address: String,
    locked_vault_json: String,
}

impl LockedVaultBackupPackage {
    #[must_use]
    pub const fn format_version(&self) -> u8 {
        self.format_version
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        self.kind.as_str()
    }

    #[must_use]
    pub fn cluster(&self) -> &str {
        self.cluster.as_str()
    }

    #[must_use]
    pub fn public_address(&self) -> &str {
        self.public_address.as_str()
    }

    #[must_use]
    pub fn locked_vault_json(&self) -> &str {
        self.locked_vault_json.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedLockedVaultBackup {
    public_address: String,
    locked_vault_json: String,
}

impl ValidatedLockedVaultBackup {
    #[must_use]
    pub fn public_address(&self) -> &str {
        self.public_address.as_str()
    }

    #[must_use]
    pub fn locked_vault_json(&self) -> &str {
        self.locked_vault_json.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryBackupError {
    EmptyVaultJson,
    VaultParseFailed,
    AddressDerivationFailed,
    BackupSerializationFailed,
    EmptyBackupJson,
    BackupParseFailed,
    UnsupportedBackupVersion,
    InvalidBackupKind,
    InvalidCluster,
    EmptyPublicAddress,
    EmptyEmbeddedVault,
    AddressMismatch,
}

impl fmt::Display for RecoveryBackupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyVaultJson => "encrypted vault JSON is empty",
            Self::VaultParseFailed => "encrypted vault JSON could not be parsed",
            Self::AddressDerivationFailed => "Devnet public address could not be derived",
            Self::BackupSerializationFailed => "backup package could not be serialized",
            Self::EmptyBackupJson => "backup package JSON is empty",
            Self::BackupParseFailed => "backup package JSON could not be parsed",
            Self::UnsupportedBackupVersion => "backup package version is unsupported",
            Self::InvalidBackupKind => "backup package kind is invalid",
            Self::InvalidCluster => "backup package cluster is invalid",
            Self::EmptyPublicAddress => "backup package public address is empty",
            Self::EmptyEmbeddedVault => "backup package encrypted vault is empty",
            Self::AddressMismatch => {
                "backup package public address does not match encrypted vault identity"
            }
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for RecoveryBackupError {}

pub fn create_locked_vault_backup(
    vault_json: &str,
) -> Result<String, RecoveryBackupError> {
    let vault_json = vault_json.trim();

    if vault_json.is_empty() {
        return Err(RecoveryBackupError::EmptyVaultJson);
    }

    let vault =
        LockedVault::from_json(vault_json).map_err(|_| RecoveryBackupError::VaultParseFailed)?;

    let account = vault
        .devnet_account()
        .map_err(|_| RecoveryBackupError::AddressDerivationFailed)?;

    let package = LockedVaultBackupPackage {
        format_version: BACKUP_FORMAT_VERSION,
        kind: BACKUP_KIND.to_owned(),
        cluster: Cluster::Devnet.rpc_name().to_owned(),
        public_address: account.address().to_string(),
        locked_vault_json: vault_json.to_owned(),
    };

    serde_json::to_string_pretty(&package)
        .map_err(|_| RecoveryBackupError::BackupSerializationFailed)
}

pub fn validate_locked_vault_backup(
    backup_json: &str,
) -> Result<ValidatedLockedVaultBackup, RecoveryBackupError> {
    let backup_json = backup_json.trim();

    if backup_json.is_empty() {
        return Err(RecoveryBackupError::EmptyBackupJson);
    }

    let package: LockedVaultBackupPackage =
        serde_json::from_str(backup_json).map_err(|_| RecoveryBackupError::BackupParseFailed)?;

    validate_package(package)
}

fn validate_package(
    package: LockedVaultBackupPackage,
) -> Result<ValidatedLockedVaultBackup, RecoveryBackupError> {
    if package.format_version != BACKUP_FORMAT_VERSION {
        return Err(RecoveryBackupError::UnsupportedBackupVersion);
    }

    if package.kind != BACKUP_KIND {
        return Err(RecoveryBackupError::InvalidBackupKind);
    }

    if package.cluster != Cluster::Devnet.rpc_name() {
        return Err(RecoveryBackupError::InvalidCluster);
    }

    if package.public_address.trim().is_empty() {
        return Err(RecoveryBackupError::EmptyPublicAddress);
    }

    let locked_vault_json = package.locked_vault_json.trim();

    if locked_vault_json.is_empty() {
        return Err(RecoveryBackupError::EmptyEmbeddedVault);
    }

    let vault = LockedVault::from_json(locked_vault_json)
        .map_err(|_| RecoveryBackupError::VaultParseFailed)?;

    let account = vault
        .devnet_account()
        .map_err(|_| RecoveryBackupError::AddressDerivationFailed)?;

    let derived_address = account.address().to_string();

    if derived_address != package.public_address {
        return Err(RecoveryBackupError::AddressMismatch);
    }

    Ok(ValidatedLockedVaultBackup {
        public_address: derived_address,
        locked_vault_json: locked_vault_json.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet_engine::SecretPassphrase;

    fn test_vault_json() -> Result<String, Box<dyn std::error::Error>> {
        let passphrase =
            SecretPassphrase::new("recovery-backup-test-passphrase".to_owned());

        let vault = LockedVault::generate(&passphrase)?;

        Ok(vault.to_json()?)
    }

    #[test]
    fn rejects_empty_vault_json() {
        let result = create_locked_vault_backup("   ");

        assert_eq!(result, Err(RecoveryBackupError::EmptyVaultJson));
    }

    #[test]
    fn creates_devnet_locked_vault_backup() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;

        let backup_json = create_locked_vault_backup(&vault_json)?;

        let package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        assert_eq!(package.format_version(), BACKUP_FORMAT_VERSION);
        assert_eq!(package.kind(), BACKUP_KIND);
        assert_eq!(package.cluster(), "devnet");
        assert!(!package.public_address().is_empty());
        assert_eq!(package.locked_vault_json(), vault_json);

        Ok(())
    }

    #[test]
    fn backup_round_trip_preserves_locked_vault_identity(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;

        let original_vault = LockedVault::from_json(&vault_json)?;
        let original_address =
            original_vault.devnet_account()?.address().to_string();

        let backup_json = create_locked_vault_backup(&vault_json)?;
        let validated = validate_locked_vault_backup(&backup_json)?;

        assert_eq!(validated.public_address(), original_address);
        assert_eq!(validated.locked_vault_json(), vault_json);

        let restored_vault =
            LockedVault::from_json(validated.locked_vault_json())?;

        assert_eq!(
            restored_vault.devnet_account()?.address().to_string(),
            original_address
        );

        Ok(())
    }

    #[test]
    fn rejects_empty_backup_json() {
        let result = validate_locked_vault_backup("   ");

        assert_eq!(result, Err(RecoveryBackupError::EmptyBackupJson));
    }

    #[test]
    fn rejects_invalid_backup_json() {
        let result =
            validate_locked_vault_backup("{not-valid-json}");

        assert_eq!(result, Err(RecoveryBackupError::BackupParseFailed));
    }

    #[test]
    fn rejects_wrong_backup_version() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;
        let backup_json = create_locked_vault_backup(&vault_json)?;

        let mut package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        package.format_version = BACKUP_FORMAT_VERSION + 1;

        let modified = serde_json::to_string(&package)?;
        let result = validate_locked_vault_backup(&modified);

        assert_eq!(
            result,
            Err(RecoveryBackupError::UnsupportedBackupVersion)
        );

        Ok(())
    }

    #[test]
    fn rejects_wrong_backup_kind() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;
        let backup_json = create_locked_vault_backup(&vault_json)?;

        let mut package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        package.kind = "not-scout-backup".to_owned();

        let modified = serde_json::to_string(&package)?;
        let result = validate_locked_vault_backup(&modified);

        assert_eq!(result, Err(RecoveryBackupError::InvalidBackupKind));

        Ok(())
    }

    #[test]
    fn rejects_non_devnet_cluster() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;
        let backup_json = create_locked_vault_backup(&vault_json)?;

        let mut package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        package.cluster = "mainnet-beta".to_owned();

        let modified = serde_json::to_string(&package)?;
        let result = validate_locked_vault_backup(&modified);

        assert_eq!(result, Err(RecoveryBackupError::InvalidCluster));

        Ok(())
    }

    #[test]
    fn rejects_empty_public_address() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;
        let backup_json = create_locked_vault_backup(&vault_json)?;

        let mut package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        package.public_address.clear();

        let modified = serde_json::to_string(&package)?;
        let result = validate_locked_vault_backup(&modified);

        assert_eq!(result, Err(RecoveryBackupError::EmptyPublicAddress));

        Ok(())
    }

    #[test]
    fn rejects_empty_embedded_vault() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;
        let backup_json = create_locked_vault_backup(&vault_json)?;

        let mut package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        package.locked_vault_json.clear();

        let modified = serde_json::to_string(&package)?;
        let result = validate_locked_vault_backup(&modified);

        assert_eq!(result, Err(RecoveryBackupError::EmptyEmbeddedVault));

        Ok(())
    }

    #[test]
    fn rejects_public_address_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;
        let backup_json = create_locked_vault_backup(&vault_json)?;

        let mut package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        package.public_address =
            "11111111111111111111111111111111".to_owned();

        let modified = serde_json::to_string(&package)?;
        let result = validate_locked_vault_backup(&modified);

        assert_eq!(result, Err(RecoveryBackupError::AddressMismatch));

        Ok(())
    }

    #[test]
    fn backup_contains_no_passphrase_field() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;

        let backup_json = create_locked_vault_backup(&vault_json)?;

        let value: serde_json::Value =
            serde_json::from_str(&backup_json)?;

        let object = value
            .as_object()
            .ok_or("backup package was not a JSON object")?;

        assert!(!object.contains_key("passphrase"));
        assert!(!object.contains_key("password"));
        assert!(!object.contains_key("seed"));
        assert!(!object.contains_key("private_key"));
        assert!(!object.contains_key("signing_key"));

        Ok(())
    }

    #[test]
    fn backup_is_explicitly_devnet_only() -> Result<(), Box<dyn std::error::Error>> {
        let vault_json = test_vault_json()?;

        let backup_json = create_locked_vault_backup(&vault_json)?;
        let validated = validate_locked_vault_backup(&backup_json)?;

        assert!(!validated.public_address().is_empty());

        let package: LockedVaultBackupPackage =
            serde_json::from_str(&backup_json)?;

        assert_eq!(package.cluster(), Cluster::Devnet.rpc_name());
        assert_eq!(package.cluster(), "devnet");

        Ok(())
    }
}
