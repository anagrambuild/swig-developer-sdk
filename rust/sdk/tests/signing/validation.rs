use super::support::{preparation, secp256r1_authority, TestResult};
use crate::{
    ParticipantSetApprovalPlan, Secp256r1TransactionSigner, TransactionSignError,
    TransactionSigner, WalletAuthority,
};
use p256::ecdsa::SigningKey as Secp256r1Key;
use solana_hash::Hash;
use solana_keypair::Keypair;
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;
#[tokio::test]
async fn rejects_wrong_key_curve_and_unsupported_authorities_without_partial_output() -> TestResult
{
    let key = Secp256r1Key::from_slice(&[7; 32])?;
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let prepared = preparation(secp256r1_authority(&key)?, false, 2)?;
    for authority in [
        secp256r1_authority(&Secp256r1Key::from_slice(&[8; 32])?)?,
        WalletAuthority::Secp256k1 {
            public_key: [2; 33],
        },
        WalletAuthority::Ed25519 {
            public_key: Pubkey::new_from_array([1; 32]),
        },
        WalletAuthority::ParticipantSet {
            address: Pubkey::new_from_array([2; 32]),
            role_id: None,
        },
    ] {
        let mut input = prepared.clone();
        input
            .signature_requests
            .last_mut()
            .ok_or("missing request")?
            .authority = authority;
        assert!(matches!(
            signer.sign_prepared_transaction(input).await,
            Err(TransactionSignError::AuthorityMismatch)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn pending_participant_approvals_block_signing_even_without_requests() -> TestResult {
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let mut prepared = preparation(
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
        false,
        0,
    )?;
    prepared.participant_set_approval_plan = Some(ParticipantSetApprovalPlan {
        participant_set_address: Pubkey::new_from_array([1; 32]),
        role_id: 0,
        expiration_slot: 900,
        nonce: 1,
        transaction_digest: [4; 32],
        compilation_envelope: "fixture-envelope".into(),
        threshold: 1,
        members: vec![],
    });
    assert!(matches!(
        signer.sign_prepared_transaction(prepared).await,
        Err(TransactionSignError::ParticipantApprovalsPending)
    ));
    Ok(())
}

#[tokio::test]
async fn rejects_outer_signatures_when_patching_but_preserves_no_request_transactions() -> TestResult
{
    let signers: Vec<(Box<dyn TransactionSigner>, WalletAuthority)> = vec![(
        Box::new(Secp256r1TransactionSigner::from_bytes(&[7; 32])?),
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
    )];
    for (signer, authority) in signers {
        let mut prepared = preparation(authority, true, 1)?;
        prepared.transaction = VersionedTransaction::try_new(
            prepared.transaction.message,
            &[&Keypair::new_from_array([9; 32])],
        )?;
        assert!(prepared.transaction.verify_and_hash_message().is_ok());
        assert!(matches!(
            signer.sign_prepared_transaction(prepared.clone()).await,
            Err(TransactionSignError::OuterSignaturesPresent)
        ));
        prepared.signature_requests.clear();
        let expected = prepared.transaction.clone();
        assert_eq!(signer.sign_prepared_transaction(prepared).await?, expected);
    }
    Ok(())
}

#[tokio::test]
async fn rejects_duplicate_requests_and_ambiguous_instruction_targets() -> TestResult {
    let signers: Vec<(Box<dyn TransactionSigner>, WalletAuthority)> = vec![(
        Box::new(Secp256r1TransactionSigner::from_bytes(&[7; 32])?),
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
    )];
    for (signer, authority) in signers {
        let prepared = preparation(authority, false, 1)?;
        let mut duplicate = prepared.clone();
        duplicate.signature_requests.push(
            duplicate
                .signature_requests
                .first()
                .ok_or("missing request")?
                .clone(),
        );
        assert!(matches!(
            signer.sign_prepared_transaction(duplicate).await,
            Err(TransactionSignError::DuplicateSigningTarget)
        ));
        let mut ambiguous = prepared;
        if let VersionedMessage::Legacy(message) = &mut ambiguous.transaction.message {
            message.instructions.push(
                message
                    .instructions
                    .first()
                    .ok_or("missing instruction")?
                    .clone(),
            );
        }
        assert!(matches!(
            signer.sign_prepared_transaction(ambiguous).await,
            Err(TransactionSignError::AmbiguousSigningTarget)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn rejects_malformed_transaction_and_blockhash_metadata() -> TestResult {
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let prepared = preparation(
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
        true,
        1,
    )?;
    let mut malformed = prepared.clone();
    malformed.transaction.signatures.clear();
    assert!(matches!(
        signer.sign_prepared_transaction(malformed).await,
        Err(TransactionSignError::InvalidPreparation(
            "invalid Solana transaction structure"
        ))
    ));
    let mut malformed = prepared.clone();
    if let VersionedMessage::V0(message) = &mut malformed.transaction.message {
        message
            .instructions
            .first_mut()
            .ok_or("missing instruction")?
            .program_id_index = u8::MAX;
    }
    assert!(matches!(
        signer.sign_prepared_transaction(malformed).await,
        Err(TransactionSignError::InvalidPreparation(
            "invalid Solana transaction structure"
        ))
    ));
    let mut mismatched = prepared;
    mismatched.recent_blockhash = Some(Hash::new_from_array([0; 32]));
    assert!(matches!(
        signer.sign_prepared_transaction(mismatched).await,
        Err(TransactionSignError::InvalidPreparation(
            "recent blockhash does not match the transaction"
        ))
    ));
    Ok(())
}

#[test]
fn scalar_constructors_reject_zero_and_out_of_range_keys() {
    for invalid in [[0; 32], [u8::MAX; 32]] {
        assert!(Secp256r1TransactionSigner::from_bytes(&invalid).is_err());
    }
}

#[tokio::test]
async fn invalid_later_request_cannot_return_a_partially_signed_transaction() -> TestResult {
    let signer = Secp256r1TransactionSigner::from_bytes(&[7; 32])?;
    let mut prepared = preparation(
        secp256r1_authority(&Secp256r1Key::from_slice(&[7; 32])?)?,
        true,
        2,
    )?;
    prepared
        .signature_requests
        .last_mut()
        .ok_or("missing request")?
        .message_hash = [0; 32];
    let unchanged = prepared.transaction.clone();
    assert!(matches!(
        signer.sign_prepared_transaction(prepared.clone()).await,
        Err(TransactionSignError::SigningTargetNotFound)
    ));
    assert_eq!(prepared.transaction, unchanged);
    Ok(())
}
