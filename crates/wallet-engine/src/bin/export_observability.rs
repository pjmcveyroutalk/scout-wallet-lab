use std::{env, error::Error, fs};

use wallet_engine::{observability::WalletObservabilitySnapshot, LockedVault, SecretPassphrase};

const VAULT_PATH_ENV: &str = "SCOUT_WALLET_VAULT_PATH";
const PASSPHRASE_ENV: &str = "SCOUT_WALLET_PASSPHRASE";
const SNAPSHOT_PATH: &str = "dashboard/wallet-observability.json";

fn main() -> Result<(), Box<dyn Error>> {
    let vault_path = env::var(VAULT_PATH_ENV)?;
    let passphrase_value = env::var(PASSPHRASE_ENV)?;

    env::remove_var(PASSPHRASE_ENV);

    let passphrase = SecretPassphrase::new(passphrase_value);
    let encoded_vault = fs::read_to_string(vault_path)?;
    let vault = LockedVault::from_json(&encoded_vault)?;
    let wallet = vault.unlock(&passphrase)?;
    let account = wallet.devnet_account();

    let snapshot = WalletObservabilitySnapshot::capture(&wallet, &account, None);
    let encoded_snapshot = serde_json::to_string_pretty(&snapshot)?;

    fs::write(SNAPSHOT_PATH, format!("{encoded_snapshot}\n"))?;

    Ok(())
}
