use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    try_from = "crate::types::wire::wallet::roles::RoleAuthorityWire",
    into = "crate::types::wire::wallet::roles::RoleAuthorityWire"
)]
pub enum SwigRoleAuthority {
    Ed25519 { public_key: Pubkey },
    Ed25519Session { public_key: Pubkey },
    Secp256k1 { public_key: [u8; 33] },
    Secp256k1Session { public_key: [u8; 33] },
    Secp256r1 { public_key: [u8; 33] },
    Secp256r1Session { public_key: [u8; 33] },
    ProgramExec { instruction_prefix: Vec<u8> },
    ProgramExecSession { instruction_prefix: Vec<u8> },
}
