use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use crate::api::error::ApiConversionError;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum CryptoAsset {
    Sol {},
    Token {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
    },
}

#[derive(Clone, Copy, Serialize)]
pub(crate) enum Environment {
    #[serde(rename = "RAMP_ENVIRONMENT_SANDBOX")]
    Sandbox,
    #[serde(rename = "RAMP_ENVIRONMENT_PRODUCTION")]
    Production,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum Direction {
    Buy,
    Sell,
}

impl TryFrom<String> for Direction {
    type Error = ApiConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "RAMP_DIRECTION_BUY" | "buy" => Ok(Self::Buy),
            "RAMP_DIRECTION_SELL" | "sell" => Ok(Self::Sell),
            _ => Err(ApiConversionError::Invalid("invalid Ramp direction")),
        }
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FiatAmount {
    #[serde(alias = "currency_code")]
    pub currency_code: String,
    #[serde(alias = "minor_units")]
    #[serde_as(as = "DisplayFromStr")]
    pub minor_units: u64,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CryptoAmount {
    pub asset: CryptoAsset,
    #[serde(alias = "base_units")]
    #[serde_as(as = "DisplayFromStr")]
    pub base_units: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Route {
    pub provider: String,
    #[serde(alias = "payment_method")]
    pub payment_method: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Location {
    #[serde(alias = "country_code")]
    pub country_code: String,
    #[serde(alias = "subdivision_code", skip_serializing_if = "Option::is_none")]
    pub subdivision_code: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum OrderRequest {
    Buy {
        spend: FiatAmount,
        receive: CryptoAsset,
    },
    Sell {
        sell: CryptoAmount,
        #[serde(alias = "receive_fiat_currency_code")]
        receive_fiat_currency_code: String,
    },
}
