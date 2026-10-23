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
