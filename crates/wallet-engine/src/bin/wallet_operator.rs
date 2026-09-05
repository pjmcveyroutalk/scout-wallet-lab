use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use wallet_engine::{LockedVault, SecretPassphrase};

const DEFAULT_VAULT_PATH: &str = "vault-data/execution-vault.json";

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
        "usage: wallet_operator <generate|address|verify> [vault-path]\n\
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

    use super::{load_vault, verify_vault_identity, write_new_vault};
    use wallet_engine::{LockedVault, SecretPassphrase, SecretSeed};

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
