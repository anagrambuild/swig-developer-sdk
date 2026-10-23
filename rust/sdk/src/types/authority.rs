use solana_pubkey::Pubkey;

/// A wallet authority expressed using native keys.
///
/// Secp256 keys contain compressed SEC1 bytes. Wire conversion validates that
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
