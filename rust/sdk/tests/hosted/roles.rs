use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    *,
};
use serde_json::json;
use solana_pubkey::Pubkey;
use std::error::Error;

fn key(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}

#[tokio::test]
async fn roles_send_the_exact_path_network_and_credentials() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.network = Network::Mainnet;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string()}),
        ))],
    )?;
    client.list_roles(&key(3)).await?;

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(
        request.url().path(),
        format!("/wallet/swig/{}/roles", key(3))
    );
    assert_eq!(request.url().query(), Some("network=NETWORK_MAINNET"));
    let authorization = request
        .headers()
        .get("authorization")
        .ok_or("missing authorization")?;
    assert_eq!(authorization, "Bearer test-only-api-key");
    assert!(authorization.is_sensitive());
    assert!(request.body().is_none());
    Ok(())
}

#[tokio::test]
async fn role_errors_keep_the_timeout_class() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.http.retry_options.max_retries = 0;
    let client = WalletClient::new(config)?;
    queue(&client.http, vec![Err(HttpError::Timeout)])?;
    assert!(matches!(
        client.list_roles(&key(3)).await,
        Err(ListRolesError::RequestTimedOut)
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "test transport")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn roles_map_resource_exhaustion_to_generic_failure() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut grpc_error = response(200, json!({}));
    grpc_error.headers_mut().insert(
        "grpc-status",
        reqwest::header::HeaderValue::from_static("8"),
    );
    queue(
        &client.http,
        vec![
            Ok(response(429, json!({}))),
            Ok(response(400, json!({"code":"RESOURCE_EXHAUSTED"}))),
            Ok(grpc_error),
        ],
    )?;
    for _ in 0..3 {
        assert!(matches!(
            client.list_roles(&key(3)).await,
            Err(ListRolesError::UnexpectedServiceFailure)
        ));
    }
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 3);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn malformed_roles_are_rejected() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"roles":[{"roleId":1,"authorityType":99,"authorityValue":key(1).to_string(),"actions":[]}]}),
        ))],
    )?;
    assert!(matches!(
        client.list_roles(&key(3)).await,
        Err(ListRolesError::InvalidResponse(
            "malformed list_roles response"
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn roles_decode_complete_hierarchy() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "swigConfigAddress": key(3).to_string(),
        "walletAddress": key(4).to_string(),
        "roles": [
            {
                "roleId": 9,
                "authorityType": 1,
                "authorityValue": key(1).to_string(),
                "actions": [
                    {"actionIndex": 2, "actionCode": 7, "actionData": {}},
                    {"actionIndex": 5, "actionCode": 1, "actionData": {"amount": "9007199254740993"}},
                ],
            },
            {
                "roleId": 3,
                "authorityType": 7,
                "authorityValue": bs58::encode([1, 2, 3]).into_string(),
                "actions": [
                    {"actionIndex": 1, "actionCode": 3, "actionData": {"program_id": key(6).to_string()}},
                    {"actionIndex": 4, "actionCode": 6, "actionData": {
                        "token_mint": key(7).to_string(), "window": "3600",
                        "limit": "18446744073709551615", "current": "9007199254740993",
                        "last_reset": "42",
                    }},
                ],
            },
        ],
    });
    let expected = SwigRoleList {
        swig_config_address: key(3),
        wallet_address: key(4),
        roles: vec![
            SwigRole {
                role_id: 9,
                authority: SwigRoleAuthority::Ed25519 { public_key: key(1) },
                actions: vec![
                    SwigRoleAction {
                        action_index: 2,
                        permission: SwigRolePermission::All,
                    },
                    SwigRoleAction {
                        action_index: 5,
                        permission: SwigRolePermission::SolLimit {
                            amount: 9_007_199_254_740_993,
                        },
                    },
                ],
            },
            SwigRole {
                role_id: 3,
                authority: SwigRoleAuthority::ProgramExec {
                    instruction_prefix: vec![1, 2, 3],
                },
                actions: vec![
                    SwigRoleAction {
                        action_index: 1,
                        permission: SwigRolePermission::Program { program_id: key(6) },
                    },
                    SwigRoleAction {
                        action_index: 4,
                        permission: SwigRolePermission::TokenRecurringLimit {
                            token_mint: key(7),
                            window: 3600,
                            limit: u64::MAX,
                            current: 9_007_199_254_740_993,
                            last_reset: 42,
                        },
                    },
                ],
            },
        ],
    };
    for body in [payload.clone(), json!({"data": payload})] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(client.list_roles(&key(3)).await? == expected);
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn roles_require_object_responses() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!([key(3).to_string(), key(4).to_string(), []]),
        json!([]),
        json!(null),
        json!(true),
        json!(42),
        json!("private-response-detail"),
    ] {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                client.list_roles(&key(3)).await,
                Err(ListRolesError::InvalidResponse(
                    "malformed list_roles response"
                ))
            ));
            let transport = client.http.test.lock().map_err(|_| "test transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn roles_preserve_omitted_roles_and_address_aliases() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!({"swigConfigAddress": key(3).to_string(), "walletAddress": key(4).to_string()}),
        json!({"swig_config_address": key(3).to_string(), "wallet_address": key(4).to_string()}),
    ] {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            let roles = client.list_roles(&key(3)).await?;
            assert_eq!(roles.swig_config_address, key(3));
            assert_eq!(roles.wallet_address, key(4));
            assert!(roles.roles.is_empty());
            let transport = client.http.test.lock().map_err(|_| "test transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn roles_reject_over_capacity_program_exec_prefixes() -> Result<(), Box<dyn Error>> {
    for authority_type in [7, 8] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({
                    "swigConfigAddress": key(3).to_string(),
                    "walletAddress": key(4).to_string(),
                    "roles": [{
                        "roleId": 1,
                        "authorityType": authority_type,
                        "authorityValue": bs58::encode([42; 41]).into_string(),
                        "actions": [],
                    }],
                }),
            ))],
        )?;
        assert!(matches!(
            client.list_roles(&key(3)).await,
            Err(ListRolesError::InvalidResponse(
                "malformed list_roles response"
            ))
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn roles_retry_eligible_failures_with_the_same_url() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Err(HttpError::RequestFailed),
            Ok(response(503, json!({"code":"UNAVAILABLE"}))),
            Ok(response(
                200,
                json!({"data":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string()}}),
            )),
        ],
    )?;
    let roles = client.list_roles(&key(3)).await?;
    assert_eq!(roles.swig_config_address, key(3));
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 3);
    let first = transport.requests.first().ok_or("missing request")?;
    assert_eq!(first.url().query(), Some("network=NETWORK_DEVNET"));
    assert!(transport
        .requests
        .iter()
        .all(|request| request.url() == first.url()));
    Ok(())
}

#[tokio::test]
async fn invalid_role_responses_fail_without_retry() -> Result<(), Box<dyn Error>> {
    for (body, message) in [
        ("not JSON".to_owned(), "malformed list_roles response"),
        (
            json!({"swigConfigAddress":key(5).to_string(),"walletAddress":key(4).to_string()})
                .to_string(),
            "wallet identity mismatch",
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(::http::Response::builder()
                .status(200)
                .body(body)?
                .into())],
        )?;
        assert!(matches!(
            client.list_roles(&key(3)).await,
            Err(ListRolesError::InvalidResponse(actual)) if actual == message
        ));
        assert_eq!(
            client
                .http
                .test
                .lock()
                .map_err(|_| "test transport")?
                .requests
                .len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn roles_not_found_keeps_only_a_static_failure() -> Result<(), Box<dyn Error>> {
    for response in [
        response(404, json!({"message":"secret-provider-detail"})),
        response(
            400,
            json!({"code":"NOT_FOUND","message":"secret-provider-detail"}),
        ),
        response(400, json!({"code":5,"message":"secret-provider-detail"})),
        response(
            400,
            json!({"code":"WALLET_NOT_FOUND","message":"secret-provider-detail"}),
        ),
        ::http::Response::builder()
            .status(200)
            .header("grpc-status", "5")
            .body("secret-provider-detail")?
            .into(),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response)])?;
        let error = client
            .list_roles(&key(3))
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(error, ListRolesError::UnexpectedServiceFailure));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
        assert_eq!(
            client
                .http
                .test
                .lock()
                .map_err(|_| "test transport")?
                .requests
                .len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn roles_unavailable_is_not_reported_as_missing_wallet() -> Result<(), Box<dyn Error>> {
    for status in [400, 404, 503] {
        let mut config = config();
        config.http.retry_options.max_retries = 0;
        let client = WalletClient::new(config)?;
        queue(
            &client.http,
            vec![Ok(response(status, json!({"code":"UNAVAILABLE"})))],
        )?;
        assert!(matches!(
            client.list_roles(&key(3)).await,
            Err(ListRolesError::ServiceUnavailable)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn roles_reject_invalid_later_entries_without_partial_results() -> Result<(), Box<dyn Error>>
{
    let valid_role = json!({
        "roleId": 1, "authorityType": 1, "authorityValue": key(1).to_string(),
        "actions": [{"actionIndex": 0, "actionCode": 7, "actionData": {}}],
    });
    let mut invalid_authority = valid_role.clone();
    invalid_authority["authorityType"] = json!(99);
    let mut invalid_key = valid_role.clone();
    invalid_key["authorityType"] = json!(3);
    invalid_key["authorityValue"] = json!(bs58::encode([4; 33]).into_string());
    let mut invalid_permission = valid_role.clone();
    invalid_permission["actions"] = json!([
        {"actionIndex": 0, "actionCode": 7, "actionData": {}},
        {"actionIndex": 1, "actionCode": 21, "actionData": {}},
    ]);
    for invalid_role in [invalid_authority, invalid_key, invalid_permission] {
        for reverse in [false, true] {
            let mut roles = vec![valid_role.clone(), invalid_role.clone()];
            if reverse {
                roles.reverse();
            }
            let payload = json!({
                "swigConfigAddress": key(3).to_string(),
                "walletAddress": key(4).to_string(),
                "roles": roles,
            });
            for body in [payload.clone(), json!({"data": payload})] {
                let client = WalletClient::new(config())?;
                queue(&client.http, vec![Ok(response(200, body))])?;
                let error = client
                    .list_roles(&key(3))
                    .await
                    .err()
                    .ok_or("expected full response rejection")?;
                assert!(matches!(
                    error,
                    ListRolesError::InvalidResponse("malformed list_roles response")
                ));
                assert_eq!(
                    error.to_string(),
                    "invalid response: malformed list_roles response"
                );
                assert!(error.source().is_none());
                let transport = client.http.test.lock().map_err(|_| "test transport")?;
                assert_eq!(transport.requests.len(), 1);
                assert!(transport.responses.is_empty());
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn roles_reject_wallet_identity_mismatch_in_both_envelopes() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "swigConfigAddress": key(5).to_string(),
        "walletAddress": key(4).to_string(),
        "roles": [{"roleId": 1, "authorityType": 1, "authorityValue": key(1).to_string()}],
    });
    for body in [payload.clone(), json!({"data": payload})] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.list_roles(&key(3)).await,
            Err(ListRolesError::InvalidResponse("wallet identity mismatch"))
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}
