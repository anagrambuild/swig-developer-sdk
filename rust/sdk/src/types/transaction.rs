pub use solana_hash::Hash as RecentBlockhash;
use solana_transaction::versioned::VersionedTransaction;

use crate::{Network, ParticipantSetApprovalPlan, WalletAddressInfo, WalletAuthority};

#[derive(Clone)]
pub struct ClientSignatureRequest {
    pub authority: WalletAuthority,
    pub message_hash: [u8; 32],
    pub slot: u64,
    pub counter: u32,
}

use crate::api::{
    error::ApiConversionError,
    types::{
        authority::WalletAuthority as ApiWalletAuthority, encoding::SignatureScheme,
        transaction::ClientSignatureRequest as ApiClientSignatureRequest,
    },
};
impl TryFrom<ApiClientSignatureRequest> for ClientSignatureRequest {
    type Error = ApiConversionError;

    fn try_from(value: ApiClientSignatureRequest) -> Result<Self, Self::Error> {
        let authority = match value.scheme {
            SignatureScheme::Secp256k1 => ApiWalletAuthority::Secp256k1 {
                public_key: value.signer,
            },
            SignatureScheme::Secp256r1 => ApiWalletAuthority::Secp256r1 {
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

impl TryFrom<ClientSignatureRequest> for ApiClientSignatureRequest {
    type Error = ApiConversionError;

    fn try_from(value: ClientSignatureRequest) -> Result<Self, Self::Error> {
        let (scheme, signer) = match ApiWalletAuthority::try_from(value.authority)? {
            ApiWalletAuthority::Secp256k1 { public_key } => {
                (SignatureScheme::Secp256k1, public_key)
            }
            ApiWalletAuthority::Secp256r1 { public_key } => {
                (SignatureScheme::Secp256r1, public_key)
            }
            _ => {
                return Err(ApiConversionError::Invalid(
                    "unsupported client signature authority",
                ))
            }
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

pub(crate) fn decode_hex32(value: &str) -> Result<[u8; 32], ApiConversionError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    let mut bytes = [0; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|_| ApiConversionError::Invalid("invalid 32-byte hex value"))?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "../../tests/api/transaction.rs"]
pub(crate) mod tests;
#[derive(Clone)]
pub struct PreparedTransaction {
    pub transaction: VersionedTransaction,
    pub wallet: Option<WalletAddressInfo>,
    pub expires_at: Option<String>,
    pub network: Option<Network>,
    pub recent_blockhash: Option<RecentBlockhash>,
    pub signature_requests: Vec<ClientSignatureRequest>,
    pub participant_set_approval_plan: Option<ParticipantSetApprovalPlan>,
}

use crate::api::types::{
    encoding::TransactionEncoding, transaction::PreparedTransaction as ApiPreparedTransaction,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use bincode::Options;
pub(crate) fn decode_transaction(value: &str) -> Result<VersionedTransaction, ApiConversionError> {
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| ApiConversionError::Invalid("invalid Base64 transaction"))?;
    if STANDARD.encode(&bytes) != value {
        return Err(ApiConversionError::Invalid(
            "noncanonical Base64 transaction",
        ));
    }
    let byte_limit = u64::try_from(bytes.len())
        .map_err(|_| ApiConversionError::Invalid("transaction exceeds decoder limit"))?;
    let transaction: VersionedTransaction = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(byte_limit)
        .reject_trailing_bytes()
        .deserialize(&bytes)
        .map_err(|_| ApiConversionError::Invalid("invalid Solana transaction"))?;
    transaction
        .sanitize()
        .map_err(|_| ApiConversionError::Invalid("invalid Solana transaction structure"))?;
    Ok(transaction)
}

pub(crate) fn encode_transaction(
    transaction: &VersionedTransaction,
) -> Result<String, ApiConversionError> {
    transaction
        .sanitize()
        .map_err(|_| ApiConversionError::Invalid("invalid Solana transaction structure"))?;
    let bytes = bincode::serialize(transaction)
        .map_err(|_| ApiConversionError::Invalid("transaction serialization failed"))?;
    Ok(STANDARD.encode(bytes))
}

impl TryFrom<ApiPreparedTransaction> for PreparedTransaction {
    type Error = ApiConversionError;

    fn try_from(value: ApiPreparedTransaction) -> Result<Self, Self::Error> {
        // Decode accepts only the validated Base64 marker, or the peer default.
        let transaction = decode_transaction(&value.transaction)?;
        // Endpoints inspect kind before conversion. It is not a public model field.
        let _kind = value.kind;
        Ok(Self {
            transaction,
            wallet: value.wallet.map(TryInto::try_into).transpose()?,
            expires_at: value.expires_at,
            network: value.network.map(TryInto::try_into).transpose()?,
            recent_blockhash: value.recent_blockhash,
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

impl TryFrom<PreparedTransaction> for ApiPreparedTransaction {
    type Error = ApiConversionError;

    fn try_from(value: PreparedTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: encode_transaction(&value.transaction)?,
            transaction_encoding: Some(TransactionEncoding::Base64),
            wallet: value.wallet.map(Into::into),
            expires_at: value.expires_at,
            network: value.network.map(Into::into),
            recent_blockhash: value.recent_blockhash,
            kind: None,
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
