use serde::{Deserialize, Serialize};
use std::fmt;

const RPC_REQUEST_ID: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevnetSignatureResolution {
    Pending,
    Processed { slot: u64 },
    Confirmed { slot: u64 },
    Finalized { slot: u64 },
    Failed { slot: u64 },
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevnetSignatureStatusError {
    EmptySignature,
    InvalidResponse,
    RpcRejected,
    UnknownConfirmationStatus,
}

impl fmt::Display for DevnetSignatureStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptySignature => "Devnet signature status requires a public signature",
            Self::InvalidResponse => "Devnet signature status response was invalid",
            Self::RpcRejected => "Devnet signature status RPC request was rejected",
            Self::UnknownConfirmationStatus => {
                "Devnet signature status returned an unknown confirmation state"
            }
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for DevnetSignatureStatusError {}

#[derive(Serialize)]
struct SignatureStatusRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: ([&'a str; 1], SignatureStatusConfig),
}

#[derive(Serialize)]
struct SignatureStatusConfig {
    #[serde(rename = "searchTransactionHistory")]
    search_transaction_history: bool,
}

#[derive(Deserialize)]
struct SignatureStatusResponse {
    result: Option<SignatureStatusResult>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SignatureStatusResult {
    value: Vec<Option<SignatureStatusValue>>,
}

#[derive(Deserialize)]
struct SignatureStatusValue {
    slot: u64,
    err: Option<serde_json::Value>,
    #[serde(rename = "confirmationStatus")]
    confirmation_status: Option<String>,
}

pub fn request_json(signature: &str) -> Result<String, DevnetSignatureStatusError> {
    if signature.trim().is_empty() {
        return Err(DevnetSignatureStatusError::EmptySignature);
    }

    serde_json::to_string(&SignatureStatusRequest {
        jsonrpc: "2.0",
        id: RPC_REQUEST_ID,
        method: "getSignatureStatuses",
        params: (
            [signature],
            SignatureStatusConfig {
                search_transaction_history: false,
            },
        ),
    })
    .map_err(|_| DevnetSignatureStatusError::InvalidResponse)
}

pub fn resolve_response_json(
    response_json: &str,
    current_block_height: u64,
    last_valid_block_height: u64,
) -> Result<DevnetSignatureResolution, DevnetSignatureStatusError> {
    let response = serde_json::from_str::<SignatureStatusResponse>(response_json)
        .map_err(|_| DevnetSignatureStatusError::InvalidResponse)?;

    resolve_response(response, current_block_height, last_valid_block_height)
}

fn resolve_response(
    response: SignatureStatusResponse,
    current_block_height: u64,
    last_valid_block_height: u64,
) -> Result<DevnetSignatureResolution, DevnetSignatureStatusError> {
    if response.error.is_some() {
        return Err(DevnetSignatureStatusError::RpcRejected);
    }

    let result = response
        .result
        .ok_or(DevnetSignatureStatusError::InvalidResponse)?;
    if result.value.len() != 1 {
        return Err(DevnetSignatureStatusError::InvalidResponse);
    }

    let Some(status) = result.value.into_iter().next().flatten() else {
        return if current_block_height > last_valid_block_height {
            Ok(DevnetSignatureResolution::Expired)
        } else {
            Ok(DevnetSignatureResolution::Pending)
        };
    };

    if status.err.is_some() {
        return Ok(DevnetSignatureResolution::Failed { slot: status.slot });
    }

    match status.confirmation_status.as_deref() {
        Some("processed") => Ok(DevnetSignatureResolution::Processed { slot: status.slot }),
        Some("confirmed") => Ok(DevnetSignatureResolution::Confirmed { slot: status.slot }),
        Some("finalized") => Ok(DevnetSignatureResolution::Finalized { slot: status.slot }),
        _ => Err(DevnetSignatureStatusError::UnknownConfirmationStatus),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        request_json, resolve_response_json, DevnetSignatureResolution,
        DevnetSignatureStatusError,
    };

    #[test]
    fn request_is_read_only_and_history_search_is_disabled() {
        let request = request_json("example-public-signature").expect("request should serialize");
        let value: serde_json::Value = serde_json::from_str(&request).expect("request should parse");

        assert_eq!(value["method"], "getSignatureStatuses");
        assert_eq!(value["params"][0][0], "example-public-signature");
        assert_eq!(value["params"][1]["searchTransactionHistory"], false);
    }

    #[test]
    fn missing_status_is_pending_before_expiry() {
        let response = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":[null]},"id":4}"#;

        assert_eq!(
            resolve_response_json(response, 99, 100),
            Ok(DevnetSignatureResolution::Pending)
        );
    }

    #[test]
    fn missing_status_is_expired_after_blockhash_boundary() {
        let response = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":[null]},"id":4}"#;

        assert_eq!(
            resolve_response_json(response, 101, 100),
            Ok(DevnetSignatureResolution::Expired)
        );
    }

    #[test]
    fn confirmed_and_finalized_states_are_distinct() {
        let confirmed = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":[{"slot":55,"confirmations":2,"err":null,"confirmationStatus":"confirmed"}]},"id":4}"#;
        let finalized = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":[{"slot":56,"confirmations":null,"err":null,"confirmationStatus":"finalized"}]},"id":4}"#;

        assert_eq!(
            resolve_response_json(confirmed, 90, 100),
            Ok(DevnetSignatureResolution::Confirmed { slot: 55 })
        );
        assert_eq!(
            resolve_response_json(finalized, 90, 100),
            Ok(DevnetSignatureResolution::Finalized { slot: 56 })
        );
    }

    #[test]
    fn execution_error_is_terminal_failure() {
        let response = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":[{"slot":57,"confirmations":1,"err":{"InstructionError":[0,"Custom"]},"confirmationStatus":"confirmed"}]},"id":4}"#;

        assert_eq!(
            resolve_response_json(response, 90, 100),
            Ok(DevnetSignatureResolution::Failed { slot: 57 })
        );
    }

    #[test]
    fn malformed_or_rejected_responses_fail_closed() {
        assert_eq!(
            resolve_response_json("not-json", 90, 100),
            Err(DevnetSignatureStatusError::InvalidResponse)
        );

        let rejected = r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"failed"},"id":4}"#;
        assert_eq!(
            resolve_response_json(rejected, 90, 100),
            Err(DevnetSignatureStatusError::RpcRejected)
        );
    }
}
