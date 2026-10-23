use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum PaymasterKindWire {
    Name(PaymasterKindName),
    Number(u32),
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum PaymasterKindName {
    Api,
    Idp,
}

impl TryFrom<String> for PaymasterKindName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "PAYMASTER_KIND_API" | "api" => Ok(Self::Api),
            "PAYMASTER_KIND_IDP" | "idp" => Ok(Self::Idp),
            _ => Err("invalid paymaster kind"),
        }
    }
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymasterBalanceWire {
    pub kind: PaymasterKindWire,
    pub address: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default, alias = "balance_lamports")]
    #[serde_as(as = "DisplayFromStr")]
    pub balance_lamports: u64,
    #[serde(default, alias = "balance_sol")]
    pub balance_sol: f64,
}

#[derive(Deserialize)]
pub(crate) struct PaymasterConfigurationWire {
    #[serde(default)]
    pub configured: bool,
}

#[derive(Serialize)]
pub(crate) struct SponsorRequest {
    pub base58_encoded_transaction: String,
    pub network: &'static str,
    #[serde(rename = "idempotencyKey", skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[serde_as]
#[derive(Deserialize)]
pub(crate) struct SubmittedTransactionWire {
    #[serde(alias = "requestId")]
    pub request_id: String,
    pub signature: String,
    #[serde(default, alias = "spentByPaymaster")]
    #[serde_as(as = "DisplayFromStr")]
    pub spent_by_paymaster: i64,
}

#[derive(Serialize)]
pub(crate) struct SponsorBundleRequest {
    pub base58_encoded_transactions: Vec<String>,
    pub network: &'static str,
    #[serde(rename = "idempotencyKey", skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[serde_as]
#[derive(Deserialize)]
pub(crate) struct SubmittedTransactionBundleWire {
    #[serde(alias = "requestId")]
    pub request_id: String,
    #[serde(alias = "bundleId")]
    pub bundle_id: String,
    pub signatures: Vec<String>,
    #[serde(default, alias = "estimatedSpentByPaymaster")]
    #[serde_as(as = "DisplayFromStr")]
    pub estimated_spent_by_paymaster: u64,
}
