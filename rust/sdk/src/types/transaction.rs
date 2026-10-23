use crate::WalletAuthority;

#[derive(Clone)]
pub struct ClientSignatureRequest {
    pub authority: WalletAuthority,
    pub message_hash: [u8; 32],
    pub slot: u64,
    pub counter: u32,
}
