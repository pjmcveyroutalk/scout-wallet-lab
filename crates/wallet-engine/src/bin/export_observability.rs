use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use rustls::crypto::CryptoProvider;
use serde_json::{json, Value};
use wallet_engine::{
    observability::WalletObservabilitySnapshot, DevnetAccount, LockedVault, SecretPassphrase,
    UnlockedWallet,
};

const VAULT_PATH_ENV: &str = "SCOUT_WALLET_VAULT_PATH";
const PASSPHRASE_ENV: &str = "SCOUT_WALLET_PASSPHRASE";
const SNAPSHOT_PATH: &str = "dashboard/wallet-observability.json";
const RPC_TIMEOUT_SECONDS: u64 = 10;
const RPC_REQUEST_ID: u64 = 1;

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
    let mut account = wallet.devnet_account();

    verify_observability_identity(&wallet, &account).map_err(io::Error::other)?;

    let lamports = refresh_devnet_balance(&mut account)?;

    if account.lamports() != Some(lamports) {
        return Err(io::Error::other(
            "Devnet balance observation was not recorded on the verified account",
        )
        .into());
    }

    verify_observability_identity(&wallet, &account).map_err(io::Error::other)?;

    let snapshot = WalletObservabilitySnapshot::capture(&wallet, &account, None);
    let encoded_snapshot = serde_json::to_string_pretty(&snapshot)?;

    write_snapshot_atomically(Path::new(SNAPSHOT_PATH), &encoded_snapshot)?;

    Ok(())
}

fn verify_observability_identity(
    wallet: &UnlockedWallet,
    account: &DevnetAccount,
) -> Result<(), &'static str> {
    if account.address().to_bytes() != wallet.public_key() {
        return Err("observability account does not match unlocked wallet identity");
    }

    Ok(())
}

fn refresh_devnet_balance(account: &mut DevnetAccount) -> Result<u64, Box<dyn Error>> {
    ensure_tls_provider()?;
    account.clear_balance();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(RPC_TIMEOUT_SECONDS))
        .build()
        .map_err(|_| io::Error::other("Devnet RPC client initialization failed"))?;

    let request = build_balance_request(account);
    let response = client
        .post(account.cluster().rpc_url())
        .json(&request)
        .send()
        .map_err(|_| io::Error::other("Devnet RPC transport failed"))?;

    if !response.status().is_success() {
        return Err(io::Error::other("Devnet RPC HTTP status rejected").into());
    }

    let response = response
        .json::<Value>()
        .map_err(|_| io::Error::other("Devnet RPC response was invalid"))?;
    let lamports = parse_balance_response(&response).map_err(io::Error::other)?;

    account.record_balance(lamports);

    Ok(lamports)
}

fn ensure_tls_provider() -> Result<(), Box<dyn Error>> {
    if CryptoProvider::get_default().is_some() {
        return Ok(());
    }

    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        return Err(io::Error::other("Devnet RPC TLS initialization failed").into());
    }

    Ok(())
}

fn build_balance_request(account: &DevnetAccount) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": RPC_REQUEST_ID,
        "method": "getBalance",
        "params": [
            account.address().to_string(),
            {
                "commitment": "confirmed"
            }
        ]
    })
}

fn parse_balance_response(response: &Value) -> Result<u64, &'static str> {
    if response.get("error").is_some_and(|error| !error.is_null()) {
        return Err("Devnet RPC request was rejected");
    }

    response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_u64)
        .ok_or("Devnet RPC response was invalid")
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

    use super::{
        build_balance_request, parse_balance_response, temporary_snapshot_path,
        verify_observability_identity, write_snapshot_atomically, RPC_REQUEST_ID,
    };
    use serde_json::json;
    use wallet_engine::{LockedVault, SecretPassphrase, SecretSeed};

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

    #[test]
    fn observability_identity_accepts_wallet_owned_account(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let passphrase = SecretPassphrase::new("observability identity match".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([221_u8; 32]))?;
        let wallet = vault.unlock(&passphrase)?;
        let account = wallet.devnet_account();

        assert!(verify_observability_identity(&wallet, &account).is_ok());

        Ok(())
    }

    #[test]
    fn observability_identity_rejects_unrelated_account() -> Result<(), Box<dyn std::error::Error>>
    {
        let first_passphrase = SecretPassphrase::new("observability identity first".to_owned());
        let second_passphrase = SecretPassphrase::new("observability identity second".to_owned());

        let first_vault =
            LockedVault::import_seed(&first_passphrase, SecretSeed::new([222_u8; 32]))?;
        let second_vault =
            LockedVault::import_seed(&second_passphrase, SecretSeed::new([223_u8; 32]))?;

        let first_wallet = first_vault.unlock(&first_passphrase)?;
        let unrelated_account = second_vault.devnet_account()?;

        assert!(verify_observability_identity(&first_wallet, &unrelated_account).is_err());

        Ok(())
    }

    #[test]
    fn balance_request_uses_verified_account_identity() -> Result<(), Box<dyn std::error::Error>> {
        let passphrase = SecretPassphrase::new("observability balance request".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([224_u8; 32]))?;
        let wallet = vault.unlock(&passphrase)?;
        let account = wallet.devnet_account();

        verify_observability_identity(&wallet, &account)?;

        let request = build_balance_request(&account);

        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["id"], RPC_REQUEST_ID);
        assert_eq!(request["method"], "getBalance");
        assert_eq!(request["params"][0], account.address().to_string());
        assert_eq!(request["params"][1]["commitment"], "confirmed");

        Ok(())
    }

    #[test]
    fn balance_response_accepts_unsigned_lamports() {
        let response = json!({
            "jsonrpc": "2.0",
            "result": {
                "context": {
                    "slot": 1
                },
                "value": 987_654_u64
            },
            "id": RPC_REQUEST_ID
        });

        assert_eq!(parse_balance_response(&response), Ok(987_654));
    }

    #[test]
    fn balance_response_rejects_rpc_error() {
        let response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32_000,
                "message": "rejected"
            },
            "id": RPC_REQUEST_ID
        });

        assert!(parse_balance_response(&response).is_err());
    }

    #[test]
    fn balance_response_rejects_missing_value() {
        let response = json!({
            "jsonrpc": "2.0",
            "result": {
                "context": {
                    "slot": 1
                }
            },
            "id": RPC_REQUEST_ID
        });

        assert!(parse_balance_response(&response).is_err());
    }
}
