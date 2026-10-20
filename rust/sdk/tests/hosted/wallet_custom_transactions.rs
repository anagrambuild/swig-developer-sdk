use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::wire::tests::transaction::positional_prepared_responses,
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;

#[tokio::test]
async fn custom_transaction_rejects_positional_objects() -> Result<(), Box<dyn Error>> {
    for payload in positional_prepared_responses(prepared()?) {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                client.prepare_transaction(custom()).await,
                Err(PrepareTransactionError::InvalidResponse(
                    "malformed prepared transaction"
                ))
            ));
            assert_eq!(
                client
                    .http
                    .test
                    .lock()
                    .map_err(|_| "test transport lock")?
                    .requests
                    .len(),
                1
            );
        }
    }
    Ok(())
}

fn custom() -> PrepareTransactionArgs {
    PrepareTransactionArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        requester_authority: authority(),
        instructions: vec![Instruction {
            program_id: key(9),
            accounts: vec![
                AccountMeta::new(key(1), true),
                AccountMeta::new_readonly(key(2), false),
            ],
            data: vec![1, 2, 3],
        }],
        address_lookup_table_accounts: vec![key(7)],
    }
}

#[tokio::test]
async fn build_transaction_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client.prepare_transaction(custom()).await?;
    let context = json!({
        "network":"NETWORK_DEVNET", "swigAddress":key(3).to_string(), "feePayer":key(1).to_string(),
        "requesterAuthority":{"ed25519":{"publicKey":key(1).to_string()}},
    });
    let mut expected = json!({"instructions":[{"programId":key(9).to_string(),"accounts":[{"pubkey":key(1).to_string(),"isSigner":true,"isWritable":true},{"pubkey":key(2).to_string(),"isSigner":false,"isWritable":false}],"data":"AQID"}],"addressLookupTableAccounts":[key(7).to_string()]});
    expected
        .as_object_mut()
        .ok_or("expected object")?
        .extend(context.as_object().ok_or("expected context")?.clone());
    let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(request.url().path(), "/transaction/prepare/custom");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    assert_eq!(
        body, expected,
        "wrong payload for /transaction/prepare/custom"
    );
    Ok(())
}

#[tokio::test]
async fn posts_make_one_attempt_on_transport_and_server_failure() -> Result<(), Box<dyn Error>> {
    for transport_failure in [true, false] {
        let client = WalletClient::new(config())?;
        let response = if transport_failure {
            Err(HttpError::Timeout)
        } else {
            Ok(response(503, json!({"code":"UNAVAILABLE"})))
        };
        queue(&client.http, vec![response])?;
        let error = client
            .prepare_transaction(custom())
            .await
            .err()
            .ok_or("expected build_transaction error")?;
        assert!(matches!(
            (&error, transport_failure),
            (PrepareTransactionError::RequestTimedOut, true)
                | (PrepareTransactionError::ServiceUnavailable, false)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn resource_failures_keep_only_static_classes() -> Result<(), Box<dyn Error>> {
    for (status, body, grpc_status) in [
        (429, json!({}), None),
        (400, json!({"code":"RESOURCE_EXHAUSTED"}), None),
        (200, json!({}), Some("8")),
        (404, json!({"message":"secret-provider-detail"}), None),
        (
            400,
            json!({"code":"NOT_FOUND","message":"secret-provider-detail"}),
            None,
        ),
        (200, json!({"message":"secret-provider-detail"}), Some("5")),
    ] {
        let client = WalletClient::new(config())?;
        let mut response = response(status, body);
        if let Some(code) = grpc_status {
            response.headers_mut().insert(
                "grpc-status",
                reqwest::header::HeaderValue::from_static(code),
            );
        }
        queue(&client.http, vec![Ok(response)])?;
        let error = client
            .prepare_transaction(custom())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            PrepareTransactionError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn empty_instructions_fail_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut args = custom();
    args.instructions.clear();
    assert!(matches!(
        client.prepare_transaction(args).await,
        Err(PrepareTransactionError::InvalidInput(
            "instructions must not be empty"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "test transport lock")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn authority_and_lookup_capabilities_are_checked_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let secp256k1 = WalletAuthority::Secp256k1 {
        public_key: [0; 33],
    };
    let participant = WalletAuthority::ParticipantSet {
        address: key(7),
        role_id: Some(2),
    };
    let mut args = custom();
    args.requester_authority = secp256k1;
    assert!(matches!(
        client.prepare_transaction(args).await,
        Err(PrepareTransactionError::InvalidInput(
            "unsupported requester authority"
        ))
    ));
    let mut args = custom();
    args.requester_authority = participant;
    assert!(matches!(
        client.prepare_transaction(args).await,
        Err(PrepareTransactionError::InvalidInput(
            "participant-set requests do not support lookup tables"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "test transport lock")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn custom_accepts_only_wallet_fee_payer_and_requester_signer_accounts(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let (wallet, _) = Pubkey::try_find_program_address(
        &[b"swig-wallet-address", key(3).as_ref()],
        &solana_pubkey::pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB"),
    )
    .ok_or("wallet derivation failed")?;
    let mut args = custom();
    args.fee_payer = key(6);
    args.instructions
        .first_mut()
        .ok_or("missing instruction")?
        .accounts = vec![
        AccountMeta::new(wallet, true),
        AccountMeta::new(key(6), true),
        AccountMeta::new(key(1), true),
    ];
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client.prepare_transaction(args).await?;
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "test transport lock")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn custom_transactions_reject_extra_signers_and_duplicate_lookups(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let instruction = solana_instruction::Instruction {
        program_id: key(9),
        accounts: vec![solana_instruction::AccountMeta::new(key(8), true)],
        data: vec![],
    };
    assert!(matches!(
        client
            .prepare_transaction(PrepareTransactionArgs {
                swig_config_address: key(3),
                fee_payer: key(1),
                requester_authority: authority(),
                instructions: vec![instruction.clone()],
                address_lookup_table_accounts: vec![]
            })
            .await,
        Err(PrepareTransactionError::InvalidInput(
            "unsupported instruction signer"
        ))
    ));
    assert!(matches!(
        client
            .prepare_transaction(PrepareTransactionArgs {
                swig_config_address: key(3),
                fee_payer: key(1),
                requester_authority: authority(),
                instructions: vec![instruction],
                address_lookup_table_accounts: vec![key(5), key(5)]
            })
            .await,
        Err(PrepareTransactionError::InvalidInput(
            "duplicate lookup table address"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "poisoned test transport")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn build_transaction_returns_native_preparation() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client
        .prepare_transaction(PrepareTransactionArgs {
            swig_config_address: key(3),
            fee_payer: key(1),
            requester_authority: authority(),
            instructions: vec![solana_instruction::Instruction {
                program_id: key(9),
                accounts: vec![],
                data: vec![1, 2, 3],
            }],
            address_lookup_table_accounts: vec![],
        })
        .await?;
    let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    assert_eq!(body["instructions"][0]["data"], "AQID");
    Ok(())
}
