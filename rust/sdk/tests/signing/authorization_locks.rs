use super::support::{lock_preparation, secp256k1_authority, secp256r1_authority, TestResult};
use crate::{
    Secp256k1TransactionSigner, Secp256r1TransactionSigner, TransactionSignError, TransactionSigner,
};
use k256::ecdsa::{
    signature::DigestVerifier, RecoveryId, Signature as K1Signature, SigningKey as K1Key,
    VerifyingKey,
};
use p256::ecdsa::{signature::Verifier, Signature as R1Signature, SigningKey as R1Key};
use sha3::{Digest, Keccak256};
use solana_message::VersionedMessage;

#[tokio::test]
async fn signs_both_lock_operations_with_k1_and_preserves_headers_and_outer_message() -> TestResult
{
    let key = K1Key::from_slice(&[3; 32])?;
    let signer = Secp256k1TransactionSigner::from_bytes(&[3; 32])?;
    for add in [true, false] {
        for versioned in [true, false] {
            let prepared = lock_preparation(secp256k1_authority(&key)?, versioned, add)?;
            let before = prepared.transaction.clone();
            let hash = prepared
                .signature_requests
                .first()
                .ok_or("request")?
                .message_hash;
            let signed = signer.sign_prepared_transaction(prepared).await?;
            let data = &signed
                .message
                .instructions()
                .first()
                .ok_or("instruction")?
                .data;
            let offset = if add { 88 + 12 } else { 16 + 12 };
            let signature =
                K1Signature::from_slice(data.get(offset..offset + 64).ok_or("signature")?)?;
            let recovery = RecoveryId::try_from(
                data.get(offset + 64)
                    .ok_or("recovery")?
                    .checked_sub(27)
                    .ok_or("invalid recovery")?,
            )?;
            assert_eq!(
                VerifyingKey::recover_from_digest(
                    Keccak256::new_with_prefix(hex::encode(hash)),
                    &signature,
                    recovery
                )?,
                *key.verifying_key()
            );
            let mut changed = hash;
            changed[0] ^= 1;
            assert!(key
                .verifying_key()
                .verify_digest(Keccak256::new_with_prefix(hex::encode(changed)), &signature)
                .is_err());
            let mut restored = signed;
            let instructions = match &mut restored.message {
                VersionedMessage::Legacy(m) => &mut m.instructions,
                VersionedMessage::V0(m) => &mut m.instructions,
            };
            instructions
                .first_mut()
                .ok_or("instruction")?
                .data
                .get_mut(offset..)
                .ok_or("signature")?
                .fill(0);
            assert_eq!(restored, before);
        }
    }
    Ok(())
}

#[tokio::test]
async fn signs_both_lock_operations_with_r1_and_preserves_arguments() -> TestResult {
    let key = R1Key::from_slice(&[7; 32])?;
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    for add in [true, false] {
        for versioned in [true, false] {
            let prepared = lock_preparation(secp256r1_authority(&key)?, versioned, add)?;
            let before = prepared.transaction.clone();
            let hash = prepared
                .signature_requests
                .first()
                .ok_or("request")?
                .message_hash;
            let signed = signer.sign_prepared_transaction(prepared).await?;
            let data = &signed
                .message
                .instructions()
                .first()
                .ok_or("instruction")?
                .data;
            let signature = R1Signature::from_slice(data.get(49..113).ok_or("signature")?)?;
            key.verifying_key().verify(&hash, &signature)?;
            let mut changed = hash;
            changed[0] ^= 1;
            assert!(key.verifying_key().verify(&changed, &signature).is_err());
            let mut restored = signed;
            let instructions = match &mut restored.message {
                VersionedMessage::Legacy(m) => &mut m.instructions,
                VersionedMessage::V0(m) => &mut m.instructions,
            };
            instructions
                .first_mut()
                .ok_or("instruction")?
                .data
                .get_mut(49..113)
                .ok_or("signature")?
                .fill(0);
            assert_eq!(restored, before);
        }
    }
    Ok(())
}

#[tokio::test]
async fn lock_signing_rejects_wrong_replay_metadata_truncation_and_prefixes() -> TestResult {
    let key = K1Key::from_slice(&[3; 32])?;
    let signer = Secp256k1TransactionSigner::from_bytes(&[3; 32])?;
    for add in [true, false] {
        for case in 0..5 {
            let mut prepared = lock_preparation(secp256k1_authority(&key)?, false, add)?;
            match case {
                0 => {
                    prepared
                        .signature_requests
                        .first_mut()
                        .ok_or("request")?
                        .slot += 1
                }
                1 => {
                    prepared
                        .signature_requests
                        .first_mut()
                        .ok_or("request")?
                        .counter += 1
                }
                _ => {
                    let VersionedMessage::Legacy(message) = &mut prepared.transaction.message
                    else {
                        return Err("legacy fixture".into());
                    };
                    let data = &mut message.instructions.first_mut().ok_or("instruction")?.data;
                    match case {
                        2 => {
                            data.truncate(if add { 87 } else { 15 });
                        }
                        3 => {
                            data.pop();
                        }
                        _ => data.extend_from_slice(b"prefix"),
                    }
                }
            }
            assert!(matches!(
                signer.sign_prepared_transaction(prepared).await,
                Err(TransactionSignError::SigningTargetNotFound
                    | TransactionSignError::InvalidPreparation(_))
            ));
        }
    }
    Ok(())
}
