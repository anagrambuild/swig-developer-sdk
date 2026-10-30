use solana_pubkey::Pubkey;

use crate::api::{
    error::ApiConversionError, types::authority::WalletAuthority as ApiWalletAuthority,
};

/// A wallet authority expressed using native keys.
///
/// Secp256 keys contain compressed SEC1 bytes. API conversion validates that
/// each key is a point on its declared curve before encoding or accepting it.
#[derive(Clone, Eq, PartialEq)]
pub enum WalletAuthority {
    Ed25519 {
        public_key: Pubkey,
    },
    Secp256k1 {
        public_key: [u8; 33],
    },
    Secp256r1 {
        public_key: [u8; 33],
    },
    ParticipantSet {
        address: Pubkey,
        role_id: Option<u32>,
    },
}

impl TryFrom<ApiWalletAuthority> for WalletAuthority {
    type Error = ApiConversionError;

    fn try_from(value: ApiWalletAuthority) -> Result<Self, Self::Error> {
        match value {
            ApiWalletAuthority::Ed25519 { public_key } => Ok(Self::Ed25519 { public_key }),
            ApiWalletAuthority::Secp256k1 { public_key } => {
                let public_key: [u8; 33] =
                    hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key))
                        .map_err(|_| ApiConversionError::Invalid("invalid secp256k1 public key"))?
                        .try_into()
                        .map_err(|_| ApiConversionError::Invalid("invalid secp256k1 public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(ApiConversionError::Invalid("invalid secp256k1 public key"));
                }
                k256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| ApiConversionError::Invalid("invalid secp256k1 public key"))?;
                Ok(Self::Secp256k1 { public_key })
            }
            ApiWalletAuthority::Secp256r1 { public_key } => {
                let public_key: [u8; 33] =
                    hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key))
                        .map_err(|_| ApiConversionError::Invalid("invalid secp256r1 public key"))?
                        .try_into()
                        .map_err(|_| ApiConversionError::Invalid("invalid secp256r1 public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(ApiConversionError::Invalid("invalid secp256r1 public key"));
                }
                p256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| ApiConversionError::Invalid("invalid secp256r1 public key"))?;
                Ok(Self::Secp256r1 { public_key })
            }
            ApiWalletAuthority::ParticipantSet {
                participant_set_address,
                role_id,
            } => Ok(Self::ParticipantSet {
                address: participant_set_address,
                role_id,
            }),
            ApiWalletAuthority::ProgramExecProof { .. } => {
                Err(ApiConversionError::Invalid("unsupported wallet authority"))
            }
        }
    }
}

impl TryFrom<WalletAuthority> for ApiWalletAuthority {
    type Error = ApiConversionError;

    fn try_from(value: WalletAuthority) -> Result<Self, Self::Error> {
        match value {
            WalletAuthority::Ed25519 { public_key } => Ok(Self::Ed25519 { public_key }),
            WalletAuthority::Secp256k1 { public_key } => {
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(ApiConversionError::Invalid("invalid secp256k1 public key"));
                }
                k256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| ApiConversionError::Invalid("invalid secp256k1 public key"))?;
                Ok(Self::Secp256k1 {
                    public_key: hex::encode(public_key),
                })
            }
            WalletAuthority::Secp256r1 { public_key } => {
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(ApiConversionError::Invalid("invalid secp256r1 public key"));
                }
                p256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| ApiConversionError::Invalid("invalid secp256r1 public key"))?;
                Ok(Self::Secp256r1 {
                    public_key: hex::encode(public_key),
                })
            }
            WalletAuthority::ParticipantSet { address, role_id } => Ok(Self::ParticipantSet {
                participant_set_address: address,
                role_id,
            }),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/api/authority_conversion.rs"]
mod tests;
