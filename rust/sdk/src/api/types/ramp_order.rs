use super::{
    ramp::CryptoAmount,
    ramp_quote::{validate_buy_quote_shape, validate_sell_quote_shape, BuyQuote, SellQuote},
};
use crate::api::error::ApiConversionError;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;
use solana_signature::Signature;

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum OrderStatus {
    #[serde(rename = "RAMP_ORDER_STATUS_CREATING")]
    Creating,
    #[serde(rename = "RAMP_ORDER_STATUS_CREATION_UNCERTAIN")]
    CreationUncertain,
    #[serde(rename = "RAMP_ORDER_STATUS_AWAITING_CUSTOMER")]
    AwaitingCustomer,
    #[serde(rename = "RAMP_ORDER_STATUS_AWAITING_TRANSFER")]
    AwaitingTransfer,
    #[serde(rename = "RAMP_ORDER_STATUS_PROCESSING")]
    Processing,
    #[serde(rename = "RAMP_ORDER_STATUS_SETTLING")]
    Settling,
    #[serde(rename = "RAMP_ORDER_STATUS_SETTLED")]
    Settled,
    #[serde(rename = "RAMP_ORDER_STATUS_DECLINED")]
    Declined,
    #[serde(rename = "RAMP_ORDER_STATUS_CANCELLED")]
    Cancelled,
    #[serde(rename = "RAMP_ORDER_STATUS_FAILED")]
    Failed,
    #[serde(rename = "RAMP_ORDER_STATUS_REFUNDED")]
    Refunded,
}

impl TryFrom<String> for OrderStatus {
    type Error = ApiConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "RAMP_ORDER_STATUS_CREATING" => Ok(Self::Creating),
            "RAMP_ORDER_STATUS_CREATION_UNCERTAIN" => Ok(Self::CreationUncertain),
            "RAMP_ORDER_STATUS_AWAITING_CUSTOMER" => Ok(Self::AwaitingCustomer),
            "RAMP_ORDER_STATUS_AWAITING_TRANSFER" => Ok(Self::AwaitingTransfer),
            "RAMP_ORDER_STATUS_PROCESSING" => Ok(Self::Processing),
            "RAMP_ORDER_STATUS_SETTLING" => Ok(Self::Settling),
            "RAMP_ORDER_STATUS_SETTLED" => Ok(Self::Settled),
            "RAMP_ORDER_STATUS_DECLINED" => Ok(Self::Declined),
            "RAMP_ORDER_STATUS_CANCELLED" => Ok(Self::Cancelled),
            "RAMP_ORDER_STATUS_FAILED" => Ok(Self::Failed),
            "RAMP_ORDER_STATUS_REFUNDED" => Ok(Self::Refunded),
            _ => Err(ApiConversionError::Invalid("invalid Ramp order status")),
        }
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Deposit {
    #[serde_as(as = "DisplayFromStr")]
    pub address: Pubkey,
    pub amount: CryptoAmount,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum TransferState {
    #[serde(rename = "TRANSFER_STATE_PREPARED")]
    Prepared,
    #[serde(rename = "TRANSFER_STATE_SUBMITTED")]
    Submitted,
    #[serde(rename = "TRANSFER_STATE_LANDED")]
    Landed,
    #[serde(rename = "TRANSFER_STATE_FAILED")]
    Failed,
    #[serde(rename = "TRANSFER_STATE_EXPIRED")]
    Expired,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Transfer {
    #[serde(alias = "transfer_id")]
    pub transfer_id: String,
    pub state: TransferState,
    #[serde(alias = "solana_signature", skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub solana_signature: Option<Signature>,
    #[serde(alias = "expires_at")]
    pub expires_at: String,
}

impl TryFrom<String> for TransferState {
    type Error = ApiConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "TRANSFER_STATE_PREPARED" => Ok(Self::Prepared),
            "TRANSFER_STATE_SUBMITTED" => Ok(Self::Submitted),
            "TRANSFER_STATE_LANDED" => Ok(Self::Landed),
            "TRANSFER_STATE_FAILED" => Ok(Self::Failed),
            "TRANSFER_STATE_EXPIRED" => Ok(Self::Expired),
            _ => Err(ApiConversionError::Invalid("invalid Ramp transfer state")),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuyOrderDetails {
    pub quote: BuyQuote,
    #[serde(alias = "launch_url", skip_serializing_if = "Option::is_none")]
    pub launch_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SellOrderDetails {
    pub quote: SellQuote,
    #[serde(alias = "launch_url", skip_serializing_if = "Option::is_none")]
    pub launch_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit: Option<Deposit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer: Option<Transfer>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Order {
    pub id: String,
    pub status: OrderStatus,
    #[serde(alias = "created_at")]
    pub created_at: String,
    #[serde(alias = "updated_at")]
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buy: Option<BuyOrderDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sell: Option<SellOrderDetails>,
}

pub(crate) struct GetOrderRequest {
    pub order_id: String,
}

#[derive(Deserialize)]
pub(crate) struct OrderResponse {
    pub order: Order,
}

pub(crate) fn validate_deposit_shape(value: &serde_json::Value) -> Result<(), ApiConversionError> {
    if !value
        .as_object()
        .and_then(|fields| fields.get("amount"))
        .is_some_and(serde_json::Value::is_object)
    {
        return Err(ApiConversionError::Invalid("malformed Ramp deposit"));
    }
    Ok(())
}

pub(crate) fn validate_order_response_shape(
    value: &serde_json::Value,
) -> Result<(), ApiConversionError> {
    let order = value
        .as_object()
        .and_then(|fields| fields.get("order"))
        .and_then(serde_json::Value::as_object)
        .ok_or(ApiConversionError::Invalid("malformed Ramp order"))?;
    if let Some(buy) = order.get("buy").filter(|value| !value.is_null()) {
        let quote = buy
            .as_object()
            .and_then(|fields| fields.get("quote"))
            .ok_or(ApiConversionError::Invalid("malformed Ramp order"))?;
        validate_buy_quote_shape(quote)?;
    }
    if let Some(sell) = order.get("sell").filter(|value| !value.is_null()) {
        let sell = sell
            .as_object()
            .ok_or(ApiConversionError::Invalid("malformed Ramp order"))?;
        let quote = sell
            .get("quote")
            .ok_or(ApiConversionError::Invalid("malformed Ramp order"))?;
        validate_sell_quote_shape(quote)?;
        if let Some(deposit) = sell.get("deposit").filter(|value| !value.is_null()) {
            validate_deposit_shape(deposit)?;
        }
        if sell
            .get("transfer")
            .is_some_and(|transfer| !transfer.is_null() && !transfer.is_object())
        {
            return Err(ApiConversionError::Invalid("malformed Ramp transfer"));
        }
    }
    Ok(())
}
