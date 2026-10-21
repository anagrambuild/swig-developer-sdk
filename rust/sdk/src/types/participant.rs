use solana_pubkey::Pubkey;

use crate::WalletAuthority;

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
