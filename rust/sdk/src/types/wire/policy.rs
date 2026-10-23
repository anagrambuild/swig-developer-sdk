use k256::elliptic_curve::sec1::ToEncodedPoint;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    types::wire::{AuthorityWire, WireError},
    PolicyAuthority, WalletAuthority,
};

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum PolicyAuthorityWire {
    Template(PolicyAuthorityConfigWire),
    Wallet(AuthorityWire),
}

#[serde_as]
#[derive(Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum PolicyAuthorityConfigWire {
    Ed25519 {
        public_key: String,
    },
    Secp256k1 {
        public_key: String,
    },
    Secp256r1 {
        public_key: String,
    },
    Ed25519Session {
        public_key: String,
        #[serde_as(as = "DisplayFromStr")]
        max_duration_slots: u64,
    },
    Secp256k1Session {
        public_key: String,
        #[serde_as(as = "DisplayFromStr")]
        max_duration_slots: u64,
    },
    Secp256r1Session {
        public_key: String,
        #[serde_as(as = "DisplayFromStr")]
        max_duration_slots: u64,
    },
}

impl TryFrom<PolicyAuthorityWire> for PolicyAuthority {
    type Error = WireError;

    fn try_from(value: PolicyAuthorityWire) -> Result<Self, Self::Error> {
        match value {
            PolicyAuthorityWire::Wallet(authority) => {
                Ok(Self::Token(WalletAuthority::try_from(authority)?))
            }
            PolicyAuthorityWire::Template(authority) => match authority {
                PolicyAuthorityConfigWire::Ed25519 { public_key } => Ok(Self::Token(
                    WalletAuthority::try_from(AuthorityWire::Ed25519 { public_key })?,
                )),
                PolicyAuthorityConfigWire::Secp256k1 { public_key } => {
                    Ok(Self::Token(WalletAuthority::Secp256k1 {
                        public_key: decode_policy_secp256k1(public_key)?,
                    }))
                }
                PolicyAuthorityConfigWire::Secp256r1 { public_key } => Ok(Self::Token(
                    WalletAuthority::try_from(AuthorityWire::Secp256r1 { public_key })?,
                )),
                PolicyAuthorityConfigWire::Ed25519Session {
                    public_key,
                    max_duration_slots,
                } => Ok(Self::Session {
                    authority: WalletAuthority::try_from(AuthorityWire::Ed25519 { public_key })?,
                    max_duration_slots,
                }),
                PolicyAuthorityConfigWire::Secp256k1Session {
                    public_key,
                    max_duration_slots,
                } => Ok(Self::Session {
                    authority: WalletAuthority::Secp256k1 {
                        public_key: decode_policy_secp256k1(public_key)?,
                    },
                    max_duration_slots,
                }),
                PolicyAuthorityConfigWire::Secp256r1Session {
                    public_key,
                    max_duration_slots,
                } => Ok(Self::Session {
                    authority: WalletAuthority::try_from(AuthorityWire::Secp256r1 { public_key })?,
                    max_duration_slots,
                }),
            },
        }
    }
}

fn decode_policy_secp256k1(public_key: String) -> Result<[u8; 33], WireError> {
    let bytes = hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key))
        .map_err(|_| WireError::Invalid("invalid policy secp256k1 public key"))?;
    // Legacy template sessions store the uncompressed coordinates without the
    // SEC1 prefix. Native authorities always retain the compressed curve point.
    let bytes = if bytes.len() == 64 {
        let mut encoded = Vec::with_capacity(65);
        encoded.push(4);
        encoded.extend_from_slice(&bytes);
        encoded
    } else {
        bytes
    };
    k256::PublicKey::from_sec1_bytes(&bytes)
        .map_err(|_| WireError::Invalid("invalid policy secp256k1 public key"))?
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| WireError::Invalid("invalid policy secp256k1 public key"))
}
