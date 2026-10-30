use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{ExactSvmExtra, ExactSvmPaymentRequirement, X402ResourceInfoV2};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceInfo {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

impl From<&X402ResourceInfoV2> for ResourceInfo {
    fn from(value: &X402ResourceInfoV2) -> Self {
        Self {
            url: value.url.clone(),
            description: value.description.clone(),
            mime_type: value.mime_type.clone(),
            service_name: value.service_name.clone(),
            tags: value.tags.clone(),
            icon_url: value.icon_url.clone(),
        }
    }
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
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExactSvmExtraData {
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_blockhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_valid_block_height: Option<String>,
}

#[serde_as]
#[derive(Deserialize, Serialize)]
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

impl From<&ExactSvmPaymentRequirement> for ExactSvmRequirement {
    fn from(value: &ExactSvmPaymentRequirement) -> Self {
        Self {
            scheme: value.scheme.clone(),
            network: value.network.clone(),
            amount: value.amount.clone(),
            asset: value.asset,
            pay_to: value.pay_to,
            max_timeout_seconds: value.max_timeout_seconds,
            extra: ExactSvmExtraData {
                fee_payer: value.extra.fee_payer,
                memo: value.extra.memo.clone(),
                recent_blockhash: value.extra.recent_blockhash.clone(),
                last_valid_block_height: value.extra.last_valid_block_height.clone(),
            },
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ExactSvmPayload {
    pub transaction: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymentPayload {
    pub x402_version: u8,
    pub resource: ResourceInfo,
    pub accepted: ExactSvmRequirement,
    pub payload: ExactSvmPayload,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub extensions: Value,
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
