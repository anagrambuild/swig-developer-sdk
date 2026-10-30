use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    *,
};
use serde_json::json;
use solana_pubkey::Pubkey;
use std::error::Error;

#[tokio::test]
async fn policy_sends_its_exact_path_and_credentials() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.network = Network::Mainnet;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"id":"policy/one","name":"Policy"}),
        ))],
    )?;
    client.get_policy("policy/one").await?;

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(request.url().path(), "/wallet/policies/policy%2Fone");
    assert_eq!(request.url().query(), None);
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
async fn policy_id_stays_in_one_path_segment_under_the_base_path() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.http.base_url = Some("https://example.test/api".into());
    let client = WalletClient::new(config)?;
    let policy_id = "policy/one?#% é";
    queue(
        &client.http,
        vec![Ok(response(200, json!({"id":policy_id,"name":"Policy"})))],
    )?;
    client.get_policy(policy_id).await?;

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    let request = transport.requests.first().ok_or("missing request")?;
    assert_eq!(
        request.url().path(),
        "/api/wallet/policies/policy%2Fone%3F%23%25%20%C3%A9"
    );
    assert_eq!(request.url().query(), None);
    assert_eq!(request.url().fragment(), None);
    Ok(())
}

#[tokio::test]
async fn policy_maps_transport_failures_without_exposing_http_types() -> Result<(), Box<dyn Error>>
{
    let mut config = config();
    config.http.retry_options.max_retries = 0;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![
            Err(HttpError::Timeout),
            Err(HttpError::RequestFailed),
            Err(HttpError::InvalidUrl),
            Err(HttpError::ResponseTooLarge),
            Ok(response(400, json!({"code":3}))),
            Ok(response(503, json!({"code":14}))),
        ],
    )?;
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::RequestTimedOut)
    ));
    for _ in 0..2 {
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::RequestFailed)
        ));
    }
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::InvalidResponse(
            "response exceeds size limit"
        ))
    ));
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::InvalidInput("service rejected the request"))
    ));
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::ServiceUnavailable)
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "test transport")?
            .requests
            .len(),
        6
    );
    Ok(())
}

#[tokio::test]
async fn malformed_policy_payloads_fail_without_retry() -> Result<(), Box<dyn Error>> {
    for body in ["not JSON", "null", r#"{"data":null}"#, r#"{"id":"policy"}"#] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(::http::Response::builder()
                .status(200)
                .body(body)?
                .into())],
        )?;
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::InvalidResponse("malformed policy"))
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
async fn policy_rejects_data_wrappers_without_retry() -> Result<(), Box<dyn Error>> {
    let valid = json!({"id":"policy","name":"Policy"});
    for payload in [
        json!({"data":valid}),
        json!({"data":{"data":valid}}),
        json!({"data":[valid]}),
        json!({"data":null}),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        let error = client
            .get_policy("policy")
            .await
            .err()
            .ok_or("expected invalid policy")?;
        assert!(matches!(
            error,
            GetPolicyError::InvalidResponse("malformed policy")
        ));
        assert_eq!(error.to_string(), "invalid response: malformed policy");
        assert!(error.source().is_none());
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn policy_ignores_unrelated_data_on_a_direct_response() -> Result<(), Box<dyn Error>> {
    for data in [
        json!({"id":"different-policy","name":"Ignored","actions":[{"type":"FutureAction"}]}),
        json!(null),
        json!([]),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({
                    "id":"policy","name":"Policy","actions":[{"type":"All"}],"data":data
                }),
            ))],
        )?;
        let policy = client.get_policy("policy").await?;
        assert_eq!(policy.id, "policy");
        assert_eq!(policy.name, "Policy");
        assert!(matches!(policy.actions.as_slice(), [WalletPermission::All]));
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
async fn policy_rejects_positional_response_and_nested_records() -> Result<(), Box<dyn Error>> {
    let key = Pubkey::new_from_array([7; 32]).to_string();
    let cases = [
        json!(["policy", "Policy", "", null, [], false, null, null, null, 0]),
        json!({"id":"policy","name":"Policy","authority":["Ed25519",key]}),
        json!({"id":"policy","name":"Policy","authority":{"ed25519":[key]}}),
        json!({"id":"policy","name":"Policy","guardianEnabled":true,"guardianAuthority":["Ed25519Session",key,"10"]}),
        json!({"id":"policy","name":"Policy","guardianEnabled":true,"guardian_authority":["Ed25519",key]}),
        json!({"id":"policy","name":"Policy","actions":[["SolLimit","100"]]}),
        json!({"id":"policy","name":"Policy","actions":[{"type":"All"},["SolLimit","100"]]}),
    ];
    for payload in cases {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::InvalidResponse("malformed policy"))
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn policy_honors_grpc_failure_even_with_http_success() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let response = ::http::Response::builder()
        .status(200)
        .header("grpc-status", "5")
        .body(r#"{"id":"policy","name":"Policy"}"#)?
        .into();
    queue(&client.http, vec![Ok(response)])?;
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::PolicyNotFound { policy_id }) if policy_id == "policy"
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
async fn invalid_policy_ids_fail_without_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![])?;
    for policy in ["", " ", "\n\t"] {
        assert!(matches!(
            client.get_policy(policy).await,
            Err(GetPolicyError::InvalidInput("policy ID must not be empty"))
        ));
    }
    for policy in [".", ".."] {
        assert!(matches!(
            client.get_policy(policy).await,
            Err(GetPolicyError::InvalidInput(
                "policy ID must not be a dot segment"
            ))
        ));
    }
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "test transport")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn policy_errors_keep_only_static_classes() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.http.retry_options.max_retries = 0;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![Ok(response(401, json!({"message":"secret-api-key"})))],
    )?;
    let error = client
        .get_policy("policy")
        .await
        .err()
        .ok_or("expected policy error")?;
    assert!(matches!(error, GetPolicyError::AuthenticationFailed));
    assert!(!error.to_string().contains("secret"));
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
async fn wallet_get_does_not_retry_a_broken_redirect_body() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let stream = futures_util::stream::iter(vec![Err::<Vec<u8>, _>(std::io::Error::other(
        "broken body",
    ))]);
    let response: reqwest::Response = ::http::Response::builder()
        .status(302)
        .body(reqwest::Body::wrap_stream(stream))?
        .into();
    queue(&client.http, vec![Ok(response)])?;
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::RequestFailed)
    ));
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn policy_retries_only_eligible_failures_and_preserves_policy_authority(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let public_key = Pubkey::new_from_array([1; 32]);
    queue(
        &client.http,
        vec![
            Err(HttpError::RequestFailed),
            Ok(response(503, json!({"code":"UNAVAILABLE"}))),
            Ok(response(
                200,
                json!({"id":"policy/one","name":"My policy","authority":{"ed25519":{"publicKey":public_key.to_string()}},"initialKeySource":2,"actions":[{"type":"All"}]}),
            )),
        ],
    )?;
    let policy = client.get_policy("policy/one").await?;
    assert!(
        matches!(policy.authority,Some(WalletAuthority::Ed25519 { public_key: key }) if key==public_key)
    );
    assert_eq!(policy.description, "");
    assert!(matches!(
        policy.actions.first(),
        Some(WalletPermission::All)
    ));
    let transport = client
        .http
        .test
        .lock()
        .map_err(|_| "poisoned test transport")?;
    assert_eq!(transport.requests.len(), 3);
    assert!(transport
        .requests
        .first()
        .ok_or("missing request")?
        .url()
        .path()
        .ends_with("policy%2Fone"));
    Ok(())
}

#[tokio::test]
async fn inconsistent_policy_authorities_fail_without_retry() -> Result<(), Box<dyn Error>> {
    let authority = json!({"ed25519": {
        "publicKey": Pubkey::new_from_array([1; 32]).to_string(),
    }});
    for (payload, message) in [
        (
            json!({"id":"policy","name":"Policy","initialKeySource":2}),
            "inconsistent initial policy authority",
        ),
        (
            json!({"id":"policy","name":"Policy","initialKeySource":1,"authority":authority}),
            "inconsistent initial policy authority",
        ),
        (
            json!({"id":"policy","name":"Policy","guardianEnabled":false,"guardianAuthority":authority}),
            "disabled guardian has authority configuration",
        ),
        (
            json!({"id":"policy","name":"Policy","guardianEnabled":false,"guardianKeySource":2}),
            "disabled guardian has authority configuration",
        ),
        (
            json!({"id":"policy","name":"Policy","guardianEnabled":false,"guardianKeySource":1}),
            "disabled guardian has authority configuration",
        ),
        (
            json!({"id":"policy","name":"Policy","guardianEnabled":true,"guardianKeySource":2}),
            "inconsistent guardian policy authority",
        ),
        (
            json!({"id":"policy","name":"Policy","guardianEnabled":true,"guardianKeySource":1,"guardianAuthority":authority}),
            "inconsistent guardian policy authority",
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::InvalidResponse(actual)) if actual == message
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
async fn policy_construction_preserves_stored_authorities_and_fields() -> Result<(), Box<dyn Error>>
{
    let client = WalletClient::new(config())?;
    let initial_key = Pubkey::new_from_array([1; 32]);
    let guardian_key = Pubkey::new_from_array([2; 32]);
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({
                "id": "policy",
                "name": "My policy",
                "description": "Policy description",
                "initialKeySource": 2,
                "authority": {"ed25519":{"publicKey":initial_key.to_string()}},
                "actions": [{"type":"All"}],
                "guardianEnabled": true,
                "guardianKeySource": 2,
                "guardianAuthority": {"ed25519": {
                    "publicKey": guardian_key.to_string(),
                }},
                "guardianDelaySeconds": 300,
            }),
        ))],
    )?;
    let policy = client.get_policy("policy").await?;
    assert_eq!(policy.id, "policy");
    assert_eq!(policy.name, "My policy");
    assert_eq!(policy.description, "Policy description");
    assert!(matches!(
        policy.authority,
        Some(WalletAuthority::Ed25519 { public_key })
            if public_key == initial_key
    ));
    assert!(matches!(policy.actions.as_slice(), [WalletPermission::All]));
    assert!(policy.guardian_enabled);
    assert!(matches!(
        policy.guardian_authority,
        Some(WalletAuthority::Ed25519 { public_key })
            if public_key == guardian_key
    ));
    assert_eq!(policy.guardian_delay_seconds, 300);
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
async fn get_does_not_retry_unclassified_client_errors_or_malformed_success(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Ok(response(403, json!({"code":"PERMISSION_DENIED"}))),
            Ok(response(429, json!({"code":"RESOURCE_EXHAUSTED"}))),
            Ok(response(
                200,
                json!({"id":"different-policy","name":"Policy"}),
            )),
        ],
    )?;
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::PermissionDenied)
    ));
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::UnexpectedServiceFailure)
    ));
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::InvalidResponse("policy identity mismatch"))
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "poisoned test transport")?
            .requests
            .len(),
        3
    );
    Ok(())
}

#[tokio::test]
async fn policy_not_found_requires_a_structured_backend_code() -> Result<(), Box<dyn Error>> {
    for response in [
        response(400, json!({"code":"NOT_FOUND"})),
        response(400, json!({"code":5})),
        response(400, json!({"code":"5"})),
        response(404, json!({"code":"NOT_FOUND"})),
        ::http::Response::builder()
            .status(404)
            .header("grpc-status", "5")
            .body(String::new())?
            .into(),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response)])?;
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::PolicyNotFound { policy_id }) if policy_id == "policy"
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
async fn policy_does_not_infer_missing_resources_from_bare_http_404() -> Result<(), Box<dyn Error>>
{
    for body in ["", "not JSON", r#"{"message":"policy not found"}"#] {
        let client = WalletClient::new(config())?;
        let response = ::http::Response::builder().status(404).body(body)?.into();
        queue(&client.http, vec![Ok(response)])?;
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::UnexpectedServiceFailure)
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
async fn policy_permission_denied_preserves_the_typed_error_without_retry(
) -> Result<(), Box<dyn Error>> {
    for response in [
        response(403, json!({})),
        response(400, json!({"code":7})),
        response(400, json!({"code":"7"})),
        response(
            400,
            json!({"code":"PERMISSION_DENIED", "message":"private upstream detail"}),
        ),
        ::http::Response::builder()
            .status(200)
            .header("grpc-status", "7")
            .body("private upstream detail")?
            .into(),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response)])?;
        let error = match client.get_policy("policy").await {
            Ok(_) => return Err("permission denial returned a policy".into()),
            Err(error) => error,
        };
        assert!(matches!(error, GetPolicyError::PermissionDenied));
        assert_eq!(error.to_string(), "permission denied");
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
async fn unsupported_backend_codes_use_the_generic_failure() -> Result<(), Box<dyn Error>> {
    for code in [
        "AUTHENTICATION_FAILED",
        "INVALID_INPUT",
        "RATE_LIMITED",
        "SERVICE_UNAVAILABLE",
        "WALLET_NOT_FOUND",
        "POLICY_NOT_FOUND",
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(400, json!({"code":code})))])?;
        assert!(matches!(
            client.get_policy("policy").await,
            Err(GetPolicyError::UnexpectedServiceFailure)
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
async fn policy_keeps_guardian_configuration_without_recovery_work() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({
                "id":"policy","name":"Policy","initialKeySource":"WALLET_POLICY_KEY_SOURCE_AT_CREATION",
                "guardianEnabled":true,"guardianKeySource":"WALLET_POLICY_KEY_SOURCE_AT_CREATION",
                "guardianDelaySeconds":300,
            }),
        ))],
    )?;
    let policy = client.get_policy("policy").await?;
    assert!(policy.authority.is_none());
    assert!(policy.guardian_enabled);
    assert!(policy.guardian_authority.is_none());
    assert_eq!(policy.guardian_delay_seconds, 300);
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "poisoned test transport")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn policy_permission_conversion_preserves_all_actions_in_order() -> Result<(), Box<dyn Error>>
{
    let mint = Pubkey::new_from_array([8; 32]);
    let destination = Pubkey::new_from_array([9; 32]);
    let actions = vec![
        json!({"type":"SolLimit","amount":"9007199254740993"}),
        json!({"type":"TokenRecurringDestinationLimit","mint":mint.to_string(),
               "recurringAmount":"18446744073709551615","window":"3600",
               "destination":destination.to_string()}),
        json!({"type":"All"}),
    ];
    let expected = vec![
        WalletPermission::SolLimit {
            amount: 9_007_199_254_740_993,
        },
        WalletPermission::TokenRecurringDestinationLimit {
            mint,
            recurring_amount: u64::MAX,
            window: 3600,
            destination,
        },
        WalletPermission::All,
    ];
    for reverse in [false, true] {
        let mut actions = actions.clone();
        let mut expected = expected.clone();
        if reverse {
            actions.reverse();
            expected.reverse();
        }
        let payload = json!({"id":"policy","name":"Policy","actions":actions});
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(client.get_policy("policy").await?.actions == expected);
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
async fn policy_defaults_omitted_actions_but_rejects_null() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!({"id":"policy","name":"Policy"}),
        json!({"id":"policy","name":"Policy","actions":[]}),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(client.get_policy("policy").await?.actions.is_empty());
    }
    let payload = json!({"id":"policy","name":"Policy","actions":null});
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, payload))])?;
    assert!(matches!(
        client.get_policy("policy").await,
        Err(GetPolicyError::InvalidResponse("malformed policy"))
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
async fn policy_rejects_invalid_authorities_and_permissions() -> Result<(), Box<dyn Error>> {
    for (payload, expected) in [
        (
            json!({"id":"policy","name":"Policy","authority":{}}),
            "malformed policy",
        ),
        (
            json!({"id":"policy","name":"Policy","authority":{"programExecProof":{"roleId":4,"zkProof":"AQID"}}}),
            "unsupported wallet authority",
        ),
        (
            json!({"id":"policy","name":"Policy","actions":[{"type":"FutureAction","amount":"1"}]}),
            "malformed policy",
        ),
        (
            json!({"id":"policy","name":"Policy","actions":[{"type":"All"},{"type":"SolLimit","amount":1}]}),
            "malformed policy",
        ),
        (
            json!({"id":"policy","name":"Policy","actions":[{"type":"All"},{"type":"FutureAction","amount":"1"}]}),
            "malformed policy",
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        let error = client
            .get_policy("policy")
            .await
            .err()
            .ok_or("expected invalid policy")?;
        assert!(matches!(error, GetPolicyError::InvalidResponse(actual) if actual == expected));
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
