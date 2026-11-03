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
            assert_eq!(lock_digest(&before, false)?, hash);
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
                    Keccak256::new_with_prefix(
                        [
                            b"\x19Ethereum Signed Message:\n64".as_slice(),
                            hex::encode(hash).as_bytes()
                        ]
                        .concat()
                    ),
                    &signature,
                    recovery
                )?,
                *key.verifying_key()
            );
            let mut changed_transaction = signed.clone();
            change_lock_argument(&mut changed_transaction)?;
            let changed = lock_digest(&changed_transaction, false)?;
            assert!(key
                .verifying_key()
                .verify_digest(
                    Keccak256::new_with_prefix(
                        [
                            b"\x19Ethereum Signed Message:\n64".as_slice(),
                            hex::encode(changed).as_bytes()
                        ]
                        .concat()
                    ),
                    &signature
                )
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
                .get_mut(offset..offset + 65)
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
            assert_eq!(lock_digest(&before, true)?, hash);
            let signed = signer.sign_prepared_transaction(prepared).await?;
            let data = &signed
                .message
                .instructions()
                .first()
                .ok_or("instruction")?
                .data;
            let signature = R1Signature::from_slice(data.get(49..113).ok_or("signature")?)?;
            key.verifying_key().verify(&hash, &signature)?;
            let mut changed_transaction = signed.clone();
            change_lock_argument(&mut changed_transaction)?;
            let changed = lock_digest(&changed_transaction, true)?;
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

fn lock_digest(
    transaction: &solana_transaction::versioned::VersionedTransaction,
    secp256r1: bool,
) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let instruction = transaction
        .message
        .instructions()
        .last()
        .ok_or("lock instruction")?;
    let offset = match instruction.data.get(..2) {
        Some([25, 0]) => 88,
        Some([26, 0]) => 16,
        _ => return Err("lock discriminator".into()),
    };
    let mut payload = instruction
        .data
        .get(..offset)
        .ok_or("lock arguments")?
        .to_vec();
    for account in &instruction.accounts {
        let index = usize::from(*account);
        payload.extend_from_slice(
            transaction
                .message
                .static_account_keys()
                .get(index)
                .ok_or("account")?
                .as_ref(),
        );
        payload.extend_from_slice(&[
            u8::from(transaction.message.is_maybe_writable(index, None)),
            u8::from(transaction.message.is_signer(index)),
        ]);
        payload.extend_from_slice(&[0; 6]);
    }
    payload.extend_from_slice(
        instruction
            .data
            .get(offset..offset + 12)
            .ok_or("replay metadata")?,
    );
    if secp256r1 {
        let index = usize::from(*instruction.data.get(offset + 12).ok_or("sysvar index")?);
        let account = usize::from(*instruction.accounts.get(index).ok_or("sysvar account")?);
        assert_eq!(
            transaction
                .message
                .static_account_keys()
                .get(account)
                .ok_or("sysvar key")?
                .to_string(),
            "Sysvar1nstructions1111111111111111111111111"
        );
        Ok(Keccak256::digest(payload).into())
    } else {
        Ok(sha2::Sha256::digest(payload).into())
    }
}

fn change_lock_argument(
    transaction: &mut solana_transaction::versioned::VersionedTransaction,
) -> TestResult {
    let instructions = match &mut transaction.message {
        VersionedMessage::Legacy(m) => &mut m.instructions,
        VersionedMessage::V0(m) => &mut m.instructions,
    };
    *instructions
        .last_mut()
        .ok_or("lock instruction")?
        .data
        .get_mut(8)
        .ok_or("lock argument")? ^= 1;
    Ok(())
}
