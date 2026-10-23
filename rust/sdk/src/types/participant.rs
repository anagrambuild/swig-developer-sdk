use solana_pubkey::Pubkey;

use crate::WalletAuthority;
#[derive(Clone)]
pub enum ParticipantApproval {
    Ed25519 {
        member_index: u32,
        signature: Vec<u8>,
    },
    Secp256k1 {
        member_index: u32,
        signature: Vec<u8>,
    },
    WebAuthnP256 {
        member_index: u32,
        authenticator_data: Vec<u8>,
        client_data_json: Vec<u8>,
        signature: Vec<u8>,
    },
}

#[derive(Clone)]
pub struct ParticipantApprovalRequest {
    pub member_index: u32,
    pub authority: WalletAuthority,
    pub challenge: [u8; 32],
}

#[derive(Clone)]
pub struct ParticipantSetApprovalPlan {
    pub participant_set_address: Pubkey,
    pub role_id: u32,
    pub expiration_slot: u64,
    pub nonce: u32,
    pub transaction_digest: [u8; 32],
    pub compilation_envelope: String,
    pub threshold: u32,
    pub members: Vec<ParticipantApprovalRequest>,
}
