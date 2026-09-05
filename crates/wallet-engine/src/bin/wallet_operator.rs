use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use rustls::crypto::CryptoProvider;
use serde_json::{json, Value};
use wallet_engine::{DevnetAccount, LockedVault, SecretPassphrase};

const DEFAULT_VAULT_PATH: &str = "vault-data/execution-vault.json";
const RPC_TIMEOUT_SECONDS: u64 = 10;
const RPC_REQUEST_ID: u64 = 1;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args();
    let _program = arguments.next();

    let command = arguments.next().ok_or_else(usage)?;
    let vault_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_VAULT_PATH));

    if arguments.next().is_some() {
        return Err(usage().into());
    }

    match command.as_str() {
        "generate" => generate_wallet(&vault_path),
        "address" => print_address(&vault_path),
        "verify" => verify_wallet(&vault_path),
        "balance" => print_balance(&vault_path),
        _ => Err(usage().into()),
    }
}

fn generate_wallet(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Err(format!(
            "refusing to overwrite existing wallet vault: {}",
            path.display()
        )
        .into());
    }

    let passphrase = read_new_passphrase()?;
    let vault = LockedVault::generate(&passphrase)?;

    let verified_address = verify_vault_identity(&vault, &passphrase)?;
    let encoded = vault.to_json()?;

    write_new_vault(path, &encoded)?;

    let persisted = load_vault(path)?;
    let persisted_address = persisted.devnet_account()?.address().to_string();

    if persisted_address != verified_address {
        return Err("persisted wallet address does not match verified wallet identity".into());
    }

    println!("Scout Wallet Lab Devnet wallet created.");
    println!("Cluster: devnet");
    println!("Vault: {}", path.display());
    println!("Address: {verified_address}");
    println!("Identity verification: locked + unlocked + persisted match");
    println!("Mainnet: disabled");
    println!("Transaction submission: disabled");

    Ok(())
}

fn print_address(path: &Path) -> Result<(), Box<dyn Error>> {
    let vault = load_vault(path)?;
    let account = vault.devnet_account()?;

    println!("Scout Wallet Lab");
    println!("Cluster: {}", account.cluster().rpc_name());
    println!("Address: {}", account.address());
    println!("Source: encrypted vault public identity");
    println!("Mainnet: disabled");
    println!("Transaction submission: disabled");

    Ok(())
}

fn verify_wallet(path: &Path) -> Result<(), Box<dyn Error>> {
    let vault = load_vault(path)?;
    let passphrase = read_existing_passphrase()?;
    let verified_address = verify_vault_identity(&vault, &passphrase)?;

    println!("Scout Wallet Lab wallet identity verified.");
    println!("Cluster: devnet");
    println!("Address: {verified_address}");
    println!("Identity verification: locked and unlocked identities match");
    println!("Mainnet: disabled");
    println!("Transaction submission: disabled");

    Ok(())
}

fn print_balance(path: &Path) -> Result<(), Box<dyn Error>> {
    let vault = load_vault(path)?;
    let passphrase = read_existing_passphrase()?;
    let verified_address = verify_vault_identity(&vault, &passphrase)?;
    let mut account = vault.devnet_account()?;

    if account.address().to_string() != verified_address {
        return Err("verified wallet address does not match Devnet account identity".into());
    }

    let lamports = query_devnet_balance(&mut account)?;

    if account.lamports() != Some(lamports) {
        return Err("Devnet balance observation was not recorded on the verified account".into());
    }

    println!("Scout Wallet Lab Devnet balance verified.");
    println!("Cluster: {}", account.cluster().rpc_name());
    println!("Address: {verified_address}");
    println!("Balance (lamports): {lamports}");
    println!("Balance state: confirmed RPC observation");
    println!("Identity verification: locked and unlocked identities match");
    println!("Mainnet: disabled");
    println!("Transaction submission: disabled");

    Ok(())
}

fn query_devnet_balance(account: &mut DevnetAccount) -> Result<u64, Box<dyn Error>> {
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
    if response
        .get("error")
        .is_some_and(|error| !error.is_null())
    {
        return Err("Devnet RPC request was rejected");
    }

    response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_u64)
        .ok_or("Devnet RPC response was invalid")
}

fn verify_vault_identity(
    vault: &LockedVault,
    passphrase: &SecretPassphrase,
) -> Result<String, Box<dyn Error>> {
    let locked_public_key = vault.public_key()?;
    let locked_account = vault.devnet_account()?;

    let unlocked = vault.unlock(passphrase)?;
    let unlocked_public_key = unlocked.public_key();
    let unlocked_account = unlocked.devnet_account();

    if locked_public_key != unlocked_public_key {
        return Err("locked and unlocked wallet public keys do not match".into());
    }

    if locked_account.address() != unlocked_account.address() {
        return Err("locked and unlocked Devnet addresses do not match".into());
    }

    Ok(locked_account.address().to_string())
}

fn load_vault(path: &Path) -> Result<LockedVault, Box<dyn Error>> {
    let encoded = fs::read_to_string(path)?;
    Ok(LockedVault::from_json(&encoded)?)
}

fn read_new_passphrase() -> Result<SecretPassphrase, Box<dyn Error>> {
    let first = read_line("Enter a new wallet passphrase: ")?;
    let second = read_line("Confirm wallet passphrase: ")?;

    if first.is_empty() {
        return Err("wallet passphrase must not be empty".into());
    }

    if first != second {
        return Err("wallet passphrase confirmation does not match".into());
    }

    Ok(SecretPassphrase::new(first))
}

fn read_existing_passphrase() -> Result<SecretPassphrase, Box<dyn Error>> {
    let value = read_line("Enter wallet passphrase: ")?;

    if value.is_empty() {
        return Err("wallet passphrase must not be empty".into());
    }

    Ok(SecretPassphrase::new(value))
}

fn read_line(prompt: &str) -> Result<String, Box<dyn Error>> {
    print!("{prompt}");
    io::stdout().flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;

    while value.ends_with(['\n', '\r']) {
        value.pop();
    }

    Ok(value)
}

fn write_new_vault(path: &Path, encoded: &str) -> Result<(), Box<dyn Error>> {
    let parent = path
        .parent()
        .ok_or_else(|| String::from("wallet vault path must have a parent directory"))?;

    fs::create_dir_all(parent)?;

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options.open(path)?;
    file.write_all(encoded.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;

    Ok(())
}

fn usage() -> String {
    format!(
        "usage: wallet_operator <generate|address|verify|balance> [vault-path]\n\
         default vault path: {DEFAULT_VAULT_PATH}"
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        build_balance_request, load_vault, parse_balance_response, verify_vault_identity,
        write_new_vault, RPC_REQUEST_ID,
    };
    use serde_json::json;
    use wallet_engine::{DevnetAccount, LockedVault, SecretPassphrase, SecretSeed};

    fn unique_test_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();

        Ok(std::env::temp_dir()
            .join(format!("scout-wallet-operator-{nonce}"))
            .join("execution-vault.json"))
    }

    #[test]
    fn new_vault_write_refuses_overwrite() -> Result<(), Box<dyn std::error::Error>> {
        let path = unique_test_path()?;

        write_new_vault(&path, "{\"first\":true}")?;
        let overwrite = write_new_vault(&path, "{\"second\":true}");

        assert!(overwrite.is_err());
        assert_eq!(fs::read_to_string(&path)?, "{\"first\":true}\n");

        if let Some(parent) = path.parent() {
            fs::remove_dir_all(parent)?;
        }

        Ok(())
    }

    #[test]
    fn stored_vault_recovers_same_devnet_address() -> Result<(), Box<dyn std::error::Error>> {
        let path = unique_test_path()?;
        let passphrase = SecretPassphrase::new("operator address test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([211_u8; 32]))?;
        let expected = vault.devnet_account()?.address();

        write_new_vault(&path, &vault.to_json()?)?;

        let loaded = load_vault(&path)?;

        assert_eq!(loaded.devnet_account()?.address(), expected);

        if let Some(parent) = path.parent() {
            fs::remove_dir_all(parent)?;
        }

        Ok(())
    }

    #[test]
    fn locked_and_unlocked_wallet_identity_must_match() -> Result<(), Box<dyn std::error::Error>> {
        let passphrase = SecretPassphrase::new("identity verification test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([212_u8; 32]))?;

        let verified_address = verify_vault_identity(&vault, &passphrase)?;

        assert_eq!(
            verified_address,
            vault.devnet_account()?.address().to_string()
        );

        Ok(())
    }

    #[test]
    fn identity_verification_rejects_wrong_passphrase() -> Result<(), Box<dyn std::error::Error>> {
        let correct = SecretPassphrase::new("correct operator passphrase".to_owned());
        let wrong = SecretPassphrase::new("incorrect operator passphrase".to_owned());
        let vault = LockedVault::import_seed(&correct, SecretSeed::new([213_u8; 32]))?;

        assert!(verify_vault_identity(&vault, &wrong).is_err());

        Ok(())
    }

    #[test]
    fn balance_request_is_pinned_to_verified_account() -> Result<(), Box<dyn std::error::Error>> {
        let passphrase = SecretPassphrase::new("balance request test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([214_u8; 32]))?;
        let account = vault.devnet_account()?;
        let request = build_balance_request(&account);

        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["id"], RPC_REQUEST_ID);
        assert_eq!(request["method"], "getBalance");
        assert_eq!(request["params"][0], account.address().to_string());
        assert_eq!(request["params"][1]["commitment"], "confirmed");

        Ok(())
    }

    #[test]
    fn balance_response_records_only_unsigned_lamports() {
        let response = json!({
            "jsonrpc": "2.0",
            "result": {
                "context": {
                    "slot": 1
                },
                "value": 123_456_u64
            },
            "id": RPC_REQUEST_ID
        });

        assert_eq!(parse_balance_response(&response), Ok(123_456));
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

    #[test]
    fn new_devnet_account_starts_without_balance() -> Result<(), Box<dyn std::error::Error>> {
        let passphrase = SecretPassphrase::new("empty balance state test".to_owned());
        let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([215_u8; 32]))?;
        let account = DevnetAccount::new(vault.devnet_account()?.address());

        assert_eq!(account.lamports(), None);

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn vault_file_is_created_owner_only() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let path = unique_test_path()?;

        write_new_vault(&path, "{\"vault\":true}")?;

        let mode = fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        if let Some(parent) = path.parent() {
            fs::remove_dir_all(parent)?;
        }

        Ok(())
    }
}
