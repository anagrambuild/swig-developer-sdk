use std::error::Error;

use serde_json::{json, Value};
use solana_keypair::Keypair;
use solana_message::{legacy::Message, v0, VersionedMessage};
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    AccountMeta, Instruction, Network, PaymasterClient, PaymasterSignError, RecentBlockhash,
    SignTransactionArgs,
};

fn transactions(
    versioned: bool,
) -> Result<(VersionedTransaction, VersionedTransaction), Box<dyn Error>> {
    let payer = Keypair::new_from_array([1; 32]);
    let cosigner = Keypair::new_from_array([2; 32]);
    let instructions = [Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array([3; 32]),
        accounts: vec![AccountMeta::new_readonly(cosigner.pubkey(), true)],
        data: vec![4],
    }];
    let hash = RecentBlockhash::new_from_array([5; 32]);
    let message = if versioned {
        VersionedMessage::V0(v0::Message::try_compile(
            &payer.pubkey(),
            &instructions,
            &[],
            hash,
        )?)
    } else {
        VersionedMessage::Legacy(Message::new_with_blockhash(
            &instructions,
            Some(&payer.pubkey()),
            &hash,
        ))
    };
    let bytes = message.serialize();
    let signed = VersionedTransaction {
        signatures: vec![payer.sign_message(&bytes), cosigner.sign_message(&bytes)],
        message,
    };
    let mut original = signed.clone();
    original.signatures[0] = Signature::default();
    Ok((original, signed))
}

fn encoded(transaction: &VersionedTransaction) -> Result<String, Box<dyn Error>> {
    Ok(bs58::encode(bincode::serialize(transaction)?).into_string())
}

#[tokio::test]
async fn sign_returns_verified_native_transaction_without_submitting() -> Result<(), Box<dyn Error>>
{
    for (versioned, network, external_id) in [
        (false, Network::Mainnet, None),
        (true, Network::Devnet, Some("match-17")),
        (true, Network::Devnet, Some("")),
    ] {
        let mut config = config();
        config.network = network;
        let client = PaymasterClient::new(config)?;
        let (original, signed) = transactions(versioned)?;
        let receipt = if versioned {
            json!({"data":{"requestId":"request-1","signedTransaction":encoded(&signed)?}})
        } else {
            json!({"request_id":"request-1","signed_transaction":encoded(&signed)?})
        };
        queue(&client.http, vec![Ok(response(200, receipt))])?;
        let mut expected = json!({
            "network": if network == Network::Devnet { "devnet" } else { "mainnet" },
            "base58_encoded_transaction": encoded(&original)?,
        });
        if let Some(id) = external_id {
            expected["external_id"] = json!(id);
        }
        let result = client
            .sign(SignTransactionArgs {
                transaction: original,
                external_id: external_id.map(str::to_owned),
            })
            .await?;
        assert_eq!(result.request_id, "request-1");
        assert_eq!(result.signed_transaction, signed);
        let transport = client.http.test.lock().map_err(|_| "lock")?;
        let [request] = transport.requests.as_slice() else {
            return Err("expected one request".into());
        };
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/paymaster/sign");
        assert_eq!(
            request.headers().get("authorization").ok_or("auth")?,
            "Bearer test-only-api-key"
        );
        let body = request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("request body")?;
        assert_eq!(serde_json::from_slice::<Value>(body)?, expected);
    }
    Ok(())
}

#[tokio::test]
async fn sign_rejects_changed_message_cosignatures_and_invalid_payer_signature(
) -> Result<(), Box<dyn Error>> {
    let (input, signed) = transactions(true)?;
    let mut changed = signed.clone();
    changed
        .message
        .set_recent_blockhash(RecentBlockhash::new_from_array([6; 32]));
    let payer = Keypair::new_from_array([1; 32]);
    changed.signatures[0] = payer.sign_message(&changed.message.serialize());
    let mut stripped = signed.clone();
    stripped.signatures[1] = Signature::default();
    let mut wrong_key = signed.clone();
    wrong_key.signatures[0] = wrong_key.signatures[1];
    for (returned, reason) in [
        (changed, "transaction message changed"),
        (stripped, "nonpayer signatures changed"),
        (input.clone(), "invalid fee payer signature"),
        (wrong_key, "invalid fee payer signature"),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({"request_id":"abc","signed_transaction":encoded(&returned)?}),
            ))],
        )?;
        let result = client
            .sign(SignTransactionArgs {
                transaction: input.clone(),
                external_id: None,
            })
            .await;
        assert!(
            matches!(result, Err(PaymasterSignError::InvalidResponse(error)) if error == reason)
        );
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn sign_rejects_malformed_receipts() -> Result<(), Box<dyn Error>> {
    let (input, signed) = transactions(false)?;
    let mut trailing = bincode::serialize(&signed)?;
    trailing.push(0);
    for (receipt, reason) in [
        (json!({"request_id":"a"}), "malformed signing receipt"),
        (
            json!({"signed_transaction":encoded(&signed)?}),
            "malformed signing receipt",
        ),
        (
            json!({"request_id":" ", "signed_transaction":encoded(&signed)?}),
            "missing request ID",
        ),
        (
            json!({"request_id":"a","signed_transaction":"0"}),
            "invalid signed transaction",
        ),
        (
            json!({"request_id":"a","signed_transaction":"1"}),
            "invalid signed transaction length",
        ),
        (
            json!({"request_id":"a","signed_transaction":bs58::encode(trailing).into_string()}),
            "invalid signed transaction",
        ),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, receipt))])?;
        let result = client
            .sign(SignTransactionArgs {
                transaction: input.clone(),
                external_id: None,
            })
            .await;
        assert!(
            matches!(result, Err(PaymasterSignError::InvalidResponse(error)) if error == reason)
        );
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn sign_rejects_invalid_input_before_sending_and_does_not_retry_after_send(
) -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    let (input, _) = transactions(false)?;
    let mut invalid = input.clone();
    invalid.signatures.clear();
    let result = client
        .sign(SignTransactionArgs {
            transaction: invalid,
            external_id: None,
        })
        .await;
    assert!(matches!(
        result,
        Err(PaymasterSignError::InvalidInput("invalid transaction"))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    for (outcome, expected) in [
        (Err(HttpError::Timeout), "request timed out"),
        (Ok(response(503, json!({}))), "service unavailable"),
        (
            Ok(response(400, json!({"code":"INVALID_ARGUMENT"}))),
            "invalid signing input: signing rejected",
        ),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![outcome])?;
        let result = client
            .sign(SignTransactionArgs {
                transaction: input.clone(),
                external_id: Some("match-17".into()),
            })
            .await;
        assert_eq!(
            result.err().ok_or("expected failure")?.to_string(),
            expected
        );
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn sign_reports_a_fee_payer_missing_from_the_project() -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    let (transaction, _) = transactions(false)?;
    queue(&client.http, vec![Ok(response(404, json!({"code": 5})))])?;
    let result = client
        .sign(SignTransactionArgs {
            transaction,
            external_id: None,
        })
        .await;
    assert!(matches!(result, Err(PaymasterSignError::PaymasterNotFound)));
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        1
    );
    Ok(())
}
