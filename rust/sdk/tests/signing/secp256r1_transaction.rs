use super::support::{preparation, secp256r1_authority, TestResult};
use crate::{Secp256r1TransactionSigner, TransactionSignError, TransactionSigner};
use p256::ecdsa::{
    signature::Verifier, Signature as Secp256r1Signature, SigningKey as Secp256r1Key,
};
use solana_keypair::Keypair;
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;
use std::sync::Arc;
#[tokio::test]
async fn secp256r1_signs_hash_with_sha256_and_preserves_other_bytes() -> TestResult {
    let key = Arc::new(Secp256r1Key::from_slice(&[7; 32])?);
    let signer = Secp256r1TransactionSigner::new(key.clone());
    for versioned in [false, true] {
        let prepared = preparation(secp256r1_authority(&key)?, versioned, 1)?;
        let before = prepared.transaction.clone();
        let hash = prepared
            .signature_requests
            .first()
            .ok_or("missing request")?
            .message_hash;
        let signed = signer.sign_prepared_transaction(prepared).await?;
        let data = &signed
            .message
            .instructions()
            .first()
            .ok_or("missing precompile")?
            .data;
        let signature =
            Secp256r1Signature::from_slice(data.get(49..113).ok_or("missing signature")?)?;
        assert!(signature.normalize_s().is_none());
        key.verifying_key().verify(&hash, &signature)?;
        assert!(key
            .verifying_key()
            .verify(hex::encode(hash).as_bytes(), &signature)
            .is_err());
        let mut restored = signed.clone();
        match &mut restored.message {
            VersionedMessage::Legacy(message) => message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(49..113)
                .ok_or("missing signature")?
                .fill(0),
            VersionedMessage::V0(message) => message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(49..113)
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
async fn secp256r1_rejects_wrong_precompile_key_message_program_and_offsets() -> TestResult {
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let prepared = preparation(
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
        false,
        1,
    )?;
    for offset in [16, 113] {
        let mut input = prepared.clone();
        if let VersionedMessage::Legacy(message) = &mut input.transaction.message {
            let byte = message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(offset)
                .ok_or("missing byte")?;
            *byte ^= 1;
        }
        assert!(matches!(
            signer.sign_prepared_transaction(input).await,
            Err(TransactionSignError::SigningTargetNotFound)
        ));
    }
    let mut input = prepared.clone();
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
    for offset in [2, 4, 6, 8, 10, 12, 14] {
        let mut input = prepared.clone();
        if let VersionedMessage::Legacy(message) = &mut input.transaction.message {
            let byte = message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(offset)
                .ok_or("missing byte")?;
            *byte ^= 1;
        }
        assert!(matches!(
            signer.sign_prepared_transaction(input).await,
            Err(TransactionSignError::InvalidPreparation(
                "invalid secp256r1 precompile offsets"
            ))
        ));
    }
    Ok(())
}

#[tokio::test]
async fn secp256r1_requires_single_signature_header_and_preserves_trailing_data() -> TestResult {
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let prepared = preparation(
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
        false,
        1,
    )?;
    for offset in [0, 1] {
        let mut input = prepared.clone();
        if let VersionedMessage::Legacy(message) = &mut input.transaction.message {
            *message
                .instructions
                .first_mut()
                .ok_or("missing instruction")?
                .data
                .get_mut(offset)
                .ok_or("missing header")? = 2;
        }
        assert!(matches!(
            signer.sign_prepared_transaction(input).await,
            Err(TransactionSignError::InvalidPreparation(
                "invalid secp256r1 precompile offsets"
            ))
        ));
    }
    let mut input = prepared;
    if let VersionedMessage::Legacy(message) = &mut input.transaction.message {
        message
            .instructions
            .first_mut()
            .ok_or("missing instruction")?
            .data
            .extend_from_slice(&[5, 6, 7]);
    }
    let signed = signer.sign_prepared_transaction(input).await?;
    assert_eq!(
        signed
            .message
            .instructions()
            .first()
            .ok_or("missing instruction")?
            .data
            .get(145..),
        Some([5, 6, 7].as_slice())
    );
    Ok(())
}

#[tokio::test]
async fn all_requests_for_one_key_are_signed_and_keep_order() -> TestResult {
    let key = Secp256r1Key::from_slice(&[7; 32])?;
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let prepared = preparation(secp256r1_authority(&key)?, true, 2)?;
    let requests = prepared.signature_requests.clone();
    let signed = signer.sign_prepared_transaction(prepared).await?;
    for (index, request) in requests.iter().enumerate() {
        let data = &signed
            .message
            .instructions()
            .get(index * 2)
            .ok_or("missing precompile")?
            .data;
        let signature =
            Secp256r1Signature::from_slice(data.get(49..113).ok_or("missing signature")?)?;
        key.verifying_key()
            .verify(&request.message_hash, &signature)?;
    }
    Ok(())
}
