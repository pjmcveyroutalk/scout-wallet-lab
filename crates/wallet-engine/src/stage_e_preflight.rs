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
use std::{fmt, str::FromStr, time::Duration};

const STAGE_E_PREFLIGHT_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const STAGE_E_PREFLIGHT_PAYLOAD: &[u8] = b"scout-stage-e-devnet-simulation-proof-v1";
const STAGE_E_MAX_FEE_LAMPORTS: u64 = 10_000;
const STAGE_E_RPC_TIMEOUT_SECONDS: u64 = 10;
const STAGE_E_RPC_REQUEST_ID: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageEPreflightError {
    Coordinator(DevnetSigningCoordinatorError),
    Rpc(RpcError),
    ClientInitializationFailed,
    TransportFailed,
    HttpStatusFailed,
    InvalidResponse,
    RpcRejected,
    FeeOutsideBoundary,
    InvalidCanonicalMessage,
}

impl fmt::Display for StageEPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Coordinator(_) => "Stage E signing coordinator gate failed",
            Self::Rpc(_) => "Stage E Devnet blockhash gate failed",
            Self::ClientInitializationFailed => "Stage E RPC client initialization failed",
            Self::TransportFailed => "Stage E RPC transport failed",
            Self::HttpStatusFailed => "Stage E RPC HTTP status rejected",
            Self::InvalidResponse => "Stage E RPC response was invalid",
            Self::RpcRejected => "Stage E RPC request was rejected",
            Self::FeeOutsideBoundary => "Stage E fee is outside the fixed Devnet boundary",
            Self::InvalidCanonicalMessage => "Stage E canonical transaction message is invalid",
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for StageEPreflightError {}

impl From<DevnetSigningCoordinatorError> for StageEPreflightError {
    fn from(error: DevnetSigningCoordinatorError) -> Self {
        Self::Coordinator(error)
    }
}

impl From<RpcError> for StageEPreflightError {
    fn from(error: RpcError) -> Self {
        Self::Rpc(error)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StageEPreflightMetadata {
    public_key: Pubkey,
    signature: SignatureBytes,
    recent_blockhash: Hash,
    fee_lamports: u64,
    simulation_slot: u64,
    units_consumed: u64,
}

impl StageEPreflightMetadata {
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
    pub const fn simulation_slot(&self) -> u64 {
        self.simulation_slot
    }

    #[must_use]
    pub const fn units_consumed(&self) -> u64 {
        self.units_consumed
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

struct StageEPreflightRpc {
    client: Client,
}

impl StageEPreflightRpc {
    fn new() -> Result<Self, StageEPreflightError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(STAGE_E_RPC_TIMEOUT_SECONDS))
            .build()
            .map_err(|_| StageEPreflightError::ClientInitializationFailed)?;

        Ok(Self { client })
    }

    async fn get_fee_for_message(&self, message: &[u8]) -> Result<u64, StageEPreflightError> {
        let request = GetFeeForMessageRequest {
            jsonrpc: "2.0",
            id: STAGE_E_RPC_REQUEST_ID,
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
            .map_err(|_| StageEPreflightError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(StageEPreflightError::HttpStatusFailed);
        }

        let response = response
            .json::<GetFeeForMessageResponse>()
            .await
            .map_err(|_| StageEPreflightError::InvalidResponse)?;

        parse_fee_response(response)
    }

    async fn simulate_signed_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<(u64, u64), StageEPreflightError> {
        let request = SimulateTransactionRequest {
            jsonrpc: "2.0",
            id: STAGE_E_RPC_REQUEST_ID,
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
            .map_err(|_| StageEPreflightError::TransportFailed)?;

        if !response.status().is_success() {
            return Err(StageEPreflightError::HttpStatusFailed);
        }

        let response = response
            .json::<SimulateTransactionResponse>()
            .await
            .map_err(|_| StageEPreflightError::InvalidResponse)?;

        parse_simulation_response(response)
    }
}

pub async fn run_fixed_devnet_simulation(
    coordinator: &DevnetSigningCoordinator,
) -> Result<StageEPreflightMetadata, StageEPreflightError> {
    let blockhash_rpc = DevnetRpc::new()?;
    let lease = blockhash_rpc.resolve_fresh_blockhash().await?;
    let instruction = stage_e_preflight_instruction()?;
    let preview_message = CanonicalTransactionMessage::new(
        std::slice::from_ref(&instruction),
        coordinator.public_key(),
        lease.recent_blockhash(),
    )
    .map_err(|_| StageEPreflightError::InvalidCanonicalMessage)?;

    let rpc = StageEPreflightRpc::new()?;
    let fee_lamports = rpc.get_fee_for_message(preview_message.bytes()).await?;
    validate_fee(fee_lamports)?;

    let policy = ExecutionPolicy::new(STAGE_E_MAX_FEE_LAMPORTS, &[instruction.program_id])
        .map_err(DevnetSigningCoordinatorError::from)?;
    let mut transaction = PreparedTransaction::reserve(
        std::slice::from_ref(&instruction),
        coordinator.public_key(),
        fee_lamports,
        lease,
    )
    .map_err(|_| StageEPreflightError::InvalidCanonicalMessage)?;

    if transaction.message().bytes() != preview_message.bytes() {
        return Err(StageEPreflightError::InvalidCanonicalMessage);
    }

    validate_single_signer_message(transaction.message().bytes(), coordinator.public_key())?;

    let signed = coordinator.sign_prepared_transaction(
        &mut transaction,
        &policy,
        lease.observed_block_height(),
    )?;
    let wire_transaction = encode_single_signature_transaction(
        signed.signature().to_bytes(),
        transaction.message().bytes(),
    );
    let (simulation_slot, units_consumed) =
        rpc.simulate_signed_transaction(&wire_transaction).await?;

    Ok(StageEPreflightMetadata {
        public_key: signed.public_key(),
        signature: signed.signature(),
        recent_blockhash: signed.recent_blockhash(),
        fee_lamports,
        simulation_slot,
        units_consumed,
    })
}

fn stage_e_preflight_instruction() -> Result<Instruction, StageEPreflightError> {
    let program_id = Pubkey::from_str(STAGE_E_PREFLIGHT_PROGRAM_ID)
        .map_err(|_| StageEPreflightError::InvalidCanonicalMessage)?;

    Ok(Instruction {
        program_id,
        accounts: Vec::new(),
        data: STAGE_E_PREFLIGHT_PAYLOAD.to_vec(),
    })
}

fn validate_fee(fee_lamports: u64) -> Result<(), StageEPreflightError> {
    if fee_lamports == 0 || fee_lamports > STAGE_E_MAX_FEE_LAMPORTS {
        return Err(StageEPreflightError::FeeOutsideBoundary);
    }

    Ok(())
}

fn validate_single_signer_message(
    message_bytes: &[u8],
    expected_payer: Pubkey,
) -> Result<(), StageEPreflightError> {
    let message: Message = bincode::deserialize(message_bytes)
        .map_err(|_| StageEPreflightError::InvalidCanonicalMessage)?;

    if message.header.num_required_signatures != 1
        || message.account_keys.first().copied() != Some(expected_payer)
    {
        return Err(StageEPreflightError::InvalidCanonicalMessage);
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

fn parse_fee_response(response: GetFeeForMessageResponse) -> Result<u64, StageEPreflightError> {
    if response.error.is_some() {
        return Err(StageEPreflightError::RpcRejected);
    }

    response
        .result
        .and_then(|result| result.value)
        .ok_or(StageEPreflightError::InvalidResponse)
}

fn parse_simulation_response(
    response: SimulateTransactionResponse,
) -> Result<(u64, u64), StageEPreflightError> {
    if response.error.is_some() {
        return Err(StageEPreflightError::RpcRejected);
    }

    let result = response
        .result
        .ok_or(StageEPreflightError::InvalidResponse)?;
    if result.value.err.is_some() {
        return Err(StageEPreflightError::RpcRejected);
    }

    let units_consumed = result
        .value
        .units_consumed
        .ok_or(StageEPreflightError::InvalidResponse)?;

    Ok((result.context.slot, units_consumed))
}

#[cfg(test)]
mod tests {
    use super::{
        encode_single_signature_transaction, parse_fee_response, parse_simulation_response,
        stage_e_preflight_instruction, validate_fee, GetFeeForMessageResponse,
        SimulateTransactionResponse, StageEPreflightError, STAGE_E_MAX_FEE_LAMPORTS,
        STAGE_E_PREFLIGHT_PAYLOAD, STAGE_E_PREFLIGHT_PROGRAM_ID,
    };

    #[test]
    fn stage_e_instruction_is_fixed_and_has_no_accounts() -> Result<(), StageEPreflightError> {
        let instruction = stage_e_preflight_instruction()?;

        assert_eq!(
            instruction.program_id.to_string(),
            STAGE_E_PREFLIGHT_PROGRAM_ID
        );
        assert_eq!(instruction.data, STAGE_E_PREFLIGHT_PAYLOAD);
        assert!(instruction.accounts.is_empty());
        Ok(())
    }

    #[test]
    fn fee_boundary_rejects_zero_and_over_cap() {
        assert!(matches!(
            validate_fee(0),
            Err(StageEPreflightError::FeeOutsideBoundary)
        ));
        assert!(validate_fee(STAGE_E_MAX_FEE_LAMPORTS).is_ok());
        assert!(matches!(
            validate_fee(STAGE_E_MAX_FEE_LAMPORTS + 1),
            Err(StageEPreflightError::FeeOutsideBoundary)
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
        let valid: GetFeeForMessageResponse =
            serde_json::from_str(r#"{"result":{"value":5000},"error":null}"#)
                .expect("valid test fixture");
        assert_eq!(parse_fee_response(valid), Ok(5_000));

        let missing: GetFeeForMessageResponse =
            serde_json::from_str(r#"{"result":{"value":null},"error":null}"#)
                .expect("valid test fixture");
        assert_eq!(
            parse_fee_response(missing),
            Err(StageEPreflightError::InvalidResponse)
        );
    }

    #[test]
    fn simulation_response_requires_clean_execution_and_units() {
        let valid: SimulateTransactionResponse = serde_json::from_str(
            r#"{"result":{"context":{"slot":123},"value":{"err":null,"unitsConsumed":456}},"error":null}"#,
        )
        .expect("valid test fixture");
        assert_eq!(parse_simulation_response(valid), Ok((123, 456)));

        let rejected: SimulateTransactionResponse = serde_json::from_str(
            r#"{"result":{"context":{"slot":123},"value":{"err":{"InstructionError":[0,"Custom"]},"unitsConsumed":456}},"error":null}"#,
        )
        .expect("valid test fixture");
        assert_eq!(
            parse_simulation_response(rejected),
            Err(StageEPreflightError::RpcRejected)
        );
    }
}
