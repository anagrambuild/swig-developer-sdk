use super::support::{preparation, secp256k1_authority, TestResult};
use crate::{Secp256k1TransactionSigner, TransactionSignError, TransactionSigner};
use k256::ecdsa::{
    signature::DigestVerifier, RecoveryId, Signature as Secp256k1Signature,
    SigningKey as Secp256k1Key, VerifyingKey as Secp256k1VerifyingKey,
};
use sha3::{Digest, Keccak256};
use solana_keypair::Keypair;
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;
use std::sync::Arc;
#[tokio::test]
async fn secp256k1_signs_hex_hash_without_personal_prefix_and_preserves_other_bytes() -> TestResult
{
    let key = Arc::new(Secp256k1Key::from_slice(&[3; 32])?);
    let signer = Secp256k1TransactionSigner::new(key.clone());
    for versioned in [false, true] {
        let prepared = preparation(secp256k1_authority(&key)?, versioned, 1)?;
        let before = prepared.transaction.clone();
        let request = prepared
            .signature_requests
            .first()
            .ok_or("missing request")?
            .clone();
        let signed = signer.sign_prepared_transaction(prepared).await?;
        let data = &signed
            .message
            .instructions()
            .first()
            .ok_or("missing instruction")?
            .data;
        let start = data.len().checked_sub(65).ok_or("truncated signature")?;
        let signature_bytes = data.get(start..start + 64).ok_or("missing signature")?;
        let signature = Secp256k1Signature::from_slice(signature_bytes)?;
        assert!(signature.normalize_s().is_none());
        let recovery_byte = *data.last().ok_or("missing recovery byte")?;
        assert!(matches!(recovery_byte, 27 | 28));
        let recovery_id = RecoveryId::try_from(recovery_byte - 27)?;
        let text = hex::encode(request.message_hash);
        let recovered = Secp256k1VerifyingKey::recover_from_digest(
            Keccak256::new_with_prefix(text.as_bytes()),
            &signature,
            recovery_id,
        )?;
        assert_eq!(recovered, *key.verifying_key());
        assert!(key
            .verifying_key()
            .verify_digest(Keccak256::new_with_prefix(request.message_hash), &signature)
            .is_err());
        let mut personal = b"\x19Ethereum Signed Message:\n64".to_vec();
        personal.extend_from_slice(text.as_bytes());
        assert!(key
            .verifying_key()
            .verify_digest(Keccak256::new_with_prefix(personal), &signature)
            .is_err());
        let mut restored = signed.clone();
        match &mut restored.message {
            VersionedMessage::Legacy(message) => message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(start..)
                .ok_or("missing signature")?
                .fill(0),
            VersionedMessage::V0(message) => message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(start..)
                .ok_or("missing signature")?
                .fill(0),
        }
        assert_eq!(restored, before);
        let outer =
            VersionedTransaction::try_new(signed.message, &[&Keypair::new_from_array([9; 32])])?;
        assert!(outer.verify_and_hash_message().is_ok());
    }
    assert_eq!(Arc::strong_count(&key), 2);
    Ok(())
}

#[tokio::test]
async fn secp256k1_requires_exact_replay_metadata_and_swig_program() -> TestResult {
    let signer = Secp256k1TransactionSigner::from_bytes(&[3; 32])?;
    let prepared = preparation(
        secp256k1_authority(&Secp256k1Key::from_slice(&[3; 32])?)?,
        false,
        1,
    )?;
    for slot_changed in [false, true] {
        let mut input = prepared.clone();
        let request = input
            .signature_requests
            .first_mut()
            .ok_or("missing request")?;
        if slot_changed {
            request.slot += 1;
        } else {
            request.counter += 1;
        }
        assert!(matches!(
            signer.sign_prepared_transaction(input).await,
            Err(TransactionSignError::SigningTargetNotFound)
        ));
    }
    let mut input = prepared;
    if let VersionedMessage::Legacy(message) = &mut input.transaction.message {
        let program_index = message
            .instructions
            .first()
            .ok_or("missing instruction")?
            .program_id_index;
        *message
            .account_keys
            .get_mut(usize::from(program_index))
            .ok_or("missing program")? = Pubkey::new_from_array([8; 32]);
    }
    assert!(matches!(
        signer.sign_prepared_transaction(input).await,
        Err(TransactionSignError::SigningTargetNotFound)
    ));
    Ok(())
}

#[tokio::test]
async fn secp256k1_rejects_truncated_headers_payloads_offsets_and_existing_prefixes() -> TestResult
{
    let signer = Secp256k1TransactionSigner::from_bytes(&[3; 32])?;
    let prepared = preparation(
        secp256k1_authority(&Secp256k1Key::from_slice(&[3; 32])?)?,
        false,
        1,
    )?;
    let mut short_header = prepared.clone();
    if let VersionedMessage::Legacy(message) = &mut short_header.transaction.message {
        message
            .instructions
            .first_mut()
            .ok_or("missing instruction")?
            .data
            .truncate(7);
    }
    assert!(matches!(
        signer.sign_prepared_transaction(short_header).await,
        Err(TransactionSignError::InvalidPreparation(
            "truncated Swig SignV2 header"
        ))
    ));

    let mut short_payload = prepared.clone();
    if let VersionedMessage::Legacy(message) = &mut short_payload.transaction.message {
        message
            .instructions
            .first_mut()
            .ok_or("missing instruction")?
            .data
            .pop();
    }
    assert!(matches!(
        signer.sign_prepared_transaction(short_payload).await,
        Err(TransactionSignError::SigningTargetNotFound)
    ));

    let mut invalid_offset = prepared.clone();
    if let VersionedMessage::Legacy(message) = &mut invalid_offset.transaction.message {
        message
            .instructions
            .first_mut()
            .ok_or("missing instruction")?
            .data
            .get_mut(2..4)
            .ok_or("missing offset")?
            .copy_from_slice(&u16::MAX.to_le_bytes());
    }
    assert!(matches!(
        signer.sign_prepared_transaction(invalid_offset).await,
        Err(TransactionSignError::SigningTargetNotFound)
    ));

    let mut prefixed = prepared;
    if let VersionedMessage::Legacy(message) = &mut prefixed.transaction.message {
        message
            .instructions
            .first_mut()
            .ok_or("missing instruction")?
            .data
            .extend_from_slice(b"\x19Ethereum Signed Message:\n64");
    }
    assert!(matches!(
        signer.sign_prepared_transaction(prefixed).await,
        Err(TransactionSignError::InvalidPreparation(
            "raw secp256k1 signing requires an unprefixed authority payload"
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn all_requests_for_one_key_are_signed_and_keep_order() -> TestResult {
    let key = Secp256k1Key::from_slice(&[3; 32])?;
    let signer = Secp256k1TransactionSigner::from_bytes(&[3; 32])?;
    let prepared = preparation(secp256k1_authority(&key)?, true, 2)?;
    let requests = prepared.signature_requests.clone();
    let signed = signer.sign_prepared_transaction(prepared).await?;
    for (index, request) in requests.iter().enumerate() {
        let data = &signed
            .message
            .instructions()
            .get(index)
            .ok_or("missing instruction")?
            .data;
        let start = data.len().checked_sub(65).ok_or("truncated signature")?;
        let signature = Secp256k1Signature::from_slice(
            data.get(start..start + 64).ok_or("missing signature")?,
        )?;
        key.verifying_key().verify_digest(
            Keccak256::new_with_prefix(hex::encode(request.message_hash)),
            &signature,
        )?;
    }
    Ok(())
}
