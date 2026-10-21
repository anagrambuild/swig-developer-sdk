use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};

use crate::{ClientSignatureRequest, PreparedTransaction, WalletAddressInfo};

use super::{
    decode_hex32, decode_transaction, encode_transaction, validate_authority_shape, AuthorityWire,
    NetworkWire, OptionalPreparedKind, ParticipantSetApprovalPlanWire, SignatureSchemeWire,
    TransactionEncodingWire, WireError,
};

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSignatureRequestWire {
    pub scheme: SignatureSchemeWire,
    pub signer: String,
    #[serde(alias = "message_hash")]
    pub message_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    pub counter: u32,
}

impl TryFrom<ClientSignatureRequestWire> for ClientSignatureRequest {
    type Error = WireError;

    fn try_from(value: ClientSignatureRequestWire) -> Result<Self, Self::Error> {
        let authority = match value.scheme {
            SignatureSchemeWire::Secp256k1 => AuthorityWire::Secp256k1 {
                public_key: value.signer,
            },
            SignatureSchemeWire::Secp256r1 => AuthorityWire::Secp256r1 {
                public_key: value.signer,
            },
        }
        .try_into()?;
        Ok(Self {
            authority,
            message_hash: decode_hex32(&value.message_hash)?,
            slot: value.slot,
            counter: value.counter,
        })
    }
}

impl TryFrom<ClientSignatureRequest> for ClientSignatureRequestWire {
    type Error = WireError;

    fn try_from(value: ClientSignatureRequest) -> Result<Self, Self::Error> {
        let (scheme, signer) = match AuthorityWire::try_from(value.authority)? {
            AuthorityWire::Secp256k1 { public_key } => (SignatureSchemeWire::Secp256k1, public_key),
            AuthorityWire::Secp256r1 { public_key } => (SignatureSchemeWire::Secp256r1, public_key),
            _ => return Err(WireError::Invalid("unsupported client signature authority")),
        };
        Ok(Self {
            scheme,
            signer,
            message_hash: hex::encode(value.message_hash),
            slot: value.slot,
            counter: value.counter,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WalletAddressInfoWire {
    #[serde(alias = "swig_config_address")]
    pub swig_config_address: String,
    #[serde(alias = "wallet_address")]
    pub wallet_address: String,
}

impl TryFrom<WalletAddressInfoWire> for WalletAddressInfo {
    type Error = WireError;

    fn try_from(value: WalletAddressInfoWire) -> Result<Self, Self::Error> {
        Ok(Self {
            swig_config_address: value
                .swig_config_address
                .parse()
                .map_err(|_| WireError::Invalid("invalid Swig config address"))?,
            wallet_address: value
                .wallet_address
                .parse()
                .map_err(|_| WireError::Invalid("invalid wallet address"))?,
        })
    }
}

impl From<WalletAddressInfo> for WalletAddressInfoWire {
    fn from(value: WalletAddressInfo) -> Self {
        Self {
            swig_config_address: value.swig_config_address.to_string(),
            wallet_address: value.wallet_address.to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedTransactionWire {
    #[serde(alias = "unsignedTransaction", alias = "unsigned_transaction")]
    pub transaction: String,
    #[serde(alias = "transaction_encoding")]
    pub transaction_encoding: Option<TransactionEncodingWire>,
    pub wallet: Option<WalletAddressInfoWire>,
    #[serde(alias = "expires_at")]
    pub expires_at: Option<String>,
    pub network: Option<NetworkWire>,
    #[serde(alias = "recent_blockhash")]
    pub recent_blockhash: Option<String>,
    #[serde(default, skip_serializing)]
    pub kind: OptionalPreparedKind,
    #[serde(default, alias = "signature_requests")]
    pub signature_requests: Vec<ClientSignatureRequestWire>,
    #[serde(alias = "participant_set_approval_plan")]
    pub participant_set_approval_plan: Option<ParticipantSetApprovalPlanWire>,
}

pub(crate) fn validate_prepared_transaction_shape(value: &Value) -> Result<(), WireError> {
    let fields = value
        .as_object()
        .ok_or(WireError::Invalid("malformed prepared transaction"))?;
    if fields
        .get("wallet")
        .is_some_and(|wallet| !wallet.is_null() && !wallet.is_object())
    {
        return Err(WireError::Invalid("malformed prepared transaction"));
    }
    for field in ["signatureRequests", "signature_requests"] {
        if fields
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|requests| requests.iter().any(|request| !request.is_object()))
        {
            return Err(WireError::Invalid("malformed prepared transaction"));
        }
    }
    for field in [
        "participantSetApprovalPlan",
        "participant_set_approval_plan",
    ] {
        let Some(plan) = fields.get(field).filter(|plan| !plan.is_null()) else {
            continue;
        };
        let plan = plan
            .as_object()
            .ok_or(WireError::Invalid("malformed prepared transaction"))?;
        if let Some(members) = plan.get("members").and_then(Value::as_array) {
            for member in members {
                let member = member
                    .as_object()
                    .ok_or(WireError::Invalid("malformed prepared transaction"))?;
                if let Some(authority) = member.get("authority") {
                    validate_authority_shape(authority)?;
                }
            }
        }
    }
    Ok(())
}

impl TryFrom<PreparedTransactionWire> for PreparedTransaction {
    type Error = WireError;

    fn try_from(value: PreparedTransactionWire) -> Result<Self, Self::Error> {
        // Decode accepts only the validated Base64 marker, or the peer default.
        let transaction = decode_transaction(&value.transaction)?;
        // Endpoints inspect kind before conversion. It is not a public model field.
        let _kind = value.kind.0;
        Ok(Self {
            transaction,
            wallet: value.wallet.map(TryInto::try_into).transpose()?,
            expires_at: value.expires_at,
            network: value.network.map(TryInto::try_into).transpose()?,
            recent_blockhash: value
                .recent_blockhash
                .map(|blockhash| {
                    blockhash
                        .parse()
                        .map_err(|_| WireError::Invalid("invalid recent blockhash"))
                })
                .transpose()?,
            signature_requests: value
                .signature_requests
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            participant_set_approval_plan: value
                .participant_set_approval_plan
                .map(TryInto::try_into)
                .transpose()?,
        })
    }
}

impl TryFrom<PreparedTransaction> for PreparedTransactionWire {
    type Error = WireError;

    fn try_from(value: PreparedTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: encode_transaction(&value.transaction)?,
            transaction_encoding: Some(TransactionEncodingWire::Base64),
            wallet: value.wallet.map(Into::into),
            expires_at: value.expires_at,
            network: value.network.map(Into::into),
            recent_blockhash: value
                .recent_blockhash
                .map(|blockhash| blockhash.to_string()),
            kind: OptionalPreparedKind::default(),
            signature_requests: value
                .signature_requests
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            participant_set_approval_plan: value
                .participant_set_approval_plan
                .map(TryInto::try_into)
                .transpose()?,
        })
    }
}
