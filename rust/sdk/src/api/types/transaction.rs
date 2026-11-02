use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use super::encoding::SignatureScheme;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSignatureRequest {
    pub scheme: SignatureScheme,
    pub signer: String,
    #[serde(alias = "message_hash")]
    pub message_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    pub counter: u32,
}

use super::{
    encoding::{PreparedTransactionKind, TransactionEncoding},
    network::Network,
    participant::ParticipantSetApprovalPlan,
};
use crate::api::error::ApiConversionError;
use serde_json::Value;
use solana_pubkey::Pubkey;
#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WalletAddressInfo {
    #[serde(alias = "swig_config_address")]
    #[serde_as(as = "DisplayFromStr")]
    pub swig_config_address: Pubkey,
    #[serde(alias = "wallet_address")]
    #[serde_as(as = "DisplayFromStr")]
    pub wallet_address: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedTransaction {
    #[serde(alias = "unsignedTransaction", alias = "unsigned_transaction")]
    pub transaction: String,
    #[serde(alias = "transaction_encoding")]
    pub transaction_encoding: Option<TransactionEncoding>,
    pub wallet: Option<WalletAddressInfo>,
    #[serde(alias = "expires_at")]
    pub expires_at: Option<String>,
    pub network: Option<Network>,
    #[serde(alias = "recent_blockhash")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub recent_blockhash: Option<solana_hash::Hash>,
    #[serde(default, skip_serializing)]
    pub kind: Option<PreparedTransactionKind>,
    #[serde(default, alias = "signature_requests")]
    pub signature_requests: Vec<ClientSignatureRequest>,
    #[serde(alias = "participant_set_approval_plan")]
    pub participant_set_approval_plan: Option<ParticipantSetApprovalPlan>,
}

pub(crate) fn validate_prepared_transaction_shape(value: &Value) -> Result<(), ApiConversionError> {
    let fields = value.as_object().ok_or(ApiConversionError::Invalid(
        "malformed prepared transaction",
    ))?;
    if fields
        .get("wallet")
        .is_some_and(|wallet| !wallet.is_null() && !wallet.is_object())
    {
        return Err(ApiConversionError::Invalid(
            "malformed prepared transaction",
        ));
    }
    for field in ["signatureRequests", "signature_requests"] {
        if fields
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|requests| requests.iter().any(|request| !request.is_object()))
        {
            return Err(ApiConversionError::Invalid(
                "malformed prepared transaction",
            ));
        }
    }
    for field in [
        "participantSetApprovalPlan",
        "participant_set_approval_plan",
    ] {
        let Some(plan) = fields.get(field).filter(|plan| !plan.is_null()) else {
            continue;
        };
        let plan = plan.as_object().ok_or(ApiConversionError::Invalid(
            "malformed prepared transaction",
        ))?;
        if let Some(members) = plan.get("members").and_then(Value::as_array) {
            for member in members {
                let member = member.as_object().ok_or(ApiConversionError::Invalid(
                    "malformed prepared transaction",
                ))?;
                if let Some(authority) = member.get("authority") {
                    let fields = authority.as_object().ok_or(ApiConversionError::Invalid(
                        "malformed prepared transaction",
                    ))?;
                    if fields.values().any(|value| !value.is_object()) {
                        return Err(ApiConversionError::Invalid(
                            "malformed prepared transaction",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
