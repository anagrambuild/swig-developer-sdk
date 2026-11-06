use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::types::{
    authority::WalletAuthority,
    network::Network,
    prepared_batch::{validate_prepared_batch_shape, PreparedBatch},
    transaction::WalletAddressInfo,
};
use crate::{
    http::{HttpClient, HttpError},
    ReservationAddressOptions,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum AddressOptions {
    AccountIndex(u32),
    Salt(String),
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReservationRequest {
    network: Network,
    root_authority: WalletAuthority,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_options: Option<AddressOptions>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_payer: Option<Pubkey>,
}

#[derive(Deserialize)]
pub(crate) struct ReservationResponse {
    pub wallet: WalletAddressInfo,
    pub network: Network,
}

impl From<ReservationAddressOptions> for AddressOptions {
    fn from(value: ReservationAddressOptions) -> Self {
        match value {
            ReservationAddressOptions::AccountIndex(index) => Self::AccountIndex(index),
            ReservationAddressOptions::Salt(salt) => Self::Salt(STANDARD.encode(salt)),
        }
    }
}

pub(crate) async fn reserve_address(
    http: &HttpClient,
    network: Network,
    root_authority: WalletAuthority,
    address_options: Option<ReservationAddressOptions>,
) -> Result<ReservationResponse, HttpError> {
    let body = serde_json::to_vec(&ReservationRequest {
        network,
        root_authority,
        address_options: address_options.map(Into::into),
        fee_payer: None,
    })
    .map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/transaction/api/wallet/reserve", body).await?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let serde_json::Value::Object(fields) = &mut value {
        if let Some(data) = fields.remove("data") {
            value = data;
        }
    }
    if !value.is_object()
        || !value
            .get("wallet")
            .is_some_and(serde_json::Value::is_object)
    {
        return Err(HttpError::InvalidResponse("malformed wallet reservation"));
    }
    serde_json::from_value(value)
        .map_err(|_| HttpError::InvalidResponse("malformed wallet reservation"))
}

pub(crate) async fn prepare_create_reserved(
    http: &HttpClient,
    network: Network,
    root_authority: WalletAuthority,
    address_options: Option<ReservationAddressOptions>,
    fee_payer: Pubkey,
) -> Result<PreparedBatch, HttpError> {
    let body = serde_json::to_vec(&ReservationRequest {
        network,
        root_authority,
        address_options: address_options.map(Into::into),
        fee_payer: Some(fee_payer),
    })
    .map_err(|_| HttpError::RequestFailed)?;
    let bytes = http
        .post("/transaction/api/wallet/create-reserved", body)
        .await?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let serde_json::Value::Object(fields) = &mut value {
        if let Some(data) = fields.remove("data") {
            value = data;
        }
    }
    validate_prepared_batch_shape(&value)
        .map_err(|_| HttpError::InvalidResponse("malformed reserved wallet creation"))?;
    serde_json::from_value(value)
        .map_err(|_| HttpError::InvalidResponse("malformed reserved wallet creation"))
}
