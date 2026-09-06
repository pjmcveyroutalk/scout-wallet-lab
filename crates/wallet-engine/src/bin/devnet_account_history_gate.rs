#![forbid(unsafe_code)]

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;
use std::{error::Error, fmt, time::Duration};
use wallet_engine::{Cluster, LockedVault};

const RPC_TIMEOUT_SECONDS: u64 = 10;
const RPC_REQUEST_ID: u64 = 1;
const HISTORY_LIMIT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevnetSignatureRecord {
    signature: String,
    slot: u64,
}

impl DevnetSignatureRecord {
    #[must_use]
    pub fn signature(&self) -> &str {
        self.signature.as_str()
    }

    #[must_use]
    pub const fn slot(&self) -> u64 {
        self.slot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryError {
    EmptyVaultJson,
    VaultParseFailed,
    AddressDerivationFailed,
    ClientInitializationFailed,
    TransportFailed,
    HttpStatusFailed,
    InvalidResponse,
    RpcRejected,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyVaultJson => "encrypted vault JSON is empty",
            Self::VaultParseFailed => "encrypted vault JSON could not be parsed",
            Self::AddressDerivationFailed => "Devnet public address could not be derived",
            Self::ClientInitializationFailed => "Devnet RPC client initialization failed",
            Self::TransportFailed => "Devnet RPC transport failed",
            Self::HttpStatusFailed => "Devnet RPC HTTP status rejected",
            Self::InvalidResponse => "Devnet RPC history response was invalid",
            Self::RpcRejected => "Devnet RPC history request was rejected",
        };

        formatter.write_str(message)
    }
}

impl Error for HistoryError {}

#[derive(Serialize)]
struct GetSignaturesForAddressRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: (String, SignatureHistoryConfig),
}

#[derive(Serialize)]
struct SignatureHistoryConfig {
    commitment: &'static str,
    limit: usize,
}

#[derive(Deserialize)]
struct GetSignaturesForAddressResponse {
    result: Option<Vec<GetSignaturesForAddressResult>>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetSignaturesForAddressResult {
    signature: String,
    slot: u64,
}

pub fn fetch_locked_vault_devnet_history(
    vault_json: &str,
) -> Result<Vec<DevnetSignatureRecord>, HistoryError> {
    if vault_json.trim().is_empty() {
        return Err(HistoryError::EmptyVaultJson);
    }

    let vault = LockedVault::from_json(vault_json).map_err(|_| HistoryError::VaultParseFailed)?;

    let account = vault
        .devnet_account()
        .map_err(|_| HistoryError::AddressDerivationFailed)?;

    fetch_devnet_history(account.address())
}

fn fetch_devnet_history(address: Pubkey) -> Result<Vec<DevnetSignatureRecord>, HistoryError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(RPC_TIMEOUT_SECONDS))
        .build()
        .map_err(|_| HistoryError::ClientInitializationFailed)?;

    let request = build_get_signatures_for_address_request(address);

    let response = client
        .post(Cluster::Devnet.rpc_url())
        .json(&request)
        .send()
        .map_err(|_| HistoryError::TransportFailed)?;

    if !response.status().is_success() {
        return Err(HistoryError::HttpStatusFailed);
    }

    let response = response
        .json::<GetSignaturesForAddressResponse>()
        .map_err(|_| HistoryError::InvalidResponse)?;

    parse_history_response(response)
}

fn build_get_signatures_for_address_request(address: Pubkey) -> GetSignaturesForAddressRequest {
    GetSignaturesForAddressRequest {
        jsonrpc: "2.0",
        id: RPC_REQUEST_ID,
        method: "getSignaturesForAddress",
        params: (
            address.to_string(),
            SignatureHistoryConfig {
                commitment: "confirmed",
                limit: HISTORY_LIMIT,
            },
        ),
    }
}

fn parse_history_response(
    response: GetSignaturesForAddressResponse,
) -> Result<Vec<DevnetSignatureRecord>, HistoryError> {
    if response.error.is_some() {
        return Err(HistoryError::RpcRejected);
    }

    let records = response.result.ok_or(HistoryError::InvalidResponse)?;

    Ok(records
        .into_iter()
        .map(|record| DevnetSignatureRecord {
            signature: record.signature,
            slot: record.slot,
        })
        .collect())
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_rpc_error_response() {
        let response = GetSignaturesForAddressResponse {
            result: None,
            error: Some(serde_json::json!({
                "code": -32603,
                "message": "Internal error"
            })),
        };

        let result = parse_history_response(response);

        assert_eq!(result, Err(HistoryError::RpcRejected));
    }

    #[test]
    fn rejects_response_without_result() {
        let response = GetSignaturesForAddressResponse {
            result: None,
            error: None,
        };

        let result = parse_history_response(response);

        assert_eq!(result, Err(HistoryError::InvalidResponse));
    }

    #[test]
    fn accepts_empty_history() {
        let response = GetSignaturesForAddressResponse {
            result: Some(Vec::new()),
            error: None,
        };

        let result = parse_history_response(response);

        assert_eq!(result, Ok(Vec::new()));
    }

    #[test]
    fn preserves_signature_and_slot() {
        let response = GetSignaturesForAddressResponse {
            result: Some(vec![GetSignaturesForAddressResult {
                signature: "test-signature".to_owned(),
                slot: 123_456,
            }]),
            error: None,
        };

        let result = parse_history_response(response);

        assert_eq!(
            result,
            Ok(vec![DevnetSignatureRecord {
                signature: "test-signature".to_owned(),
                slot: 123_456,
            }]),
        );
    }

    #[test]
    fn request_is_devnet_read_only_and_bounded() {
        let address = Pubkey::new_from_array([7_u8; 32]);
        let request = build_get_signatures_for_address_request(address);

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RPC_REQUEST_ID);
        assert_eq!(request.method, "getSignaturesForAddress");
        assert_eq!(request.params.0, address.to_string());
        assert_eq!(request.params.1.commitment, "confirmed");
        assert_eq!(request.params.1.limit, 10);
        assert_eq!(Cluster::Devnet.rpc_url(), "https://api.devnet.solana.com");
    }
}
