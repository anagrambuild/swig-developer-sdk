use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_pubkey::Pubkey;

use crate::Network;

#[derive(Clone)]
pub struct ExactSvmExtra {
    pub fee_payer: Pubkey,
    pub memo: Option<String>,
    pub recent_blockhash: Option<String>,
    pub last_valid_block_height: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X402ResourceInfoV2 {
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

#[derive(Clone)]
pub struct ExactSvmPaymentRequirement {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    pub asset: Pubkey,
    pub pay_to: Pubkey,
    pub max_timeout_seconds: f64,
    pub extra: ExactSvmExtra,
}

#[derive(Clone)]
// The public contract exposes the exact-SVM offer directly, without a Box.
#[allow(clippy::large_enum_variant)]
pub enum PaymentRequirement {
    ExactSvm(ExactSvmPaymentRequirement),
    Unsupported,
}

#[derive(Clone)]
pub struct PaymentRequiredV2 {
    pub x402_version: u8,
    pub error: Option<String>,
    pub resource: X402ResourceInfoV2,
    pub accepts: Vec<PaymentRequirement>,
    pub extensions: Value,
}

pub(crate) fn validate_required(required: &PaymentRequiredV2) -> Result<(), &'static str> {
    if required.x402_version != 2 || required.accepts.is_empty() || required.resource.url.is_empty()
    {
        return Err("invalid V2 payment requirements");
    }
    if !required.extensions.is_null() && !required.extensions.is_object() {
        return Err("extensions must be an object or null");
    }
    if required
        .resource
        .service_name
        .as_ref()
        .is_some_and(|value| !printable_label(value))
        || required
            .resource
            .tags
            .as_ref()
            .is_some_and(|tags| tags.len() > 5 || tags.iter().any(|tag| !printable_label(tag)))
        || required
            .resource
            .icon_url
            .as_ref()
            .is_some_and(|value| value.encode_utf16().count() > 2048)
    {
        return Err("invalid payment resource metadata");
    }
    for requirement in &required.accepts {
        if let PaymentRequirement::ExactSvm(offer) = requirement {
            validate_offer(offer)?;
        }
    }
    Ok(())
}

fn printable_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

pub(crate) fn validate_offer(offer: &ExactSvmPaymentRequirement) -> Result<(), &'static str> {
    if offer.scheme != "exact"
        || ![
            solana_network(Network::Devnet),
            solana_network(Network::Mainnet),
        ]
        .contains(&offer.network.as_str())
    {
        return Err("unsupported payment requirement");
    }
    if offer.amount.is_empty() || !offer.amount.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("payment amount must be a decimal string");
    }
    if !offer.max_timeout_seconds.is_finite() || offer.max_timeout_seconds <= 0.0 {
        return Err("payment timeout must be positive");
    }
    if offer
        .extra
        .last_valid_block_height
        .as_ref()
        .is_some_and(|height| {
            height.is_empty() || !height.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err("last valid block height must be a decimal string");
    }
    Ok(())
}

pub(crate) fn solana_network(network: Network) -> &'static str {
    match network {
        Network::Devnet => "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
        Network::Mainnet => "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
    }
}
