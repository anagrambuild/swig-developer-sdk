use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum WalletAuthority {
    Ed25519 {
        #[serde(alias = "public_key")]
        #[serde_as(as = "DisplayFromStr")]
        public_key: Pubkey,
    },
    Secp256k1 {
        #[serde(alias = "public_key")]
        public_key: String,
    },
    Secp256r1 {
        #[serde(alias = "public_key")]
        public_key: String,
    },
    #[serde(alias = "program_exec_proof")]
    ProgramExecProof {
        #[serde(default, alias = "role_id")]
        role_id: u32,
        #[serde(default, alias = "zk_proof")]
        zk_proof: String,
    },
    #[serde(alias = "participant_set")]
    ParticipantSet {
        #[serde(alias = "participant_set_address")]
        #[serde_as(as = "DisplayFromStr")]
        participant_set_address: Pubkey,
        #[serde(default, alias = "role_id", skip_serializing_if = "Option::is_none")]
        role_id: Option<u32>,
    },
}

#[cfg(test)]
#[path = "../../../tests/api/authority.rs"]
mod tests;
