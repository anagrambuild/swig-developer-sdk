use super::ramp::{CryptoAmount, Environment, FiatAmount, Location, OrderRequest, Route};
use crate::api::error::ApiConversionError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuyQuote {
    pub spend: FiatAmount,
    pub receive: CryptoAmount,
    #[serde(alias = "total_fee")]
    pub total_fee: FiatAmount,
    #[serde(alias = "exchange_rate")]
    pub exchange_rate: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SellQuote {
    pub sell: CryptoAmount,
    pub receive: FiatAmount,
    #[serde(alias = "total_fee")]
    pub total_fee: FiatAmount,
    #[serde(alias = "exchange_rate")]
    pub exchange_rate: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Quote {
    pub route: Route,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buy: Option<BuyQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sell: Option<SellQuote>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetQuotesRequest {
    pub configuration_id: String,
    pub environment: Environment,
    pub location: Location,
    #[serde(flatten)]
    pub order: OrderRequest,
}

#[derive(Deserialize)]
pub(crate) struct GetQuotesResponse {
    #[serde(default)]
    pub quotes: Vec<Quote>,
}

pub(crate) fn validate_buy_quote_shape(
    value: &serde_json::Value,
) -> Result<(), ApiConversionError> {
    let fields = value
        .as_object()
        .ok_or(ApiConversionError::Invalid("malformed Ramp buy quote"))?;
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
        return Err(ApiConversionError::Invalid("malformed Ramp buy quote"));
    }
    Ok(())
}

pub(crate) fn validate_sell_quote_shape(
    value: &serde_json::Value,
) -> Result<(), ApiConversionError> {
    let fields = value
        .as_object()
        .ok_or(ApiConversionError::Invalid("malformed Ramp sell quote"))?;
    if !fields
        .get("receive")
        .is_some_and(serde_json::Value::is_object)
        || !fields.get("sell").is_some_and(serde_json::Value::is_object)
        || !fields
            .get("totalFee")
            .or_else(|| fields.get("total_fee"))
            .is_some_and(serde_json::Value::is_object)
    {
        return Err(ApiConversionError::Invalid("malformed Ramp sell quote"));
    }
    Ok(())
}
