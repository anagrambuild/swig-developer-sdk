use crate::{types::transaction::tests::native_transaction, WalletAuthority};
use base64::Engine;
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;

pub(crate) fn key(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}

pub(crate) fn authority() -> WalletAuthority {
    WalletAuthority::Ed25519 { public_key: key(1) }
}

pub(crate) fn prepared() -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({"transaction":base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&native_transaction(false))?),"kind":0,"network":"NETWORK_DEVNET","wallet":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string()}}),
    )
}
