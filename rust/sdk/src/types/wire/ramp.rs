use super::WireError;
use crate::{
    CryptoAsset, RampBuyQuote, RampLocation, RampOrderRequest, RampQuote, RampQuoteDetails,
    RampRoute, RampSellQuote,
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
