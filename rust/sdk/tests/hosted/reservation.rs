use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn reservation_and_creation_share_inputs() -> Result<(), Box<dyn Error>> {
    for options in [
        None,
        Some(ReservationAddressOptions::AccountIndex(0)),
        Some(ReservationAddressOptions::AccountIndex(u32::MAX)),
        Some(ReservationAddressOptions::Salt([0; 32])),
    ] {
        for wrapped in [false, true] {
            let client = WalletClient::new(config())?;
            let mut transaction = prepared()?;
            transaction["kind"] = if wrapped {
                json!(1)
            } else {
                json!("PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET")
            };
            let payload = json!({"wallet": transaction["wallet"], "network": "NETWORK_DEVNET", "transactions": [transaction]});
            let payload = if wrapped {
                json!({"data": payload})
            } else {
                payload
            };
            queue(
                &client.http,
                vec![
                    Ok(response(200, payload.clone())),
                    Ok(response(200, payload)),
                ],
            )?;
            let reserved = client
                .reserve_address(ReserveWalletAddressArgs {
                    root_authority: authority(),
                    address_options: options,
                })
                .await?;
            let created = client
                .prepare_create_reserved_wallet(PrepareCreateReservedWalletArgs {
                    root_authority: authority(),
                    address_options: options,
                    fee_payer: key(1),
                })
                .await?;
            assert_eq!(
                created.wallet.swig_config_address,
                reserved.wallet.swig_config_address
            );
            assert_eq!(
                created.wallet.wallet_address,
                reserved.wallet.wallet_address
            );
            let transport = client.http.test.lock().map_err(|_| "transport lock")?;
            assert_eq!(transport.requests.len(), 2);
            let mut bodies = Vec::new();
            for request in &transport.requests {
                let body: Value = serde_json::from_slice(
                    request
                        .body()
                        .and_then(|b| b.as_bytes())
                        .ok_or("missing body")?,
                )?;
                assert_eq!(body["network"], "NETWORK_DEVNET");
                assert_eq!(
                    body["rootAuthority"],
                    json!({"ed25519": {"publicKey": key(1).to_string()}})
                );
                bodies.push(body);
            }
            assert_eq!(
                transport.requests[0].url().path(),
                "/transaction/api/wallet/reserve"
            );
            assert_eq!(
                transport.requests[1].url().path(),
                "/transaction/api/wallet/create-reserved"
            );
            assert_eq!(bodies[0]["addressOptions"], bodies[1]["addressOptions"]);
            match options {
                None => assert!(bodies[0].get("addressOptions").is_none()),
                Some(ReservationAddressOptions::AccountIndex(index)) => {
                    assert_eq!(bodies[0]["addressOptions"], json!({"accountIndex": index}))
                }
                Some(ReservationAddressOptions::Salt(_)) => assert_eq!(
                    bodies[0]["addressOptions"],
                    json!({"salt": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="})
                ),
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn reservation_rejects_unsupported_authority_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let root = WalletAuthority::ParticipantSet {
        address: key(7),
        role_id: None,
    };
    assert!(matches!(
        client
            .reserve_address(ReserveWalletAddressArgs {
                root_authority: root.clone(),
                address_options: None
            })
            .await,
        Err(WalletReservationError::InvalidInput(_))
    ));
    assert!(matches!(
        client
            .prepare_create_reserved_wallet(PrepareCreateReservedWalletArgs {
                root_authority: root,
                address_options: None,
                fee_payer: key(1)
            })
            .await,
        Err(WalletReservationError::InvalidInput(_))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "transport lock")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn reservation_rejects_invalid_responses() -> Result<(), Box<dyn Error>> {
    let mut transaction = prepared()?;
    transaction["kind"] = json!(1);
    for payload in [
        json!(null),
        json!({}),
        json!({"wallet": {}, "network": "NETWORK_DEVNET"}),
        json!({"wallet": transaction["wallet"], "network": "NETWORK_MAINNET"}),
        json!([transaction["wallet"], "NETWORK_DEVNET"]),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(matches!(
            client
                .reserve_address(ReserveWalletAddressArgs {
                    root_authority: authority(),
                    address_options: None
                })
                .await,
            Err(WalletReservationError::InvalidResponse(_))
        ));
    }
    let mut wrong_wallet = transaction.clone();
    wrong_wallet["wallet"]["walletAddress"] = json!(key(8).to_string());
    for transactions in [
        json!([]),
        json!([transaction, transaction]),
        json!([wrong_wallet]),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({"wallet": transaction["wallet"], "network": "NETWORK_DEVNET", "transactions": transactions}),
            ))],
        )?;
        assert!(matches!(
            client
                .prepare_create_reserved_wallet(PrepareCreateReservedWalletArgs {
                    root_authority: authority(),
                    address_options: None,
                    fee_payer: key(1)
                })
                .await,
            Err(WalletReservationError::InvalidResponse(_))
        ));
    }
    Ok(())
}

#[tokio::test]
async fn reservation_preserves_transport_errors_without_retrying_post() -> Result<(), Box<dyn Error>>
{
    for (failure, expected) in [
        (HttpError::Timeout, WalletReservationError::RequestTimedOut),
        (
            HttpError::Unauthenticated,
            WalletReservationError::AuthenticationFailed,
        ),
        (
            HttpError::Unavailable,
            WalletReservationError::ServiceUnavailable,
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Err(failure)])?;
        let result = client
            .reserve_address(ReserveWalletAddressArgs {
                root_authority: authority(),
                address_options: None,
            })
            .await;
        assert_eq!(
            result.err().ok_or("expected failure")?.to_string(),
            expected.to_string()
        );
        assert_eq!(
            client
                .http
                .test
                .lock()
                .map_err(|_| "transport lock")?
                .requests
                .len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn creation_requires_the_wallet_creation_kind() -> Result<(), Box<dyn Error>> {
    for kind in [
        None,
        Some(json!(0)),
        Some(json!(2)),
        Some(json!("PREPARED_TRANSACTION_KIND_TRANSFER_SOL")),
    ] {
        let client = WalletClient::new(config())?;
        let mut transaction = prepared()?;
        let fields = transaction.as_object_mut().ok_or("missing transaction")?;
        fields.remove("kind");
        if let Some(kind) = kind {
            fields.insert("kind".into(), kind);
        }
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({"wallet": transaction["wallet"], "network": "NETWORK_DEVNET", "transactions": [transaction]}),
            ))],
        )?;
        assert!(matches!(
            client
                .prepare_create_reserved_wallet(PrepareCreateReservedWalletArgs {
                    root_authority: authority(),
                    address_options: None,
                    fee_payer: key(1)
                })
                .await,
            Err(WalletReservationError::InvalidResponse(
                "expected a reserved wallet creation transaction"
            ))
        ));
    }
    Ok(())
}
