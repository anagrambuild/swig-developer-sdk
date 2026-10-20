use std::{error::Error, sync::Arc};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use k256::ecdsa::{RecoveryId, Signature as Secp256k1Signature, VerifyingKey};
use sha2::Sha256;
use sha3::{Digest, Keccak256};
use signature::{DigestVerifier as _, Signer as _, Verifier as _};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer as _;

use crate::{
    Ed25519ParticipantSigner, ParticipantApproval, ParticipantApprovalError,
    ParticipantApprovalRequest, ParticipantSigner, Secp256k1ParticipantSigner,
    Secp256r1ParticipantSigner, WalletAuthority,
};

#[tokio::test]
async fn ed25519_signs_exact_challenge_and_preserves_member_index() -> Result<(), Box<dyn Error>> {
    let keypair = Arc::new(Keypair::new_from_array([11; 32]));
    let public_key = keypair.pubkey();
    let signer = Ed25519ParticipantSigner::new(Arc::clone(&keypair));
    assert_eq!(Arc::strong_count(&keypair), 2);
    drop(keypair);

    let approval = signer
        .approve(ParticipantApprovalRequest {
            member_index: 7,
            authority: WalletAuthority::Ed25519 { public_key },
            challenge: [0xab; 32],
        })
        .await?;
    let ParticipantApproval::Ed25519 {
        member_index,
        signature,
    } = approval
    else {
        return Err("expected Ed25519 approval".into());
    };
    assert_eq!(member_index, 7);
    assert_eq!(signature.len(), 64);
    let signature = Signature::try_from(signature.as_slice())?;
    assert!(signature.verify(public_key.as_ref(), &[0xab; 32]));
    assert!(!signature.verify(public_key.as_ref(), &[0xac; 32]));
    assert!(!signature.verify(public_key.as_ref(), hex::encode([0xab; 32]).as_bytes()));
    Ok(())
}

#[tokio::test]
async fn secp256k1_uses_personal_sign_lowercase_hex_without_0x() -> Result<(), Box<dyn Error>> {
    let key = Arc::new(k256::ecdsa::SigningKey::from_slice(&[12; 32])?);
    let public_key: [u8; 33] = key
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .try_into()?;
    let signer = Secp256k1ParticipantSigner::new(Arc::clone(&key));
    assert_eq!(Arc::strong_count(&key), 2);
    let approval = signer
        .approve(ParticipantApprovalRequest {
            member_index: 8,
            authority: WalletAuthority::Secp256k1 { public_key },
            challenge: [0xab; 32],
        })
        .await?;
    let ParticipantApproval::Secp256k1 {
        member_index,
        signature,
    } = approval
    else {
        return Err("expected secp256k1 approval".into());
    };
    assert_eq!(member_index, 8);
    assert_eq!(signature.len(), 65);
    let recovery_id = signature.last().copied().ok_or("missing recovery id")?;
    assert!(recovery_id <= 1);
    let recovery_id = RecoveryId::try_from(recovery_id)?;
    let signature =
        Secp256k1Signature::from_slice(signature.get(..64).ok_or("missing signature bytes")?)?;
    assert!(signature.normalize_s().is_none());
    let message = "ab".repeat(32);
    let digest = Keccak256::new()
        .chain_update(b"\x19Ethereum Signed Message:\n64")
        .chain_update(message.as_bytes());
    let recovered = VerifyingKey::recover_from_digest(digest, &signature, recovery_id)?;
    assert_eq!(recovered.to_encoded_point(true).as_bytes(), public_key);

    for incompatible_message in [message.to_uppercase(), format!("0x{message}")] {
        let digest = Keccak256::new()
            .chain_update(b"\x19Ethereum Signed Message:\n64")
            .chain_update(incompatible_message.as_bytes());
        assert!(key
            .verifying_key()
            .verify_digest(digest, &signature)
            .is_err());
    }
    let without_prefix = Keccak256::new().chain_update(message.as_bytes());
    assert!(key
        .verifying_key()
        .verify_digest(without_prefix, &signature)
        .is_err());
    Ok(())
}

#[tokio::test]
async fn secp256r1_software_envelope_matches_compact_participant_format(
) -> Result<(), Box<dyn Error>> {
    let key = Arc::new(p256::ecdsa::SigningKey::from_slice(&[13; 32])?);
    let public_key: [u8; 33] = key
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .try_into()?;
    let signer = Secp256r1ParticipantSigner::new(Arc::clone(&key));
    assert_eq!(Arc::strong_count(&key), 2);
    let approval = signer
        .approve(ParticipantApprovalRequest {
            member_index: 9,
            authority: WalletAuthority::Secp256r1 { public_key },
            challenge: [0; 32],
        })
        .await?;
    let ParticipantApproval::WebAuthnP256 {
        member_index,
        authenticator_data,
        client_data_json,
        signature,
    } = approval
    else {
        return Err("expected WebAuthnP256 approval".into());
    };
    assert_eq!(member_index, 9);
    assert_eq!(authenticator_data.len(), 37);
    let expected_rp_hash: [u8; 32] = Sha256::digest(b"swig.invalid").into();
    assert_eq!(
        authenticator_data.get(..32),
        Some(expected_rp_hash.as_slice())
    );
    assert_eq!(authenticator_data.get(32), Some(&0x01));
    assert_eq!(authenticator_data.get(33..), Some([0_u8; 4].as_slice()));
    assert_eq!(
        client_data_json,
        br#"{"type":"webauthn.get","challenge":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","origin":"https://swig.invalid"}"#,
    );
    let decoded: serde_json::Value = serde_json::from_slice(&client_data_json)?;
    assert_eq!(decoded.as_object().ok_or("expected JSON object")?.len(), 3);
    let challenge = decoded
        .get("challenge")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing challenge")?;
    assert_eq!(URL_SAFE_NO_PAD.decode(challenge)?, [0_u8; 32]);

    assert_eq!(signature.len(), 64);
    let signature = p256::ecdsa::Signature::from_slice(&signature)?;
    assert!(signature.normalize_s().is_none());
    let mut message = authenticator_data;
    message.extend_from_slice(&Sha256::digest(&client_data_json));
    key.verifying_key().verify(&message, &signature)?;
    assert!(key.verifying_key().verify(&[0_u8; 32], &signature).is_err());

    let wrong_challenge_json = String::from_utf8(client_data_json)?
        .replace(challenge, &URL_SAFE_NO_PAD.encode([1_u8; 32]));
    let mut wrong_message = message
        .get(..37)
        .ok_or("missing authenticator data")?
        .to_vec();
    wrong_message.extend_from_slice(&Sha256::digest(wrong_challenge_json.as_bytes()));
    assert!(key
        .verifying_key()
        .verify(&wrong_message, &signature)
        .is_err());
    Ok(())
}

#[tokio::test]
async fn secp256r1_normalizes_a_high_s_signature() -> Result<(), Box<dyn Error>> {
    let mut high_s_case_verified = false;
    for seed in 1_u8..=8 {
        let key = Arc::new(p256::ecdsa::SigningKey::from_slice(&[seed; 32])?);
        let public_key = key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .try_into()?;
        let signer = Secp256r1ParticipantSigner::new(Arc::clone(&key));
        let approval = signer
            .approve(ParticipantApprovalRequest {
                member_index: 0,
                authority: WalletAuthority::Secp256r1 { public_key },
                challenge: [0; 32],
            })
            .await?;
        let ParticipantApproval::WebAuthnP256 {
            authenticator_data,
            client_data_json,
            signature,
            ..
        } = approval
        else {
            return Err("expected WebAuthnP256 approval".into());
        };
        let mut message = authenticator_data;
        message.extend_from_slice(&Sha256::digest(&client_data_json));
        let raw: p256::ecdsa::Signature = key.try_sign(&message)?;
        if let Some(normalized) = raw.normalize_s() {
            let expected_signature: [u8; 64] = normalized.to_bytes().into();
            assert_eq!(signature, expected_signature);
            key.verifying_key().verify(&message, &normalized)?;
            high_s_case_verified = true;
        }
    }
    assert!(high_s_case_verified);
    Ok(())
}

#[tokio::test]
async fn participant_signers_reject_other_keys_before_signing() -> Result<(), Box<dyn Error>> {
    let ed25519 = Ed25519ParticipantSigner::new(Arc::new(Keypair::new_from_array([11; 32])));
    let secp256k1 = Secp256k1ParticipantSigner::from_bytes(&[12; 32])?;
    let secp256r1 = Secp256r1ParticipantSigner::from_bytes(&[13; 32])?;
    let other_secp256k1 = k256::ecdsa::SigningKey::from_slice(&[14; 32])?;
    let other_secp256r1 = p256::ecdsa::SigningKey::from_slice(&[15; 32])?;
    let cases: Vec<(&dyn ParticipantSigner, WalletAuthority)> = vec![
        (
            &ed25519,
            WalletAuthority::Ed25519 {
                public_key: Keypair::new_from_array([16; 32]).pubkey(),
            },
        ),
        (
            &secp256k1,
            WalletAuthority::Secp256k1 {
                public_key: other_secp256k1
                    .verifying_key()
                    .to_encoded_point(true)
                    .as_bytes()
                    .try_into()?,
            },
        ),
        (
            &secp256r1,
            WalletAuthority::Secp256r1 {
                public_key: other_secp256r1
                    .verifying_key()
                    .to_encoded_point(true)
                    .as_bytes()
                    .try_into()?,
            },
        ),
    ];
    for (signer, authority) in cases {
        assert!(matches!(
            signer
                .approve(ParticipantApprovalRequest {
                    member_index: 1,
                    authority,
                    challenge: [1; 32]
                })
                .await,
            Err(ParticipantApprovalError::AuthorityMismatch),
        ));
    }
    Ok(())
}

#[tokio::test]
async fn participant_signers_reject_every_unsupported_authority() -> Result<(), Box<dyn Error>> {
    let ed25519 = Ed25519ParticipantSigner::new(Arc::new(Keypair::new_from_array([11; 32])));
    let secp256k1 = Secp256k1ParticipantSigner::from_bytes(&[12; 32])?;
    let secp256r1 = Secp256r1ParticipantSigner::from_bytes(&[13; 32])?;
    let authorities = [
        WalletAuthority::Ed25519 {
            public_key: Pubkey::new_from_array([1; 32]),
        },
        WalletAuthority::Secp256k1 {
            public_key: [2; 33],
        },
        WalletAuthority::Secp256r1 {
            public_key: [3; 33],
        },
        WalletAuthority::ParticipantSet {
            address: Pubkey::new_from_array([4; 32]),
            role_id: None,
        },
    ];
    let signers: [&dyn ParticipantSigner; 3] = [&ed25519, &secp256k1, &secp256r1];
    for (signer_index, signer) in signers.iter().enumerate() {
        for (authority_index, authority) in authorities.iter().enumerate() {
            if signer_index == authority_index {
                continue;
            }
            assert!(matches!(
                signer
                    .approve(ParticipantApprovalRequest {
                        member_index: 0,
                        authority: authority.clone(),
                        challenge: [0; 32]
                    })
                    .await,
                Err(ParticipantApprovalError::InvalidRequest(_)),
            ));
        }
    }
    Ok(())
}

#[test]
fn secp256_constructors_reject_zero_and_out_of_range_private_scalars() {
    for invalid_scalar in [[0_u8; 32], [0xff_u8; 32]] {
        assert!(Secp256k1ParticipantSigner::from_bytes(&invalid_scalar).is_err());
        assert!(Secp256r1ParticipantSigner::from_bytes(&invalid_scalar).is_err());
    }
}
