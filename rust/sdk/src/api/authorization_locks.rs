use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
use solana_pubkey::Pubkey;

use super::types::{
    authority::WalletAuthority,
    network::Network,
    transaction::{validate_prepared_transaction_shape, PreparedTransaction},
};
use crate::http::{HttpClient, HttpError};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddAuthorizationLockRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<Pubkey>,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    #[serde(flatten)]
    pub expiry: AuthorizationLockExpiry,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AuthorizationLockExpiry {
    DurationSlots(#[serde_as(as = "DisplayFromStr")] u64),
    ExpirySlot(#[serde_as(as = "DisplayFromStr")] u64),
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoveAuthorizationLockRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub lock_id: u32,
}

#[derive(Deserialize)]
struct PreparationResponse {
    transaction: PreparedTransaction,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizationLock {
    #[serde(default, alias = "lock_id")]
    pub lock_id: u32,
    #[serde(default, alias = "role_id")]
    pub role_id: u32,
    #[serde(alias = "asset_account")]
    #[serde_as(as = "DisplayFromStr")]
    pub asset_account: Pubkey,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub mint: Option<Pubkey>,
    #[serde_as(as = "PickFirst<(DisplayFromStr, _)>")]
    pub amount: u64,
    #[serde(alias = "expiry_slot")]
    #[serde_as(as = "PickFirst<(DisplayFromStr, _)>")]
    pub expiry_slot: u64,
    #[serde(default)]
    pub active: bool,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizationLocksResponse {
    #[serde(default)]
    pub locks: Vec<AuthorizationLock>,
    #[serde(alias = "current_slot")]
    #[serde_as(as = "PickFirst<(DisplayFromStr, _)>")]
    pub current_slot: u64,
    #[serde(default, alias = "next_lock_id")]
    pub next_lock_id: u32,
}

pub(crate) async fn prepare_add_authorization_lock(
    http: &HttpClient,
    request: AddAuthorizationLockRequest,
) -> Result<PreparedTransaction, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http
        .post("/transaction/wallet/authorization-lock/add", body)
        .await?;
    decode_preparation(&bytes)
}

pub(crate) async fn prepare_remove_authorization_lock(
    http: &HttpClient,
    request: RemoveAuthorizationLockRequest,
) -> Result<PreparedTransaction, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http
        .post("/transaction/wallet/authorization-lock/remove", body)
        .await?;
    decode_preparation(&bytes)
}

pub(crate) async fn get_authorization_locks(
    http: &HttpClient,
    address: &Pubkey,
    network: crate::Network,
    include_expired: bool,
) -> Result<AuthorizationLocksResponse, HttpError> {
    let network = match network {
        crate::Network::Devnet => "NETWORK_DEVNET",
        crate::Network::Mainnet => "NETWORK_MAINNET",
    };
    let path = format!("/transaction/wallet/{address}/authorization-locks?network={network}&include_expired={include_expired}");
    let bytes = http.get(&path).await?;
    let payload = decode_object(&bytes)?;
    if let Some(locks) = payload.get("locks") {
        let locks = locks
            .as_array()
            .ok_or(HttpError::InvalidResponse("malformed authorization locks"))?;
        if locks.iter().any(|lock| !lock.is_object()) {
            return Err(HttpError::InvalidResponse("malformed authorization lock"));
        }
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed authorization locks"))
}

fn decode_object(bytes: &[u8]) -> Result<Value, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse(
            "malformed authorization lock response",
        ));
    }
    Ok(payload)
}

fn decode_preparation(bytes: &[u8]) -> Result<PreparedTransaction, HttpError> {
    let payload = decode_object(bytes)?;
    let transaction = payload
        .get("transaction")
        .ok_or(HttpError::InvalidResponse("malformed prepared transaction"))?;
    validate_prepared_transaction_shape(transaction)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))?;
    let response: PreparationResponse = serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))?;
    Ok(response.transaction)
}
