use super::devnet_signing_coordinator::{DevnetSigningCoordinator, DevnetSigningCoordinatorError};
use crate::{
    CanonicalTransactionMessage, Cluster, DevnetRpc, ExecutionPolicy, PreparedTransaction,
    RpcError, SignatureBytes,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_hash::Hash;
use solana_instruction::Instruction;
use solana_message::Message;
use solana_pubkey::Pubkey;
use std::{
    fmt,
    str::FromStr,
    sync::{Mutex, OnceLock},
    time::Duration,
};
use zeroize::Zeroizing;

const STAGE_F_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const STAGE_F_PAYLOAD: &[u8] = b"scout-stage-f-devnet-submission-proof-v1";
const STAGE_F_MAX_FEE_LAMPORTS: u64 = 10_000;
const STAGE_F_MIN_REMAINING_BALANCE_LAMPORTS: u64 = 1_000_000;
const STAGE_F_RPC_TIMEOUT_SECONDS: u64 = 10;
const STAGE_F_RPC_REQUEST_ID: u64 = 3;
const STAGE_F_TOKEN_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageFPreSubmitError {
    Coordinator(DevnetSigningCoordinatorError),
    Rpc(RpcError),
    ClientInitializationFailed,
    TransportFailed,
    HttpStatusFailed,
    InvalidResponse,
    RpcRejected,
    FeeOutsideBoundary,
    InsufficientBalance,
    InvalidCanonicalMessage,
    TokenGenerationFailed,
    CandidateAlreadyPrepared,
    CandidateRegistryUnavailable,
    CandidateNotFound,
    CandidateTokenMismatch,
}

impl fmt::Display for StageFPreSubmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Coordinator(_) => "Stage F signing coordinator gate failed",
            Self::Rpc(_) => "Stage F Devnet RPC gate failed",
            Self::ClientInitializationFailed => "Stage F RPC client initialization failed",
            Self::TransportFailed => "Stage F RPC transport failed",
            Self::HttpStatusFailed => "Stage F RPC HTTP status rejected",
            Self::InvalidResponse => "Stage F RPC response was invalid",
            Self::RpcRejected => "Stage F RPC request was rejected",
            Self::FeeOutsideBoundary => "Stage F fee is outside the fixed Devnet boundary",
            Self::InsufficientBalance => "Stage F Devnet balance floor would be violated",
            Self::InvalidCanonicalMessage => "Stage F canonical transaction message is invalid",
            Self::TokenGenerationFailed => "Stage F candidate token generation failed",
            Self::CandidateAlreadyPrepared => "Stage F candidate is already prepared",
            Self::CandidateRegistryUnavailable => "Stage F candidate registry is unavailable",
            Self::CandidateNotFound => "Stage F prepared candidate was not found",
            Self::CandidateTokenMismatch => "Stage F prepared candidate token did not match",
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for StageFPreSubmitError {}

impl From<DevnetSigningCoordinatorError> for StageFPreSubmitError {
    fn from(error: DevnetSigningCoordinatorError) -> Self {
        Self::Coordinator(error)
    }
}

impl From<RpcError> for StageFPreSubmitError {
    fn from(error: RpcError) -> Self {
        Self::Rpc(error)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StageFCandidateToken {
    value: [u8; STAGE_F_TOKEN_LEN],
}

impl StageFCandidateToken {
    #[must_use]
    pub const fn from_bytes(value: [u8; STAGE_F_TOKEN_LEN]) -> Self {
        Self { value }
    }

    #[must_use]
    pub const fn to_bytes(self) -> [u8; STAGE_F_TOKEN_LEN] {
        self.value
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StageFPreSubmitMetadata {
    public_key: Pubkey,
    signature: SignatureBytes,
    recent_blockhash: Hash,
    fee_lamports: u64,
    balance_lamports: u64,
    remaining_balance_lamports: u64,
    simulation_slot: u64,
    units_consumed: u64,
    last_valid_block_height: u64,
    candidate_token: StageFCandidateToken,
}

impl StageFPreSubmitMetadata {
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
    pub const fn fee_lamports(&self) -> u64 {
        self.fee_lamports
    }

    #[must_use]
    pub const fn balance_lamports(&self) -> u64 {
        self.balance_lamports
    }

    #[must_use]
    pub const fn remaining_balance_lamports(&self) -> u64 {
        self.remaining_balance_lamports
    }

    #[must_use]
    pub const fn simulation_slot(&self) -> u64 {
        self.simulation_slot
    }

    #[must_use]
    pub const fn units_consumed(&self) -> u64 {
        self.units_consumed
    }

    #[must_use]
    pub const fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    #[must_use]
    pub const fn candidate_token(&self) -> StageFCandidateToken {
        self.candidate_token
    }
}

#[derive(Serialize)]
struct GetFeeForMessageRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: (String, CommitmentConfig),
}

#[derive(Serialize)]
struct SimulateTransactionRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: (String, SimulationConfig),
}

#[derive(Serialize)]
struct CommitmentConfig {
    commitment: &'static str,
}

#[derive(Serialize)]
struct SimulationConfig {
    encoding: &'static str,
    #[serde(rename = "sigVerify")]
    sig_verify: bool,
    #[serde(rename = "replaceRecentBlockhash")]
    replace_recent_blockhash: bool,
    commitment: &'static str,
}

#[derive(Deserialize)]
struct GetFeeForMessageResponse {
    result: Option<GetFeeForMessageResult>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetFeeForMessageResult {
    value: Option<u64>,
}

#[derive(Deserialize)]
struct SimulateTransactionResponse {
    result: Option<SimulateTransactionResult>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SimulateTransactionResult {
    context: SimulationContext,
    value: SimulationValue,
}

#[derive(Deserialize)]
struct SimulationContext {
    slot: u64,
}

#[derive(Deserialize)]
struct SimulationValue {
    err: Option<serde_json::Value>,
    #[serde(rename = "unitsConsumed")]
    units_consumed: Option<u64>,
}

struct StageFPreSubmitRpc {
    client: Client,
}

impl StageFPreSubmitRpc {
    fn new() -> Result<Self, StageFPreSubmitError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(STAGE_F_RPC_TIMEOUT_SECONDS))
            .build()
            .map_err(|_| StageFPreSubmitError::ClientInitializationFailed)?;

        Ok(Self { client })
    }

    async fn get_fee_for_message(&self, message: &[u8]) -> Result<u64, StageFPreSubmitError> {
        let request = GetFeeForMessageRequest {
            jsonrpc: "2.0",
            id: STAGE_F_RPC_REQUEST_ID,
            method: "getFeeForMessage",
            params: (
                BASE64.encode(message),
                CommitmentConfig {
                    commitment: "confirmed",
                },
            ),
        };

        let response = self
            .client
            .post(Cluster::Devnet.rpc_url())
            .json(&request)
            .send()
            .await
            .map_err(|_| StageFPreSubmitError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(StageFPreSubmitError::HttpStatusFailed);
        }

        let response = response
            .json::<GetFeeForMessageResponse>()
            .await
            .map_err(|_| StageFPreSubmitError::InvalidResponse)?;

        parse_fee_response(response)
    }

    async fn simulate_signed_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<(u64, u64), StageFPreSubmitError> {
        let request = SimulateTransactionRequest {
            jsonrpc: "2.0",
            id: STAGE_F_RPC_REQUEST_ID,
            method: "simulateTransaction",
            params: (
                BASE64.encode(transaction),
                SimulationConfig {
                    encoding: "base64",
                    sig_verify: true,
                    replace_recent_blockhash: false,
                    commitment: "confirmed",
                },
            ),
        };

        let response = self
            .client
            .post(Cluster::Devnet.rpc_url())
            .json(&request)
            .send()
            .await
            .map_err(|_| StageFPreSubmitError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(StageFPreSubmitError::HttpStatusFailed);
        }

        let response = response
            .json::<SimulateTransactionResponse>()
            .await
            .map_err(|_| StageFPreSubmitError::InvalidResponse)?;

        parse_simulation_response(response)
    }
}

struct PreparedStageFCandidate {
    token: StageFCandidateToken,
    wire_transaction: Zeroizing<Vec<u8>>,
    last_valid_block_height: u64,
}

#[derive(Default)]
struct CandidateStore {
    candidate: Option<PreparedStageFCandidate>,
}

impl CandidateStore {
    fn insert(&mut self, candidate: PreparedStageFCandidate) -> Result<(), StageFPreSubmitError> {
        if self.candidate.is_some() {
            return Err(StageFPreSubmitError::CandidateAlreadyPrepared);
        }

        if candidate.wire_transaction.is_empty() {
            return Err(StageFPreSubmitError::InvalidCanonicalMessage);
        }

        self.candidate = Some(candidate);
        Ok(())
    }

    fn discard(&mut self, token: StageFCandidateToken) -> Result<(), StageFPreSubmitError> {
        let candidate = self
            .candidate
            .as_ref()
            .ok_or(StageFPreSubmitError::CandidateNotFound)?;

        if candidate.token != token {
            return Err(StageFPreSubmitError::CandidateTokenMismatch);
        }

        self.candidate = None;
        Ok(())
    }

    fn is_expired(
        &self,
        token: StageFCandidateToken,
        current_block_height: u64,
    ) -> Result<bool, StageFPreSubmitError> {
        let candidate = self
            .candidate
            .as_ref()
            .ok_or(StageFPreSubmitError::CandidateNotFound)?;

        if candidate.token != token {
            return Err(StageFPreSubmitError::CandidateTokenMismatch);
        }

        Ok(current_block_height > candidate.last_valid_block_height)
    }
}

static STAGE_F_CANDIDATE_STORE: OnceLock<Mutex<CandidateStore>> = OnceLock::new();

fn candidate_store() -> &'static Mutex<CandidateStore> {
    STAGE_F_CANDIDATE_STORE.get_or_init(|| Mutex::new(CandidateStore::default()))
}

pub async fn prepare_fixed_devnet_candidate(
    coordinator: &DevnetSigningCoordinator,
) -> Result<StageFPreSubmitMetadata, StageFPreSubmitError> {
    ensure_candidate_slot_empty()?;

    let blockhash_rpc = DevnetRpc::new()?;
    let lease = blockhash_rpc.resolve_fresh_blockhash().await?;
    let instruction = stage_f_instruction()?;
    let preview_message = CanonicalTransactionMessage::new(
        std::slice::from_ref(&instruction),
        coordinator.public_key(),
        lease.recent_blockhash(),
    )
    .map_err(|_| StageFPreSubmitError::InvalidCanonicalMessage)?;

    let rpc = StageFPreSubmitRpc::new()?;
    let fee_lamports = rpc.get_fee_for_message(preview_message.bytes()).await?;
    validate_fee(fee_lamports)?;

    let balance_lamports = blockhash_rpc.get_balance(coordinator.public_key()).await?;
    let remaining_balance_lamports = validate_balance(balance_lamports, fee_lamports)?;

    let policy = ExecutionPolicy::new(STAGE_F_MAX_FEE_LAMPORTS, &[instruction.program_id])
        .map_err(DevnetSigningCoordinatorError::from)?;
    let mut transaction = PreparedTransaction::reserve(
        std::slice::from_ref(&instruction),
        coordinator.public_key(),
        fee_lamports,
        lease,
    )
    .map_err(|_| StageFPreSubmitError::InvalidCanonicalMessage)?;

    if transaction.message().bytes() != preview_message.bytes() {
        return Err(StageFPreSubmitError::InvalidCanonicalMessage);
    }

    validate_single_signer_message(transaction.message().bytes(), coordinator.public_key())?;

    let signed = coordinator.sign_prepared_transaction(
        &mut transaction,
        &policy,
        lease.observed_block_height(),
    )?;
    let wire_transaction = Zeroizing::new(encode_single_signature_transaction(
        signed.signature().to_bytes(),
        transaction.message().bytes(),
    ));
    let (simulation_slot, units_consumed) = rpc
        .simulate_signed_transaction(wire_transaction.as_slice())
        .await?;

    let candidate_token = generate_candidate_token()?;
    let candidate = PreparedStageFCandidate {
        token: candidate_token,
        wire_transaction,
        last_valid_block_height: lease.last_valid_block_height(),
    };
    store_candidate(candidate)?;

    Ok(StageFPreSubmitMetadata {
        public_key: signed.public_key(),
        signature: signed.signature(),
        recent_blockhash: signed.recent_blockhash(),
        fee_lamports,
        balance_lamports,
        remaining_balance_lamports,
        simulation_slot,
        units_consumed,
        last_valid_block_height: lease.last_valid_block_height(),
        candidate_token,
    })
}

pub fn discard_prepared_candidate(token: StageFCandidateToken) -> Result<(), StageFPreSubmitError> {
    let mut store = candidate_store()
        .lock()
        .map_err(|_| StageFPreSubmitError::CandidateRegistryUnavailable)?;
    store.discard(token)
}

pub fn prepared_candidate_is_expired(
    token: StageFCandidateToken,
    current_block_height: u64,
) -> Result<bool, StageFPreSubmitError> {
    let store = candidate_store()
        .lock()
        .map_err(|_| StageFPreSubmitError::CandidateRegistryUnavailable)?;
    store.is_expired(token, current_block_height)
}

fn ensure_candidate_slot_empty() -> Result<(), StageFPreSubmitError> {
    let store = candidate_store()
        .lock()
        .map_err(|_| StageFPreSubmitError::CandidateRegistryUnavailable)?;

    if store.candidate.is_some() {
        return Err(StageFPreSubmitError::CandidateAlreadyPrepared);
    }

    Ok(())
}

fn store_candidate(candidate: PreparedStageFCandidate) -> Result<(), StageFPreSubmitError> {
    let mut store = candidate_store()
        .lock()
        .map_err(|_| StageFPreSubmitError::CandidateRegistryUnavailable)?;
    store.insert(candidate)
}

fn generate_candidate_token() -> Result<StageFCandidateToken, StageFPreSubmitError> {
    let mut token = [0_u8; STAGE_F_TOKEN_LEN];
    getrandom::getrandom(&mut token).map_err(|_| StageFPreSubmitError::TokenGenerationFailed)?;
    Ok(StageFCandidateToken::from_bytes(token))
}

fn stage_f_instruction() -> Result<Instruction, StageFPreSubmitError> {
    let program_id = Pubkey::from_str(STAGE_F_PROGRAM_ID)
        .map_err(|_| StageFPreSubmitError::InvalidCanonicalMessage)?;

    Ok(Instruction {
        program_id,
        accounts: Vec::new(),
        data: STAGE_F_PAYLOAD.to_vec(),
    })
}

fn validate_fee(fee_lamports: u64) -> Result<(), StageFPreSubmitError> {
    if fee_lamports == 0 || fee_lamports > STAGE_F_MAX_FEE_LAMPORTS {
        return Err(StageFPreSubmitError::FeeOutsideBoundary);
    }

    Ok(())
}

fn validate_balance(balance_lamports: u64, fee_lamports: u64) -> Result<u64, StageFPreSubmitError> {
    let remaining_balance_lamports = balance_lamports
        .checked_sub(fee_lamports)
        .ok_or(StageFPreSubmitError::InsufficientBalance)?;

    if remaining_balance_lamports < STAGE_F_MIN_REMAINING_BALANCE_LAMPORTS {
        return Err(StageFPreSubmitError::InsufficientBalance);
    }

    Ok(remaining_balance_lamports)
}

fn validate_single_signer_message(
    message_bytes: &[u8],
    expected_payer: Pubkey,
) -> Result<(), StageFPreSubmitError> {
    let message: Message = bincode::deserialize(message_bytes)
        .map_err(|_| StageFPreSubmitError::InvalidCanonicalMessage)?;

    if message.header.num_required_signatures != 1
        || message.account_keys.first().copied() != Some(expected_payer)
    {
        return Err(StageFPreSubmitError::InvalidCanonicalMessage);
    }

    Ok(())
}

fn encode_single_signature_transaction(signature: [u8; 64], message_bytes: &[u8]) -> Vec<u8> {
    let mut transaction = Vec::with_capacity(1 + signature.len() + message_bytes.len());
    transaction.push(1_u8);
    transaction.extend_from_slice(&signature);
    transaction.extend_from_slice(message_bytes);
    transaction
}

fn parse_fee_response(response: GetFeeForMessageResponse) -> Result<u64, StageFPreSubmitError> {
    if response.error.is_some() {
        return Err(StageFPreSubmitError::RpcRejected);
    }

    response
        .result
        .and_then(|result| result.value)
        .ok_or(StageFPreSubmitError::InvalidResponse)
}

fn parse_simulation_response(
    response: SimulateTransactionResponse,
) -> Result<(u64, u64), StageFPreSubmitError> {
    if response.error.is_some() {
        return Err(StageFPreSubmitError::RpcRejected);
    }

    let result = response
        .result
        .ok_or(StageFPreSubmitError::InvalidResponse)?;
    if result.value.err.is_some() {
        return Err(StageFPreSubmitError::RpcRejected);
    }

    let units_consumed = result
        .value
        .units_consumed
        .ok_or(StageFPreSubmitError::InvalidResponse)?;

    Ok((result.context.slot, units_consumed))
}

#[cfg(test)]
mod tests {
    use super::{
        encode_single_signature_transaction, parse_fee_response, parse_simulation_response,
        stage_f_instruction, validate_balance, validate_fee, CandidateStore,
        GetFeeForMessageResponse, GetFeeForMessageResult, PreparedStageFCandidate,
        SimulateTransactionResponse, SimulateTransactionResult, SimulationContext, SimulationValue,
        StageFCandidateToken, StageFPreSubmitError, STAGE_F_MAX_FEE_LAMPORTS,
        STAGE_F_MIN_REMAINING_BALANCE_LAMPORTS, STAGE_F_PAYLOAD, STAGE_F_PROGRAM_ID,
    };
    use zeroize::Zeroizing;

    #[test]
    fn stage_f_instruction_is_fixed_and_has_no_accounts() -> Result<(), StageFPreSubmitError> {
        let instruction = stage_f_instruction()?;

        assert_eq!(instruction.program_id.to_string(), STAGE_F_PROGRAM_ID);
        assert_eq!(instruction.data, STAGE_F_PAYLOAD);
        assert!(instruction.accounts.is_empty());
        Ok(())
    }

    #[test]
    fn fee_boundary_rejects_zero_and_over_cap() {
        assert!(matches!(
            validate_fee(0),
            Err(StageFPreSubmitError::FeeOutsideBoundary)
        ));
        assert!(validate_fee(STAGE_F_MAX_FEE_LAMPORTS).is_ok());
        assert!(matches!(
            validate_fee(STAGE_F_MAX_FEE_LAMPORTS + 1),
            Err(StageFPreSubmitError::FeeOutsideBoundary)
        ));
    }

    #[test]
    fn balance_boundary_preserves_fixed_floor() {
        let fee = STAGE_F_MAX_FEE_LAMPORTS;
        let exact_balance = STAGE_F_MIN_REMAINING_BALANCE_LAMPORTS + fee;

        assert_eq!(
            validate_balance(exact_balance, fee),
            Ok(STAGE_F_MIN_REMAINING_BALANCE_LAMPORTS)
        );
        assert!(matches!(
            validate_balance(exact_balance - 1, fee),
            Err(StageFPreSubmitError::InsufficientBalance)
        ));
    }

    #[test]
    fn wire_encoding_is_one_signature_followed_by_message() {
        let signature = [0x5a_u8; 64];
        let message = [0x31_u8, 0x32_u8, 0x33_u8];
        let encoded = encode_single_signature_transaction(signature, &message);

        assert_eq!(encoded[0], 1_u8);
        assert_eq!(&encoded[1..65], &signature);
        assert_eq!(&encoded[65..], &message);
    }

    #[test]
    fn fee_response_requires_value() {
        let valid = GetFeeForMessageResponse {
            result: Some(GetFeeForMessageResult { value: Some(5_000) }),
            error: None,
        };
        assert_eq!(parse_fee_response(valid), Ok(5_000));

        let missing = GetFeeForMessageResponse {
            result: Some(GetFeeForMessageResult { value: None }),
            error: None,
        };
        assert_eq!(
            parse_fee_response(missing),
            Err(StageFPreSubmitError::InvalidResponse)
        );
    }

    #[test]
    fn simulation_response_requires_clean_execution_and_units() {
        let valid = SimulateTransactionResponse {
            result: Some(SimulateTransactionResult {
                context: SimulationContext { slot: 123 },
                value: SimulationValue {
                    err: None,
                    units_consumed: Some(456),
                },
            }),
            error: None,
        };
        assert_eq!(parse_simulation_response(valid), Ok((123, 456)));

        let rejected = SimulateTransactionResponse {
            result: Some(SimulateTransactionResult {
                context: SimulationContext { slot: 123 },
                value: SimulationValue {
                    err: Some(serde_json::Value::Bool(true)),
                    units_consumed: Some(456),
                },
            }),
            error: None,
        };
        assert_eq!(
            parse_simulation_response(rejected),
            Err(StageFPreSubmitError::RpcRejected)
        );
    }

    #[test]
    fn candidate_store_is_single_slot_and_token_bound() -> Result<(), StageFPreSubmitError> {
        let first_token = StageFCandidateToken::from_bytes([0x11_u8; 16]);
        let second_token = StageFCandidateToken::from_bytes([0x22_u8; 16]);
        let mut store = CandidateStore::default();
        let candidate = PreparedStageFCandidate {
            token: first_token,
            wire_transaction: Zeroizing::new(vec![1_u8, 2_u8, 3_u8]),
            last_valid_block_height: 500,
        };

        store.insert(candidate)?;
        assert!(matches!(
            store.insert(PreparedStageFCandidate {
                token: second_token,
                wire_transaction: Zeroizing::new(vec![4_u8]),
                last_valid_block_height: 600,
            }),
            Err(StageFPreSubmitError::CandidateAlreadyPrepared)
        ));
        assert!(matches!(
            store.discard(second_token),
            Err(StageFPreSubmitError::CandidateTokenMismatch)
        ));
        assert!(!store.is_expired(first_token, 500)?);
        assert!(store.is_expired(first_token, 501)?);
        store.discard(first_token)?;
        assert!(matches!(
            store.discard(first_token),
            Err(StageFPreSubmitError::CandidateNotFound)
        ));

        Ok(())
    }
}
