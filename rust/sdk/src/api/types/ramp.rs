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
