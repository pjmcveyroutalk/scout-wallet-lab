#[test]
fn policy_mints_authorization_for_reserved_allowed_transaction() -> Result<(), VaultError> {
    let passphrase = SecretPassphrase::new("policy signing test".to_owned());
    let vault = LockedVault::import_seed(&passphrase, SecretSeed::new([166_u8; 32]))?;
    let unlocked = vault.unlock(&passphrase)?;
    let payer = Pubkey::new_from_array(unlocked.public_key());
    let program_id = Pubkey::new_from_array([167_u8; 32]);
    let blockhash = Hash::new_from_array([168_u8; 32]);
    let instruction = Instruction {
        program_id,
        accounts: Vec::new(),
        data: vec![5_u8, 6_u8],
    };
    let lease = test_lease(blockhash, 1_100, 1_090);
    let mut prepared = PreparedTransaction::reserve(&[instruction], payer, 400_000_000, lease)
        .map_err(|_| VaultError::SerializationFailed)?;
    let policy = ExecutionPolicy::new(500_000_000, &[program_id])
        .map_err(|_| VaultError::SerializationFailed)?;

    assert_eq!(policy.max_reserved_lamports(), 500_000_000);
    assert_eq!(policy.allowed_programs(), &[program_id]);

    let signature = {
        let authorized = policy
            .authorize(&prepared, 1_090)
            .map_err(|_| VaultError::SerializationFailed)?;

        unlocked
            .sign_transaction_message(&authorized)
            .map_err(|_| VaultError::SerializationFailed)?
    };

    let verifying_key = VerifyingKey::from_bytes(&unlocked.public_key())
        .map_err(|_| VaultError::InvalidFormat)?;
    let verification_signature = Signature::from_bytes(&signature.to_bytes());

    assert!(verifying_key
        .verify(prepared.message().bytes(), &verification_signature)
        .is_ok());

    prepared
        .ledger_mut()
        .mark_signed(signature)
        .map_err(|_| VaultError::SerializationFailed)?;

    assert_eq!(prepared.ledger().state(), TransactionState::Signed);
    assert!(prepared.ledger().signature() == Some(signature));
    Ok(())
}
