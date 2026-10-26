use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use crate::{
    api::{
        get_policy::get_policy,
        types::{
            authority::WalletAuthority, permission::PolicyPermission, policy::PolicyKeySource,
        },
    },
    hosted_support::{config, queue, response},
    http::{HttpClient, HttpError},
};

#[tokio::test]
async fn policy_api_builds_the_request_and_preserves_backend_fields() -> Result<(), Box<dyn Error>>
{
    let mut config = config();
    config.http.base_url = Some("https://example.test/api".into());
    let client = HttpClient::new(config.http)?;
    let key = Pubkey::new_from_array([3; 32]);
    let destination = Pubkey::new_from_array([4; 32]);
    let actions = json!([
        {"type":"SolLimit","amount":"9007199254740993"},
        {"type":"TokenRecurringDestinationLimit","mint":key.to_string(),
         "recurringAmount":"18446744073709551615","window":"3600",
         "destination":destination.to_string()},
        {"type":"All"}
    ]);
    queue(
        &client,
        vec![Ok(response(
            200,
            json!({
                "id":"backend-policy","name":"Policy","description":"Description",
                "initialKeySource":2,"authority":{"ed25519":{"publicKey":key.to_string()}},
                "actions":actions,"guardianEnabled":false,"guardianKeySource":1,
                "guardianAuthority":{"participantSet":{"participantSetAddress":key.to_string(),"roleId":0}},
                "guardianDelaySeconds":300
            }),
        ))],
    )?;

    let policy = get_policy(&client, "policy/one?#% é").await?;
    assert_eq!(policy.id, "backend-policy");
    assert_eq!(policy.name, "Policy");
    assert_eq!(policy.description, "Description");
    assert!(matches!(
        policy.initial_key_source,
        Some(PolicyKeySource::Stored)
    ));
    assert!(matches!(policy.authority, Some(
        WalletAuthority::Ed25519 { public_key }
    ) if public_key == key));
    assert!(matches!(policy.actions.as_slice(), [
        PolicyPermission::SolLimit { amount: 9_007_199_254_740_993 },
        PolicyPermission::TokenRecurringDestinationLimit {
            mint, recurring_amount: u64::MAX, window: 3600, destination: actual_destination,
        },
        PolicyPermission::All {},
    ] if *mint == key && *actual_destination == destination));
    assert!(!policy.guardian_enabled);
    assert!(matches!(
        policy.guardian_key_source,
        Some(PolicyKeySource::AtCreation)
    ));
    assert!(matches!(policy.guardian_authority, Some(
        WalletAuthority::ParticipantSet { participant_set_address, role_id: Some(0) }
    ) if participant_set_address == key));
    assert_eq!(policy.guardian_delay_seconds, 300);

    let transport = client.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(
        request.url().path(),
        "/api/wallet/policies/policy%2Fone%3F%23%25%20%C3%A9"
    );
    assert!(request.url().query().is_none());
    assert!(request.body().is_none());
    let authorization = request
        .headers()
        .get("authorization")
        .ok_or("missing credentials")?;
    assert_eq!(authorization, "Bearer test-only-api-key");
    assert!(authorization.is_sensitive());
    Ok(())
}

#[tokio::test]
async fn policy_api_rejects_invalid_permission_collections_without_retry(
) -> Result<(), Box<dyn Error>> {
    for actions in [
        Value::Null,
        json!({"type":"All"}),
        json!([["SolLimit", "123"]]),
        json!([{"type":"All"}, ["SolLimit", "123"]]),
        json!([{"type":"All"}, {"type":"FutureAction"}]),
        json!([{"type":"All"}, {"type":"SolLimit","amount":123}]),
        json!([{"type":"All"}, {"type":"SolLimit","amount":"18446744073709551616"}]),
        json!([{"type":"All"}, {"type":"TokenLimit","mint":"invalid","amount":"1"}]),
        json!([{"type":"All"}, {"type":"SolLimit"}]),
        json!([{"type":"All"}, {"type":"All","unexpected":true}]),
    ] {
        let payload = json!({"id":"policy","name":"Policy","actions":actions});
        let client = HttpClient::new(config().http)?;
        queue(&client, vec![Ok(response(200, payload))])?;
        let error = get_policy(&client, "policy")
            .await
            .err()
            .ok_or("expected invalid policy")?;
        assert!(matches!(
            error,
            HttpError::InvalidResponse("malformed policy")
        ));
        assert_eq!(error.to_string(), "invalid response: malformed policy");
        assert!(error.source().is_none());
        let transport = client.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn policy_api_defaults_only_omitted_actions_to_empty() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!({"id":"policy","name":"Policy"}),
        json!({"id":"policy","name":"Policy","actions":[]}),
    ] {
        let client = HttpClient::new(config().http)?;
        queue(&client, vec![Ok(response(200, payload))])?;
        assert!(get_policy(&client, "policy").await?.actions.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn policy_api_preserves_key_source_presence_and_codes() -> Result<(), Box<dyn Error>> {
    for (source, expected) in [
        (None, None),
        (Some(Value::Null), None),
        (Some(json!(0)), Some(0)),
        (Some(json!(1)), Some(1)),
        (Some(json!(2)), Some(2)),
        (Some(json!("WALLET_POLICY_KEY_SOURCE_UNSPECIFIED")), Some(0)),
        (Some(json!("WALLET_POLICY_KEY_SOURCE_AT_CREATION")), Some(1)),
        (Some(json!("WALLET_POLICY_KEY_SOURCE_STORED")), Some(2)),
    ] {
        let client = HttpClient::new(config().http)?;
        let mut payload = json!({"id":"policy","name":"Policy"});
        if let Some(source) = source {
            let fields = payload.as_object_mut().ok_or("policy must be an object")?;
            fields.insert("initial_key_source".into(), source.clone());
            fields.insert("guardian_key_source".into(), source);
        }
        queue(&client, vec![Ok(response(200, payload))])?;
        let policy = get_policy(&client, "policy").await?;
        for actual in [policy.initial_key_source, policy.guardian_key_source] {
            assert!(matches!(
                (actual, expected),
                (None, None)
                    | (Some(PolicyKeySource::Unspecified), Some(0))
                    | (Some(PolicyKeySource::AtCreation), Some(1))
                    | (Some(PolicyKeySource::Stored), Some(2))
            ));
        }
        assert_eq!(policy.description, "");
        assert!(policy.actions.is_empty());
        assert!(policy.authority.is_none());
        assert!(policy.guardian_authority.is_none());
    }
    Ok(())
}

#[tokio::test]
async fn policy_api_preserves_authorities_before_sdk_validation() -> Result<(), Box<dyn Error>> {
    for (authority, case) in [
        (
            json!({"program_exec_proof":{"role_id":4,"zk_proof":"AQID"}}),
            1,
        ),
        (json!({"secp256r1":{"publicKey":"not-a-curve-point"}}), 2),
    ] {
        let client = HttpClient::new(config().http)?;
        queue(
            &client,
            vec![Ok(response(
                200,
                json!({
                    "id":"policy","name":"Policy","authority":authority,
                    "guardian_authority":authority,"guardian_enabled":false,
                    "guardian_delay_seconds":42
                }),
            ))],
        )?;
        let policy = get_policy(&client, "policy").await?;
        assert!(!policy.guardian_enabled);
        assert_eq!(policy.guardian_delay_seconds, 42);
        for authority in [policy.authority, policy.guardian_authority] {
            match case {
                1 => assert!(matches!(authority, Some(
                    WalletAuthority::ProgramExecProof { role_id: 4, zk_proof }
                ) if zk_proof == "AQID")),
                _ => assert!(matches!(authority, Some(
                    WalletAuthority::Secp256r1 { public_key }
                ) if public_key == "not-a-curve-point")),
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn policy_api_rejects_malformed_shapes_without_retry() -> Result<(), Box<dyn Error>> {
    let key = Pubkey::new_from_array([4; 32]).to_string();
    for payload in [
        json!(["policy", "Policy"]),
        json!({"id":"policy"}),
        json!({"id":"policy","name":"Policy","authority":{}}),
        json!({"id":"policy","name":"Policy","guardianAuthority":{}}),
        json!({"id":"policy","name":"Policy","guardian_authority":{}}),
        json!({"id":"policy","name":"Policy","actions":[[]]}),
        json!({"id":"policy","name":"Policy","actions":null}),
        json!({"id":"policy","name":"Policy","authority":["Ed25519",key]}),
        json!({"id":"policy","name":"Policy","authority":{"ed25519":[key]}}),
        json!({"id":"policy","name":"Policy","guardian_authority":{"program_exec_proof":[1,"AQID"]}}),
        json!({"id":"policy","name":"Policy","authority":{"ed25519":{"publicKey":"invalid"}}}),
        json!({"id":"policy","name":"Policy","initialKeySource":3}),
        json!({"id":"policy","name":"Policy","guardianKeySource":"UNKNOWN"}),
        json!({"id":"policy","name":"Policy","guardianDelaySeconds":-1}),
        json!({"id":"policy","name":"Policy","guardianEnabled":true,"guardian_enabled":false}),
    ] {
        let client = HttpClient::new(config().http)?;
        queue(&client, vec![Ok(response(200, payload))])?;
        let error = get_policy(&client, "policy")
            .await
            .err()
            .ok_or("expected invalid response")?;
        assert!(matches!(
            error,
            HttpError::InvalidResponse("malformed policy")
        ));
        assert_eq!(error.to_string(), "invalid response: malformed policy");
        assert!(error.source().is_none());
        assert_eq!(
            client
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
async fn policy_api_rejects_legacy_authority_templates_without_fallback(
) -> Result<(), Box<dyn Error>> {
    let public_key = Pubkey::new_from_array([8; 32]).to_string();
    for kind in [
        "Ed25519",
        "Secp256k1",
        "Secp256r1",
        "Ed25519Session",
        "Secp256k1Session",
        "Secp256r1Session",
    ] {
        let mut authority = json!({"type":kind,"publicKey":public_key});
        if kind.ends_with("Session") {
            authority["maxDurationSlots"] = json!("42");
        }
        for field in ["authority", "guardianAuthority", "guardian_authority"] {
            let mut payload = json!({"id":"policy","name":"Policy"});
            payload[field] = authority.clone();
            let client = HttpClient::new(config().http)?;
            queue(&client, vec![Ok(response(200, payload))])?;
            assert!(matches!(
                get_policy(&client, "policy").await,
                Err(HttpError::InvalidResponse("malformed policy"))
            ));
            assert_eq!(
                client
                    .test
                    .lock()
                    .map_err(|_| "test transport")?
                    .requests
                    .len(),
                1
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn policy_api_accepts_direct_objects_without_selecting_nested_data(
) -> Result<(), Box<dyn Error>> {
    let valid = json!({"id":"policy","name":"Policy"});
    for payload in [
        valid.clone(),
        json!({"id":"policy","name":"Policy","authority":null,"guardianAuthority":null}),
        json!({"id":"policy","name":"Policy","data":{"id":"ignored","name":"Ignored","authority":{}}}),
        json!({"id":"policy","name":"Policy","data":null}),
        json!({"id":"policy","name":"Policy","data":[valid]}),
    ] {
        let client = HttpClient::new(config().http)?;
        queue(&client, vec![Ok(response(200, payload))])?;
        let policy = get_policy(&client, "policy").await?;
        assert_eq!(policy.id, "policy");
        assert_eq!(policy.name, "Policy");
        assert!(policy.authority.is_none());
        assert!(policy.guardian_authority.is_none());
    }
    Ok(())
}

#[tokio::test]
async fn policy_api_rejects_data_wrappers_without_retry() -> Result<(), Box<dyn Error>> {
    let valid = json!({"id":"policy","name":"Policy"});
    for body in [
        "not JSON".to_owned(),
        "null".to_owned(),
        json!({"data":valid}).to_string(),
        json!({"data":null}).to_string(),
        json!({"data":[valid]}).to_string(),
        json!({"data":{"data":valid}}).to_string(),
    ] {
        let client = HttpClient::new(config().http)?;
        queue(
            &client,
            vec![Ok(::http::Response::builder()
                .status(200)
                .body(body)?
                .into())],
        )?;
        let error = get_policy(&client, "policy")
            .await
            .err()
            .ok_or("expected invalid response")?;
        assert!(matches!(
            error,
            HttpError::InvalidResponse("malformed policy")
        ));
        assert_eq!(error.to_string(), "invalid response: malformed policy");
        assert!(error.source().is_none());
        assert_eq!(
            client
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
async fn policy_api_preserves_http_error_classes() -> Result<(), Box<dyn Error>> {
    let client = HttpClient::new(config().http)?;
    queue(
        &client,
        vec![Ok(response(404, json!({"code":"NOT_FOUND"})))],
    )?;
    assert!(matches!(
        get_policy(&client, "policy").await,
        Err(HttpError::NotFound)
    ));
    assert_eq!(
        client
            .test
            .lock()
            .map_err(|_| "test transport")?
            .requests
            .len(),
        1
    );
    Ok(())
}
