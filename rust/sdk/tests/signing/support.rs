use crate::{ClientSignatureRequest, Network, PreparedTransaction, WalletAuthority};
use k256::ecdsa::SigningKey as Secp256k1Key;
use p256::ecdsa::SigningKey as Secp256r1Key;
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use solana_hash::Hash;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{v0, Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::error::Error;
const SWIG: Pubkey = Pubkey::from_str_const("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");
const SYSTEM: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");
const INSTRUCTIONS: Pubkey = Pubkey::from_str_const("Sysvar1nstructions1111111111111111111111111");
pub(super) type TestResult = Result<(), Box<dyn Error>>;
// Mirrors SignV2Instruction::{new_secp256k1,new_secp256r1} and the compact
// transfer/account payloads from swig-wallet/interface. The precompile comes
// from Solana's production builder, not a test-specific encoding.
pub(super) fn preparation(
    authority: WalletAuthority,
    versioned: bool,
    count: u32,
) -> Result<PreparedTransaction, Box<dyn Error>> {
    let mut instructions = Vec::new();
    let mut requests = Vec::new();
    for counter in 1..=count {
        let mut accounts = vec![
            AccountMeta::new(Pubkey::new_from_array([1; 32]), false),
            AccountMeta::new(Pubkey::new_from_array([2; 32]), false),
            AccountMeta::new_readonly(SYSTEM, false),
        ];
        let program_index = if matches!(authority, WalletAuthority::Secp256r1 { .. }) {
            accounts.push(AccountMeta::new_readonly(INSTRUCTIONS, false));
            4_u8
        } else {
            3_u8
        };
        accounts.push(AccountMeta::new_readonly(SYSTEM, false));
        accounts.push(AccountMeta::new(Pubkey::new_from_array([3; 32]), false));

        let mut compact = vec![1, program_index, 2, 1, program_index + 1, 12, 0];
        compact.extend_from_slice(&2_u32.to_le_bytes());
        compact.extend_from_slice(&1_000_u64.to_le_bytes());

        let mut digest_payload = compact.clone();
        for account in &accounts {
            digest_payload.extend_from_slice(account.pubkey.as_ref());
            digest_payload
                .extend_from_slice(&[u8::from(account.is_writable), u8::from(account.is_signer)]);
            digest_payload.extend_from_slice(&[0; 6]);
        }
        let slot = 800_u64 + u64::from(counter);
        digest_payload.extend_from_slice(&slot.to_le_bytes());
        digest_payload.extend_from_slice(&counter.to_le_bytes());
        let message_hash: [u8; 32] = match authority {
            WalletAuthority::Secp256k1 { .. } => Sha256::digest(&digest_payload).into(),
            WalletAuthority::Secp256r1 { .. } => Keccak256::digest(&digest_payload).into(),
            _ => return Err("fixture requires a secp256 authority".into()),
        };

        let mut data = 11_u16.to_le_bytes().to_vec();
        data.extend_from_slice(&u16::try_from(compact.len())?.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&compact);
        data.extend_from_slice(&slot.to_le_bytes());
        data.extend_from_slice(&counter.to_le_bytes());
        match authority {
            WalletAuthority::Secp256k1 { .. } => data.extend_from_slice(&[0; 65]),
            WalletAuthority::Secp256r1 { public_key } => {
                instructions.push(
                    solana_secp256r1_program::new_secp256r1_instruction_with_signature(
                        &message_hash,
                        &[0; 64],
                        &public_key,
                    ),
                );
                data.extend_from_slice(&[3, 0, 0, 0, 0]);
            }
            _ => return Err("fixture requires a secp256 authority".into()),
        }
        instructions.push(Instruction {
            program_id: SWIG,
            accounts,
            data,
        });
        requests.push(ClientSignatureRequest {
            authority: authority.clone(),
            message_hash,
            slot,
            counter,
        });
    }
    let fee_payer = Keypair::new_from_array([9; 32]);
    let blockhash = Hash::new_from_array([8; 32]);
    let message = if versioned {
        VersionedMessage::V0(v0::Message::try_compile(
            &fee_payer.pubkey(),
            &instructions,
            &[],
            blockhash,
        )?)
    } else {
        VersionedMessage::Legacy(Message::new_with_blockhash(
            &instructions,
            Some(&fee_payer.pubkey()),
            &blockhash,
        ))
    };
    let transaction = VersionedTransaction {
        signatures: vec![Signature::default()],
        message,
    };
    transaction.sanitize()?;
    Ok(PreparedTransaction {
        transaction,
        wallet: None,
        expires_at: None,
        network: Some(Network::Devnet),
        recent_blockhash: Some(blockhash),
        signature_requests: requests,
        participant_set_approval_plan: None,
    })
}

pub(super) fn secp256r1_authority(key: &Secp256r1Key) -> Result<WalletAuthority, Box<dyn Error>> {
    Ok(WalletAuthority::Secp256r1 {
        public_key: key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .try_into()?,
    })
}
pub(super) fn secp256k1_authority(key: &Secp256k1Key) -> Result<WalletAuthority, Box<dyn Error>> {
    Ok(WalletAuthority::Secp256k1 {
        public_key: key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .try_into()?,
    })
}
