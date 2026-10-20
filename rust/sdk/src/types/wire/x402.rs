use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{ExactSvmExtra, ExactSvmPaymentRequirement, X402ResourceInfoV2};

use super::WireError;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymentRequiredHeaderWire {
    pub x402_version: u8,
    pub error: Option<String>,
    pub resource: X402ResourceInfoV2,
    pub accepts: Vec<Value>,
    pub extensions: Option<Map<String, Value>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExactSvmExtraWire {
    pub fee_payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_blockhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_valid_block_height: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExactSvmRequirementWire {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    pub asset: String,
    pub pay_to: String,
    pub max_timeout_seconds: f64,
    pub extra: ExactSvmExtraWire,
}

impl TryFrom<ExactSvmRequirementWire> for ExactSvmPaymentRequirement {
    type Error = WireError;

    fn try_from(value: ExactSvmRequirementWire) -> Result<Self, Self::Error> {
        Ok(Self {
            scheme: value.scheme,
            network: value.network,
            amount: value.amount,
            asset: value
                .asset
                .parse()
                .map_err(|_| WireError::Invalid("invalid payment asset"))?,
            pay_to: value
                .pay_to
                .parse()
                .map_err(|_| WireError::Invalid("invalid payment recipient"))?,
            max_timeout_seconds: value.max_timeout_seconds,
            extra: ExactSvmExtra {
                fee_payer: value
                    .extra
                    .fee_payer
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid payment fee payer"))?,
                memo: value.extra.memo,
                recent_blockhash: value.extra.recent_blockhash,
                last_valid_block_height: value.extra.last_valid_block_height,
            },
        })
    }
}
