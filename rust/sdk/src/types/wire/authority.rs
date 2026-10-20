use serde::{Deserialize, Serialize};

use crate::{types::wire::WireError, WalletAuthority};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum AuthorityWire {
    Ed25519 {
        public_key: String,
    },
    Secp256k1 {
        public_key: String,
    },
    Secp256r1 {
        public_key: String,
    },
    ParticipantSet {
        #[serde(rename = "participantSetAddress", alias = "participant_set_address")]
        address: String,
        #[serde(skip_serializing_if = "Option::is_none", alias = "role_id")]
        role_id: Option<u32>,
    },
}

impl TryFrom<AuthorityWire> for WalletAuthority {
    type Error = WireError;

    fn try_from(value: AuthorityWire) -> Result<Self, Self::Error> {
        match value {
            AuthorityWire::Ed25519 { public_key } => Ok(Self::Ed25519 {
                public_key: public_key
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid Ed25519 public key"))?,
            }),
            AuthorityWire::Secp256k1 { public_key } => {
                let public_key: [u8; 33] =
                    hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key))
                        .map_err(|_| WireError::Invalid("invalid secp256k1 public key"))?
                        .try_into()
                        .map_err(|_| WireError::Invalid("invalid secp256k1 public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("invalid secp256k1 public key"));
                }
                k256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("invalid secp256k1 public key"))?;
                Ok(Self::Secp256k1 { public_key })
            }
            AuthorityWire::Secp256r1 { public_key } => {
                let public_key: [u8; 33] =
                    hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key))
                        .map_err(|_| WireError::Invalid("invalid secp256r1 public key"))?
                        .try_into()
                        .map_err(|_| WireError::Invalid("invalid secp256r1 public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("invalid secp256r1 public key"));
                }
                p256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("invalid secp256r1 public key"))?;
                Ok(Self::Secp256r1 { public_key })
            }
            AuthorityWire::ParticipantSet { address, role_id } => Ok(Self::ParticipantSet {
                address: address
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid participant set address"))?,
                role_id,
            }),
        }
    }
}

impl TryFrom<WalletAuthority> for AuthorityWire {
    type Error = WireError;

    fn try_from(value: WalletAuthority) -> Result<Self, Self::Error> {
        match value {
            WalletAuthority::Ed25519 { public_key } => Ok(Self::Ed25519 {
                public_key: public_key.to_string(),
            }),
            WalletAuthority::Secp256k1 { public_key } => {
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("invalid secp256k1 public key"));
                }
                k256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("invalid secp256k1 public key"))?;
                Ok(Self::Secp256k1 {
                    public_key: hex::encode(public_key),
                })
            }
            WalletAuthority::Secp256r1 { public_key } => {
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("invalid secp256r1 public key"));
                }
                p256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("invalid secp256r1 public key"))?;
                Ok(Self::Secp256r1 {
                    public_key: hex::encode(public_key),
                })
            }
            WalletAuthority::ParticipantSet { address, role_id } => Ok(Self::ParticipantSet {
                address: address.to_string(),
                role_id,
            }),
        }
    }
}
