use serde::Deserialize;
use serde_json::{Map, Value};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{ExactSvmExtra, ExactSvmPaymentRequirement, X402ResourceInfoV2};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceInfo {
    pub url: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub service_name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub icon_url: Option<String>,
}

impl From<ResourceInfo> for X402ResourceInfoV2 {
    fn from(value: ResourceInfo) -> Self {
        Self {
            url: value.url,
            description: value.description,
            mime_type: value.mime_type,
            service_name: value.service_name,
            tags: value.tags,
            icon_url: value.icon_url,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymentRequiredHeader {
    pub x402_version: u8,
    pub error: Option<String>,
    pub resource: ResourceInfo,
    pub accepts: Vec<Value>,
    pub extensions: Option<Map<String, Value>>,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExactSvmExtraData {
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub memo: Option<String>,
    pub recent_blockhash: Option<String>,
    pub last_valid_block_height: Option<String>,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExactSvmRequirement {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    #[serde_as(as = "DisplayFromStr")]
    pub asset: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub pay_to: Pubkey,
    pub max_timeout_seconds: f64,
    pub extra: ExactSvmExtraData,
}

impl From<ExactSvmRequirement> for ExactSvmPaymentRequirement {
    fn from(value: ExactSvmRequirement) -> Self {
        Self {
            scheme: value.scheme,
            network: value.network,
            amount: value.amount,
            asset: value.asset,
            pay_to: value.pay_to,
            max_timeout_seconds: value.max_timeout_seconds,
            extra: ExactSvmExtra {
                fee_payer: value.extra.fee_payer,
                memo: value.extra.memo,
                recent_blockhash: value.extra.recent_blockhash,
                last_valid_block_height: value.extra.last_valid_block_height,
            },
        }
    }
}
