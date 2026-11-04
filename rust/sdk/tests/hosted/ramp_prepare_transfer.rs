use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    ramp_support::{client, transfer},
    types::transaction::{encode_transaction, tests::native_transaction},
    *,
};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;
const SECP256R1_KEY: [u8; 33] = [
    0x03, 0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47, 0xf8, 0xbc, 0xe6, 0xe5, 0x63, 0xa4, 0x40,
    0xf2, 0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0, 0xf4, 0xa1, 0x39, 0x45, 0xd8, 0x98, 0xc2,
    0x96,
];

fn preparation(signature_requests: Value) -> Result<Value, Box<dyn Error>> {
    let transaction = native_transaction(false);
    Ok(json!({"preparedTransfer": {
        "transfer":transfer("transfer"),
        "preparedTransaction":{
            "transaction":encode_transaction(&transaction)?,
            "network":"NETWORK_MAINNET",
            "recentBlockhash":transaction.message.recent_blockhash().to_string(),
            "signatureRequests":signature_requests
        },
        "deposit":{"address":Pubkey::new_from_array([7;32]).to_string(),"amount":{"asset":{"sol":{}},"baseUnits":"12"}}
    }}))
}

fn prepare_args() -> PrepareRampTransferArgs {
    PrepareRampTransferArgs {
        order_id: "saved".into(),
        requester_authority: WalletAuthority::Ed25519 {
            public_key: Pubkey::new_from_array([1; 32]),
        },
        fee_payer: Pubkey::new_from_array([1; 32]),
    }
}

#[tokio::test]
async fn positional_preparation_objects_are_rejected_without_retry() -> Result<(), Box<dyn Error>> {
    let valid_request = json!({"scheme":"AUTHORITY_SIGNATURE_SCHEME_SECP256R1","signer":hex::encode(SECP256R1_KEY),"messageHash":"11".repeat(32),"slot":"7","counter":0});
    let positional_request = json!([
        "AUTHORITY_SIGNATURE_SCHEME_SECP256R1",
        hex::encode(SECP256R1_KEY),
        "11".repeat(32),
        "7",
        0
    ]);
    let mut payload = preparation(json!([]))?;
    payload["preparedTransfer"]["preparedTransaction"]["wallet"] = json!({
        "swigConfigAddress":Pubkey::new_from_array([7;32]).to_string(),
        "walletAddress":Pubkey::new_from_array([8;32]).to_string()
    });
    let prepared = &payload["preparedTransfer"];
    let transaction = &prepared["preparedTransaction"];
    let deposit = &prepared["deposit"];
    for (path, replacement) in [
        ("", json!([prepared])),
        (
            "/preparedTransfer",
            json!([prepared["transfer"], transaction, deposit]),
        ),
        (
            "/preparedTransfer/transfer",
            json!([
                "transfer",
                "TRANSFER_STATE_PREPARED",
                null,
                "2026-10-19T12:00:00Z"
            ]),
        ),
        (
            "/preparedTransfer/deposit",
            json!([deposit["address"], deposit["amount"]]),
        ),
        ("/preparedTransfer/deposit/amount", json!([{"sol":{}},"12"])),
        ("/preparedTransfer/deposit/amount/asset/sol", json!([])),
        (
            "/preparedTransfer/preparedTransaction",
            json!([
                transaction["transaction"],
                null,
                null,
                null,
                "NETWORK_MAINNET",
                transaction["recentBlockhash"],
                null,
                [],
                null
            ]),
        ),
        (
            "/preparedTransfer/preparedTransaction/wallet",
            json!([
                transaction["wallet"]["swigConfigAddress"],
                transaction["wallet"]["walletAddress"]
            ]),
        ),
        (
            "/preparedTransfer/preparedTransaction/signatureRequests",
            json!([valid_request, positional_request]),
        ),
    ] {
        let mut invalid = payload.clone();
        *invalid.pointer_mut(path).ok_or("preparation path")? = replacement;
        for body in [invalid.clone(), json!({"data":invalid})] {
            let ramp = client(Network::Mainnet)?;
            queue(&ramp.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                ramp.prepare_transfer(prepare_args()).await,
                Err(PrepareRampTransferError::InvalidResponse(
                    "malformed Ramp preparation"
                ))
            ));
            assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn preparation_aliases_null_optionals_and_request_lists_remain_accepted(
) -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    let payload = preparation(json!([]))?;
    let transaction = &payload["preparedTransfer"]["preparedTransaction"];
    queue(
        &ramp.http,
        vec![Ok(response(
            200,
            json!({"data":{"prepared_transfer":{
                "transfer":{"transfer_id":"transfer","state":"TRANSFER_STATE_PREPARED",
                    "solana_signature":null,"expires_at":"2026-10-19T12:00:00Z"},
                "prepared_transaction":{"unsigned_transaction":transaction["transaction"],
                    "network":"NETWORK_MAINNET","recent_blockhash":transaction["recentBlockhash"],
                    "wallet":null,"expires_at":null,"participant_set_approval_plan":null,
                    "signature_requests":[]},
                "deposit":payload["preparedTransfer"]["deposit"]
            }}}),
        ))],
    )?;
    let prepared = ramp.prepare_transfer(prepare_args()).await?;
    assert!(prepared.prepared_transaction.signature_requests.is_empty());
    assert!(prepared
        .prepared_transaction
        .participant_set_approval_plan
        .is_none());
    assert!(prepared.prepared_transaction.wallet.is_none());
    assert_eq!(prepared.deposit.amount.base_units, 12);
    Ok(())
}

#[tokio::test]
async fn preparation_decodes_legacy_metadata_and_checks_payer_and_requester(
) -> Result<(), Box<dyn Error>> {
    let client = client(Network::Devnet)?;
    let mut missing_network = preparation(json!([]))?;
    missing_network
        .pointer_mut("/preparedTransfer/preparedTransaction")
        .and_then(Value::as_object_mut)
        .ok_or("prepared")?
        .remove("network");
    queue(&client.http, vec![Ok(response(200, missing_network))])?;
    assert!(matches!(
        client.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::InvalidResponse(_))
    ));

    let mut versioned = preparation(json!([]))?;
    versioned["preparedTransfer"]["preparedTransaction"]["transaction"] =
        json!(encode_transaction(&native_transaction(true))?);
    let mut pending = preparation(json!([]))?;
    pending["preparedTransfer"]["preparedTransaction"]["participantSetApprovalPlan"] = json!({
        "participantSetAddress":Pubkey::new_from_array([7;32]).to_string(), "roleId":0,
        "expirationSlot":"100", "nonce":1, "transactionDigest":"ab".repeat(32),
        "compilationEnvelope":"opaque", "threshold":1, "members":[{
            "memberIndex":0, "authority":{"ed25519":{"publicKey":Pubkey::new_from_array([1;32]).to_string()}}, "challenge":"cd".repeat(32)
        }]
    });
    let mut wrong_hash = preparation(json!([]))?;
    wrong_hash["preparedTransfer"]["preparedTransaction"]["recentBlockhash"] =
        json!(crate::RecentBlockhash::new_from_array([9; 32]).to_string());
    let mut nonprepared = preparation(json!([]))?;
    nonprepared["preparedTransfer"]["transfer"]["state"] = json!("TRANSFER_STATE_EXPIRED");
    let mut missing_transfer = preparation(json!([]))?;
    missing_transfer["preparedTransfer"]["transfer"]["transferId"] = json!("");
    let mut zero_deposit = preparation(json!([]))?;
    zero_deposit["preparedTransfer"]["deposit"]["amount"]["baseUnits"] = json!("0");
    let mut invalid_deposit = preparation(json!([]))?;
    invalid_deposit["preparedTransfer"]["deposit"]["address"] = json!("invalid");
    for body in [
        versioned,
        pending,
        wrong_hash,
        nonprepared,
        missing_transfer,
        zero_deposit,
        invalid_deposit,
    ] {
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.prepare_transfer(prepare_args()).await,
            Err(PrepareRampTransferError::InvalidResponse(_))
        ));
    }
    for requester_is_wrong in [false, true] {
        queue(
            &client.http,
            vec![Ok(response(200, preparation(json!([]))?))],
        )?;
        let mut args = prepare_args();
        if requester_is_wrong {
            args.requester_authority = WalletAuthority::Ed25519 {
                public_key: Pubkey::new_from_array([2; 32]),
            };
        } else {
            args.fee_payer = Pubkey::new_from_array([2; 32]);
        }
        assert!(matches!(
            client.prepare_transfer(args).await,
            Err(PrepareRampTransferError::InvalidResponse(_))
        ));
    }
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        10
    );
    Ok(())
}

#[tokio::test]
async fn preparation_rejects_mismatched_or_unexpected_signature_requests(
) -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    let valid_request = json!({"scheme":"AUTHORITY_SIGNATURE_SCHEME_SECP256R1","signer":hex::encode(SECP256R1_KEY),"messageHash":"11".repeat(32),"slot":"7","counter":0});
    queue(
        &client.http,
        vec![Ok(response(
            200,
            preparation(json!([valid_request.clone()]))?,
        ))],
    )?;
    assert!(matches!(
        client.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::InvalidResponse(_))
    ));
    for (field, value) in [
        (
            "signer",
            json!(format!("02{}", &hex::encode(SECP256R1_KEY)[2..])),
        ),
        ("signer", json!("bad")),
        ("messageHash", json!("bad")),
        ("slot", json!("18446744073709551616")),
        ("counter", json!(-1)),
    ] {
        let mut request = valid_request.clone();
        request[field] = value;
        queue(
            &client.http,
            vec![Ok(response(200, preparation(json!([request]))?))],
        )?;
        let mut args = prepare_args();
        args.requester_authority = WalletAuthority::Secp256r1 {
            public_key: SECP256R1_KEY,
        };
        assert!(matches!(
            client.prepare_transfer(args).await,
            Err(PrepareRampTransferError::InvalidResponse(_))
        ));
    }
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        6
    );
    Ok(())
}

#[tokio::test]
async fn preparation_uses_saved_network_and_native_requester() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Devnet)?;
    queue(
        &client.http,
        vec![Ok(response(200, preparation(json!([]))?))],
    )?;
    let requester_authority = WalletAuthority::Ed25519 {
        public_key: Pubkey::new_from_array([1; 32]),
    };
    let prepared = client
        .prepare_transfer(PrepareRampTransferArgs {
            order_id: "saved/order ?#%+".into(),
            requester_authority: requester_authority.clone(),
            fee_payer: Pubkey::new_from_array([1; 32]),
        })
        .await?;
    assert!(prepared.requester_authority == requester_authority);
    assert!(prepared.prepared_transaction.network == Some(Network::Mainnet));
    let test = client.http.test.lock().map_err(|_| "lock failed")?;
    assert_eq!(test.requests.len(), 1);
    let request = test.requests.first().ok_or("missing request")?;
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.url().path(),
        "/wallet/api/ramp/orders/saved%2Forder%20%3F%23%25%2B/transfer/prepare"
    );
    assert!(request.url().query().is_none());
    assert!(request.url().fragment().is_none());
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .ok_or("missing body")?,
    )?;
    assert_eq!(
        body,
        json!({"requesterAuthority":{"ed25519":{"publicKey":Pubkey::new_from_array([1;32]).to_string()}},"feePayer":Pubkey::new_from_array([1;32]).to_string()})
    );
    Ok(())
}

#[tokio::test]
async fn preparation_checks_requester_cardinality_and_metadata() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    let request = json!({"scheme":"AUTHORITY_SIGNATURE_SCHEME_SECP256R1","signer":hex::encode(SECP256R1_KEY),"messageHash":"11".repeat(32),"slot":"7","counter":0});
    queue(
        &client.http,
        vec![Ok(response(200, preparation(json!([request.clone()]))?))],
    )?;
    client
        .prepare_transfer(PrepareRampTransferArgs {
            order_id: "saved".into(),
            requester_authority: WalletAuthority::Secp256r1 {
                public_key: SECP256R1_KEY,
            },
            fee_payer: Pubkey::new_from_array([1; 32]),
        })
        .await?;
    for requests in [json!([]), json!([request.clone(), request.clone()])] {
        queue(
            &client.http,
            vec![Ok(response(200, preparation(requests)?))],
        )?;
        assert!(matches!(
            client
                .prepare_transfer(PrepareRampTransferArgs {
                    order_id: "saved".into(),
                    requester_authority: WalletAuthority::Secp256r1 {
                        public_key: SECP256R1_KEY
                    },
                    fee_payer: Pubkey::new_from_array([1; 32]),
                })
                .await,
            Err(PrepareRampTransferError::InvalidResponse(_))
        ));
    }
    let mut wrong_network = preparation(json!([]))?;
    wrong_network["preparedTransfer"]["preparedTransaction"]["network"] = json!("NETWORK_DEVNET");
    queue(&client.http, vec![Ok(response(200, wrong_network))])?;
    assert!(matches!(
        client
            .prepare_transfer(PrepareRampTransferArgs {
                order_id: "saved".into(),
                requester_authority: WalletAuthority::Ed25519 {
                    public_key: Pubkey::new_from_array([1; 32])
                },
                fee_payer: Pubkey::new_from_array([1; 32]),
            })
            .await,
        Err(PrepareRampTransferError::InvalidResponse(_))
    ));
    Ok(())
}

#[tokio::test]
async fn preparation_requires_supported_valid_input_before_http() -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    let mut args = prepare_args();
    args.order_id = " ".into();
    assert!(matches!(
        ramp.prepare_transfer(args).await,
        Err(PrepareRampTransferError::InvalidInput(_))
    ));
    for requester_authority in [
        WalletAuthority::Secp256k1 {
            public_key: [2; 33],
        },
        WalletAuthority::ParticipantSet {
            address: Pubkey::new_from_array([5; 32]),
            role_id: None,
        },
    ] {
        let mut args = prepare_args();
        args.requester_authority = requester_authority;
        assert!(matches!(
            ramp.prepare_transfer(args).await,
            Err(PrepareRampTransferError::UnsupportedRequester)
        ));
    }
    let mut args = prepare_args();
    args.requester_authority = WalletAuthority::Secp256r1 {
        public_key: [0; 33],
    };
    assert!(matches!(
        ramp.prepare_transfer(args).await,
        Err(PrepareRampTransferError::InvalidInput(_))
    ));
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn malformed_transaction_is_not_retried() -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    let mut body = preparation(json!([]))?;
    body["preparedTransfer"]["preparedTransaction"]["transaction"] = json!("AA==");
    queue(&ramp.http, vec![Ok(response(200, body))])?;
    assert!(matches!(
        ramp.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::InvalidResponse(
            "invalid Solana transaction"
        ))
    ));
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
    Ok(())
}
#[tokio::test]
async fn preparation_without_configuration_runs_once_and_prefers_grpc_precondition(
) -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    let precondition: ::http::Response<String> = ::http::Response::builder()
        .status(503)
        .header("grpc-status", "9")
        .body(json!({"code":"UNAVAILABLE"}).to_string())?;
    let unavailable: ::http::Response<String> = ::http::Response::builder()
        .status(400)
        .header("grpc-status", "14")
        .body("{}".into())?;
    queue(
        &ramp.http,
        vec![
            Ok(response(200, preparation(json!([]))?)),
            Ok(precondition.into()),
            Ok(unavailable.into()),
            Ok(response(400, json!({}))),
            Err(HttpError::Timeout),
        ],
    )?;
    ramp.prepare_transfer(prepare_args()).await?;
    assert!(matches!(
        ramp.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::PreconditionFailed)
    ));
    assert!(matches!(
        ramp.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::ServiceUnavailable)
    ));
    assert!(matches!(
        ramp.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::UnexpectedServiceFailure)
    ));
    assert!(matches!(
        ramp.prepare_transfer(prepare_args()).await,
        Err(PrepareRampTransferError::RequestTimedOut)
    ));
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 5);
    Ok(())
}
