use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use wallet_engine::{observability::WalletObservabilitySnapshot, LockedVault, SecretPassphrase};

const VAULT_PATH_ENV: &str = "SCOUT_WALLET_VAULT_PATH";
const PASSPHRASE_ENV: &str = "SCOUT_WALLET_PASSPHRASE";
const SNAPSHOT_PATH: &str = "dashboard/wallet-observability.json";

fn main() -> Result<(), Box<dyn Error>> {
    let vault_path =
        env::var_os(VAULT_PATH_ENV).ok_or_else(|| format!("{VAULT_PATH_ENV} is required"))?;

    let passphrase_value =
        env::var(PASSPHRASE_ENV).map_err(|_| format!("{PASSPHRASE_ENV} is required"))?;

    env::remove_var(PASSPHRASE_ENV);

    let passphrase = SecretPassphrase::new(passphrase_value);
    let encoded_vault = fs::read_to_string(vault_path)?;
    let vault = LockedVault::from_json(&encoded_vault)?;
    let wallet = vault.unlock(&passphrase)?;
    let account = wallet.devnet_account();

    let snapshot = WalletObservabilitySnapshot::capture(&wallet, &account, None);
    let encoded_snapshot = serde_json::to_string_pretty(&snapshot)?;

    write_snapshot_atomically(Path::new(SNAPSHOT_PATH), &encoded_snapshot)?;

    Ok(())
}

fn write_snapshot_atomically(path: &Path, snapshot: &str) -> Result<(), Box<dyn Error>> {
    let parent = path
        .parent()
        .ok_or_else(|| String::from("snapshot path must have a parent directory"))?;

    fs::create_dir_all(parent)?;

    let temporary_path = temporary_snapshot_path(path)?;

    let write_result = (|| -> Result<(), Box<dyn Error>> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;

        file.write_all(snapshot.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;

        fs::rename(&temporary_path, path)?;

        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    write_result
}

fn temporary_snapshot_path(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| String::from("snapshot path must end in a UTF-8 file name"))?;

    Ok(path.with_file_name(format!(".{file_name}.tmp")))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{temporary_snapshot_path, write_snapshot_atomically};

    fn unique_test_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();

        Ok(std::env::temp_dir()
            .join(format!("scout-wallet-observability-{nonce}"))
            .join("wallet-observability.json"))
    }

    #[test]
    fn atomic_export_replaces_snapshot_without_leaving_temporary_file(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = unique_test_path()?;
        let temporary_path = temporary_snapshot_path(&path)?;

        write_snapshot_atomically(&path, "{\"engine\":\"scout-wallet-lab\"}")?;

        let written = fs::read_to_string(&path)?;

        assert_eq!(written, "{\"engine\":\"scout-wallet-lab\"}\n");
        assert!(!temporary_path.exists());

        if let Some(parent) = path.parent() {
            fs::remove_dir_all(parent)?;
        }

        Ok(())
    }

    #[test]
    fn temporary_snapshot_stays_next_to_final_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from("dashboard/wallet-observability.json");
        let temporary_path = temporary_snapshot_path(&path)?;

        assert_eq!(
            temporary_path,
            PathBuf::from("dashboard/.wallet-observability.json.tmp")
        );

        Ok(())
    }
}
