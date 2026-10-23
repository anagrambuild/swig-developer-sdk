use super::NetworkWire;
use super::WireError;
use super::{transaction::PreparedTransactionWire, AuthorityWire};
use crate::{
    CryptoAsset, RampBuyQuote, RampLocation, RampOrderRequest, RampQuote, RampQuoteDetails,
    RampRoute, RampSellQuote,
};
use crate::{
    RampBuyOrder, RampDeposit, RampOrder, RampOrderStatus, RampSellOrder, RampTransfer,
    RampTransferState,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum CryptoAssetWire {
    Sol {},
    Token { mint: String },
}

pub(crate) fn validate_buy_quote_shape(value: &serde_json::Value) -> Result<(), WireError> {
    let fields = value
        .as_object()
        .ok_or(WireError::Invalid("malformed Ramp buy quote"))?;
    if !fields
        .get("spend")
        .is_some_and(serde_json::Value::is_object)
        || !fields
            .get("receive")
            .is_some_and(serde_json::Value::is_object)
        || !fields
            .get("totalFee")
            .or_else(|| fields.get("total_fee"))
            .is_some_and(serde_json::Value::is_object)
    {
        return Err(WireError::Invalid("malformed Ramp buy quote"));
    }
    Ok(())
}

pub(crate) fn validate_sell_quote_shape(value: &serde_json::Value) -> Result<(), WireError> {
    let fields = value
        .as_object()
        .ok_or(WireError::Invalid("malformed Ramp sell quote"))?;
    if !fields
        .get("receive")
        .is_some_and(serde_json::Value::is_object)
        || !fields.get("sell").is_some_and(serde_json::Value::is_object)
        || !fields
            .get("totalFee")
            .or_else(|| fields.get("total_fee"))
            .is_some_and(serde_json::Value::is_object)
    {
        return Err(WireError::Invalid("malformed Ramp sell quote"));
    }
    Ok(())
}

pub(crate) fn validate_deposit_shape(value: &serde_json::Value) -> Result<(), WireError> {
    if !value
        .as_object()
        .and_then(|fields| fields.get("amount"))
        .is_some_and(serde_json::Value::is_object)
    {
        return Err(WireError::Invalid("malformed Ramp deposit"));
    }
    Ok(())
}

pub(crate) fn validate_order_response_shape(value: &serde_json::Value) -> Result<(), WireError> {
    let order = value
        .as_object()
        .and_then(|fields| fields.get("order"))
        .and_then(serde_json::Value::as_object)
        .ok_or(WireError::Invalid("malformed Ramp order"))?;
    if let Some(buy) = order.get("buy").filter(|value| !value.is_null()) {
        let quote = buy
            .as_object()
            .and_then(|fields| fields.get("quote"))
            .ok_or(WireError::Invalid("malformed Ramp order"))?;
        validate_buy_quote_shape(quote)?;
    }
    if let Some(sell) = order.get("sell").filter(|value| !value.is_null()) {
        let sell = sell
            .as_object()
            .ok_or(WireError::Invalid("malformed Ramp order"))?;
        let quote = sell
            .get("quote")
            .ok_or(WireError::Invalid("malformed Ramp order"))?;
        validate_sell_quote_shape(quote)?;
        if let Some(deposit) = sell.get("deposit").filter(|value| !value.is_null()) {
            validate_deposit_shape(deposit)?;
        }
        if sell
            .get("transfer")
            .is_some_and(|transfer| !transfer.is_null() && !transfer.is_object())
        {
            return Err(WireError::Invalid("malformed Ramp transfer"));
        }
    }
    Ok(())
}

impl TryFrom<CryptoAssetWire> for CryptoAsset {
    type Error = WireError;

    fn try_from(value: CryptoAssetWire) -> Result<Self, Self::Error> {
        match value {
            CryptoAssetWire::Sol {} => Ok(Self::Sol),
            CryptoAssetWire::Token { mint } => Ok(Self::Token {
                mint: mint
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid Ramp token mint"))?,
            }),
        }
    }
}

impl From<CryptoAsset> for CryptoAssetWire {
    fn from(value: CryptoAsset) -> Self {
        match value {
            CryptoAsset::Sol => Self::Sol {},
            CryptoAsset::Token { mint } => Self::Token {
                mint: mint.to_string(),
            },
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RampQuoteWire {
    pub route: RampRoute,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buy: Option<RampBuyQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sell: Option<RampSellQuote>,
}

impl TryFrom<RampQuoteWire> for RampQuote {
    type Error = WireError;

    fn try_from(value: RampQuoteWire) -> Result<Self, Self::Error> {
        let details = match (value.buy, value.sell) {
            (Some(buy), None) => RampQuoteDetails::Buy(buy),
            (None, Some(sell)) => RampQuoteDetails::Sell(sell),
            _ => {
                return Err(WireError::Invalid(
                    "Ramp quote requires exactly one direction",
                ))
            }
        };
        Ok(Self {
            route: value.route,
            details,
        })
    }
}

impl From<RampQuote> for RampQuoteWire {
    fn from(value: RampQuote) -> Self {
        let (buy, sell) = match value.details {
            RampQuoteDetails::Buy(buy) => (Some(buy), None),
            RampQuoteDetails::Sell(sell) => (None, Some(sell)),
        };
        Self {
            route: value.route,
            buy,
            sell,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuotesRequest {
    pub configuration_id: String,
    pub environment: &'static str,
    pub location: RampLocation,
    #[serde(flatten)]
    pub order: RampOrderRequest,
}

#[derive(Deserialize)]
pub(crate) struct QuotesResponse {
    #[serde(default)]
    pub quotes: Vec<RampQuote>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum TransferStateWire {
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RampTransferWire {
    #[serde(alias = "transfer_id")]
    pub transfer_id: String,
    pub state: TransferStateWire,
    #[serde(alias = "solana_signature", skip_serializing_if = "Option::is_none")]
    pub solana_signature: Option<String>,
    #[serde(alias = "expires_at")]
    pub expires_at: String,
}

impl TryFrom<RampTransferWire> for RampTransfer {
    type Error = WireError;

    fn try_from(value: RampTransferWire) -> Result<Self, Self::Error> {
        if value.transfer_id.is_empty() || value.expires_at.is_empty() {
            return Err(WireError::Invalid("incomplete Ramp transfer"));
        }
        let signature = value
            .solana_signature
            .map(|signature| {
                signature
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid Ramp transfer signature"))
            })
            .transpose()?;
        let state = match (value.state, signature) {
            (TransferStateWire::Prepared, None) => RampTransferState::Prepared,
            (TransferStateWire::Expired, None) => RampTransferState::Expired,
            (TransferStateWire::Submitted, Some(signature)) => {
                RampTransferState::Submitted { signature }
            }
            (TransferStateWire::Landed, Some(signature)) => RampTransferState::Landed { signature },
            (TransferStateWire::Failed, Some(signature)) => RampTransferState::Failed { signature },
            _ => {
                return Err(WireError::Invalid(
                    "Ramp transfer state and signature disagree",
                ))
            }
        };
        Ok(Self {
            transfer_id: value.transfer_id,
            state,
            expires_at: value.expires_at,
        })
    }
}

impl From<RampTransfer> for RampTransferWire {
    fn from(value: RampTransfer) -> Self {
        let (state, signature) = match value.state {
            RampTransferState::Prepared => (TransferStateWire::Prepared, None),
            RampTransferState::Expired => (TransferStateWire::Expired, None),
            RampTransferState::Submitted { signature } => {
                (TransferStateWire::Submitted, Some(signature))
            }
            RampTransferState::Landed { signature } => (TransferStateWire::Landed, Some(signature)),
            RampTransferState::Failed { signature } => (TransferStateWire::Failed, Some(signature)),
        };
        Self {
            transfer_id: value.transfer_id,
            state,
            solana_signature: signature.map(|signature| signature.to_string()),
            expires_at: value.expires_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuyOrderDetails {
    pub quote: RampBuyQuote,
    #[serde(alias = "launch_url", skip_serializing_if = "Option::is_none")]
    pub launch_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SellOrderDetails {
    pub quote: RampSellQuote,
    #[serde(alias = "launch_url", skip_serializing_if = "Option::is_none")]
    pub launch_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit: Option<RampDeposit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer: Option<RampTransfer>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RampOrderWire {
    pub id: String,
    pub status: RampOrderStatus,
    #[serde(alias = "created_at")]
    pub created_at: String,
    #[serde(alias = "updated_at")]
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buy: Option<BuyOrderDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sell: Option<SellOrderDetails>,
}

impl TryFrom<RampOrderWire> for RampOrder {
    type Error = WireError;

    fn try_from(value: RampOrderWire) -> Result<Self, Self::Error> {
        if value.id.is_empty() || value.created_at.is_empty() || value.updated_at.is_empty() {
            return Err(WireError::Invalid("incomplete Ramp order"));
        }
        match (value.buy, value.sell) {
            (Some(details), None) => Ok(Self::Buy(RampBuyOrder {
                id: value.id,
                status: value.status,
                created_at: value.created_at,
                updated_at: value.updated_at,
                quote: details.quote,
                launch_url: details.launch_url.filter(|url| !url.is_empty()),
            })),
            (None, Some(details)) => Ok(Self::Sell(RampSellOrder {
                id: value.id,
                status: value.status,
                created_at: value.created_at,
                updated_at: value.updated_at,
                quote: details.quote,
                launch_url: details.launch_url.filter(|url| !url.is_empty()),
                deposit: details.deposit,
                transfer: details.transfer,
            })),
            _ => Err(WireError::Invalid(
                "Ramp order requires exactly one direction",
            )),
        }
    }
}

impl From<RampOrder> for RampOrderWire {
    fn from(value: RampOrder) -> Self {
        match value {
            RampOrder::Buy(order) => Self {
                id: order.id,
                status: order.status,
                created_at: order.created_at,
                updated_at: order.updated_at,
                buy: Some(BuyOrderDetails {
                    quote: order.quote,
                    launch_url: order.launch_url,
                }),
                sell: None,
            },
            RampOrder::Sell(order) => Self {
                id: order.id,
                status: order.status,
                created_at: order.created_at,
                updated_at: order.updated_at,
                buy: None,
                sell: Some(SellOrderDetails {
                    quote: order.quote,
                    launch_url: order.launch_url,
                    deposit: order.deposit,
                    transfer: order.transfer,
                }),
            },
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct OrderResponse {
    pub order: RampOrder,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrderContextRequest {
    pub customer_id: String,
    pub swig_config_address: String,
    pub network: NetworkWire,
    pub location: RampLocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateOrderRequest {
    pub request_id: String,
    pub configuration_id: String,
    pub environment: &'static str,
    pub context: OrderContextRequest,
    pub route: RampRoute,
    #[serde(flatten)]
    pub order: RampOrderRequest,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareRequest {
    pub requester_authority: AuthorityWire,
    pub fee_payer: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareResponse {
    #[serde(alias = "prepared_transfer")]
    pub prepared_transfer: PreparedTransferResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedTransferResponse {
    pub transfer: RampTransfer,
    #[serde(alias = "prepared_transaction")]
    pub prepared_transaction: PreparedTransactionWire,
    pub deposit: RampDeposit,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitRequest {
    pub transfer_id: String,
    pub signed_transaction: String,
}

#[derive(Deserialize)]
pub(crate) struct SubmitResponse {
    pub transfer: RampTransfer,
}

impl TryFrom<String> for TransferStateWire {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "TRANSFER_STATE_PREPARED" => Ok(Self::Prepared),
            "TRANSFER_STATE_SUBMITTED" => Ok(Self::Submitted),
            "TRANSFER_STATE_LANDED" => Ok(Self::Landed),
            "TRANSFER_STATE_FAILED" => Ok(Self::Failed),
            "TRANSFER_STATE_EXPIRED" => Ok(Self::Expired),
            _ => Err("invalid Ramp transfer state"),
        }
    }
}
