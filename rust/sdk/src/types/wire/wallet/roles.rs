use crate::types::wire::WireError;
use crate::SwigRoleAuthority;
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleAuthorityWire {
    #[serde(alias = "authority_type")]
    authority_type: i32,
    #[serde(alias = "authority_value")]
    authority_value: String,
}

impl TryFrom<RoleAuthorityWire> for SwigRoleAuthority {
    type Error = WireError;

    fn try_from(wire: RoleAuthorityWire) -> Result<Self, Self::Error> {
        let bytes = bs58::decode(wire.authority_value)
            .into_vec()
            .map_err(|_| WireError::Invalid("Invalid role authority encoding"))?;
        match wire.authority_type {
            1 | 2 => {
                let key: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| WireError::Invalid("Invalid Ed25519 role public key"))?;
                let public_key = Pubkey::new_from_array(key);
                Ok(if wire.authority_type == 1 {
                    Self::Ed25519 { public_key }
                } else {
                    Self::Ed25519Session { public_key }
                })
            }
            3 | 4 => {
                let public_key: [u8; 33] = bytes
                    .try_into()
                    .map_err(|_| WireError::Invalid("Invalid secp256k1 role public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("Invalid secp256k1 role public key"));
                }
                k256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("Invalid secp256k1 role public key"))?;
                Ok(if wire.authority_type == 3 {
                    Self::Secp256k1 { public_key }
                } else {
                    Self::Secp256k1Session { public_key }
                })
            }
            5 | 6 => {
                let public_key: [u8; 33] = bytes
                    .try_into()
                    .map_err(|_| WireError::Invalid("Invalid secp256r1 role public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("Invalid secp256r1 role public key"));
                }
                p256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("Invalid secp256r1 role public key"))?;
                Ok(if wire.authority_type == 5 {
                    Self::Secp256r1 { public_key }
                } else {
                    Self::Secp256r1Session { public_key }
                })
            }
            7 | 8 => {
                const MAX_INSTRUCTION_PREFIX_LEN: usize = 40;
                if bytes.len() > MAX_INSTRUCTION_PREFIX_LEN {
                    return Err(WireError::Invalid(
                        "ProgramExec instruction prefix exceeds 40 bytes",
                    ));
                }
                Ok(if wire.authority_type == 7 {
                    Self::ProgramExec {
                        instruction_prefix: bytes,
                    }
                } else {
                    Self::ProgramExecSession {
                        instruction_prefix: bytes,
                    }
                })
            }
            _ => Err(WireError::Invalid("Unknown role authority type")),
        }
    }
}

impl From<SwigRoleAuthority> for RoleAuthorityWire {
    fn from(authority: SwigRoleAuthority) -> Self {
        let (authority_type, bytes) = match authority {
            SwigRoleAuthority::Ed25519 { public_key } => (1, public_key.to_bytes().to_vec()),
            SwigRoleAuthority::Ed25519Session { public_key } => (2, public_key.to_bytes().to_vec()),
            SwigRoleAuthority::Secp256k1 { public_key } => (3, public_key.to_vec()),
            SwigRoleAuthority::Secp256k1Session { public_key } => (4, public_key.to_vec()),
            SwigRoleAuthority::Secp256r1 { public_key } => (5, public_key.to_vec()),
            SwigRoleAuthority::Secp256r1Session { public_key } => (6, public_key.to_vec()),
            SwigRoleAuthority::ProgramExec { instruction_prefix } => (7, instruction_prefix),
            SwigRoleAuthority::ProgramExecSession { instruction_prefix } => (8, instruction_prefix),
        };
        Self {
            authority_type,
            authority_value: bs58::encode(bytes).into_string(),
        }
    }
}
