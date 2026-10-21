use crate::WalletAuthority;

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
mod tests;
