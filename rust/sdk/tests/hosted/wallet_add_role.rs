use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::wire::tests::transaction::positional_prepared_responses,
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn add_role_rejects_positional_objects() -> Result<(), Box<dyn Error>> {
    let prepared = prepared()?;
    let mut payloads = vec![json!([prepared])];
    payloads.extend(
        positional_prepared_responses(prepared)
            .into_iter()
            .map(|value| json!({"transaction": value})),
    );
    for payload in payloads {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                client.prepare_add_role(role()).await,
                Err(PrepareAddRoleError::InvalidResponse(
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

fn role() -> PrepareAddRoleArgs {
    PrepareAddRoleArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        authority: WalletAuthority::Ed25519 { public_key: key(8) },
        actions: vec![WalletPermission::All],
        requester_authority: authority(),
    }
}

#[tokio::test]
async fn add_role_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transaction":prepared()?})))],
    )?;
    client.prepare_add_role(role()).await?;
    let context = json!({
        "network":"NETWORK_DEVNET", "swigAddress":key(3).to_string(), "feePayer":key(1).to_string(),
        "requesterAuthority":{"ed25519":{"publicKey":key(1).to_string()}},
    });
    let mut expected =
        json!({"authority":{"ed25519":{"publicKey":key(8).to_string()}},"actions":[{"all":{}}]});
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
    assert_eq!(request.url().path(), "/transaction/wallet/role/add");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    assert_eq!(
        body, expected,
        "wrong payload for /transaction/wallet/role/add"
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
            .prepare_add_role(role())
            .await
            .err()
            .ok_or("expected add_role error")?;
        assert!(matches!(
            (&error, transport_failure),
            (PrepareAddRoleError::RequestTimedOut, true)
                | (PrepareAddRoleError::ServiceUnavailable, false)
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
            .prepare_add_role(role())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            PrepareAddRoleError::UnexpectedServiceFailure
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
async fn empty_actions_fail_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut args = role();
    args.actions.clear();
    assert!(matches!(
        client.prepare_add_role(args).await,
        Err(PrepareAddRoleError::InvalidInput(
            "actions must not be empty"
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
async fn authority_capabilities_are_checked_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let secp256k1 = WalletAuthority::Secp256k1 {
        public_key: [0; 33],
    };
    let participant = WalletAuthority::ParticipantSet {
        address: key(7),
        role_id: Some(2),
    };
    for authority in [secp256k1, participant.clone()] {
        let mut args = role();
        args.requester_authority = authority;
        assert!(matches!(
            client.prepare_add_role(args).await,
            Err(PrepareAddRoleError::InvalidInput(
                "unsupported requester authority"
            ))
        ));
    }
    let mut args = role();
    args.authority = participant;
    assert!(matches!(
        client.prepare_add_role(args).await,
        Err(PrepareAddRoleError::InvalidInput(
            "new participant-set authority must omit role ID"
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
async fn add_role_returns_native_preparation() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transaction":prepared()?})))],
    )?;
    client
        .prepare_add_role(PrepareAddRoleArgs {
            swig_config_address: key(3),
            fee_payer: key(1),
            authority: WalletAuthority::ParticipantSet {
                address: key(7),
                role_id: None,
            },
            actions: vec![WalletPermission::SolLimit { amount: u64::MAX }],
            requester_authority: authority(),
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
    assert_eq!(
        body["actions"],
        json!([{"solLimit":{"amount":u64::MAX.to_string()}}])
    );
    Ok(())
}
