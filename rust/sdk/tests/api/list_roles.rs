use std::error::Error;

use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    api::{list_roles::list_roles, types::role_permission::RolePermission},
    hosted_support::{config, queue, response},
    http::{HttpClient, HttpError},
    Network,
};

#[tokio::test]
async fn roles_api_sends_the_exact_request() -> Result<(), Box<dyn Error>> {
    let address = Pubkey::new_from_array([1; 32]);
    let wallet = Pubkey::new_from_array([2; 32]);
    for (network, query) in [
        (Network::Devnet, "network=NETWORK_DEVNET"),
        (Network::Mainnet, "network=NETWORK_MAINNET"),
    ] {
        let mut options = config().http;
        options.base_url = Some("https://example.test/api".into());
        let client = HttpClient::new(options)?;
        queue(
            &client,
            vec![Ok(response(
                200,
                json!({
                    "swigConfigAddress": address.to_string(),
                    "walletAddress": wallet.to_string(),
                }),
            ))],
        )?;
        let result = list_roles(&client, &address, network).await?;
        assert_eq!(result.swig_config_address, address);
        assert_eq!(result.wallet_address, wallet);
        assert!(result.roles.is_empty());

        let transport = client.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        let request = transport.requests.first().ok_or("missing request")?;
        assert_eq!(request.method(), reqwest::Method::GET);
        assert_eq!(
            request.url().path(),
            format!("/api/wallet/swig/{address}/roles")
        );
        assert_eq!(request.url().query(), Some(query));
        assert!(request.body().is_none());
        let authorization = request
            .headers()
            .get("authorization")
            .ok_or("missing credentials")?;
        assert_eq!(authorization, "Bearer test-only-api-key");
        assert!(authorization.is_sensitive());
    }
    Ok(())
}

#[tokio::test]
async fn roles_api_preserves_all_role_and_action_state() -> Result<(), Box<dyn Error>> {
    let requested = Pubkey::new_from_array([1; 32]);
    let returned = Pubkey::new_from_array([2; 32]);
    let wallet = Pubkey::new_from_array([3; 32]);
    let program = Pubkey::new_from_array([4; 32]);
    let first_role = json!({
        "roleId": 9, "authorityType": 1, "authorityValue": requested.to_string(),
        "actions": [
            {"actionIndex": 2, "actionCode": 7, "actionData": {}},
            {"actionIndex": 5, "actionCode": 2, "actionData": {
                "recurring_amount": "9007199254740993", "window": "3600",
                "last_reset": "42", "current_amount": "18446744073709551615",
            }},
        ],
    });
    let second_role = json!({
        "roleId": 3, "authorityType": 99, "authorityValue": "backend-authority",
        "actions": [{"actionIndex": 1, "actionCode": 3, "actionData": {
            "program_id": program.to_string(),
        }}],
    });
    for reverse in [false, true] {
        let mut roles = vec![first_role.clone(), second_role.clone()];
        if reverse {
            roles.reverse();
        }
        let payload = json!({
            "swigConfigAddress": returned.to_string(), "walletAddress": wallet.to_string(),
            "roles": roles,
        });
        for body in [payload.clone(), json!({"data": payload})] {
            let client = HttpClient::new(config().http)?;
            queue(&client, vec![Ok(response(200, body))])?;
            let result = list_roles(&client, &requested, Network::Devnet).await?;
            assert_eq!(result.swig_config_address, returned);
            assert_eq!(result.wallet_address, wallet);
            assert_eq!(result.roles.len(), 2);
            let ids: Vec<_> = result.roles.iter().map(|role| role.role_id).collect();
            assert_eq!(ids, if reverse { vec![3, 9] } else { vec![9, 3] });
            for role in result.roles {
                match role.role_id {
                    9 => {
                        assert_eq!(role.authority.authority_type, 1);
                        assert_eq!(role.authority.authority_value, requested.to_string());
                        assert_eq!(role.actions.len(), 2);
                        let mut actions = role.actions.into_iter();
                        let all = actions.next().ok_or("missing all action")?;
                        assert_eq!(all.action_index, 2);
                        assert!(matches!(all.permission, RolePermission::All));
                        let recurring = actions.next().ok_or("missing recurring action")?;
                        assert_eq!(recurring.action_index, 5);
                        assert!(matches!(
                            recurring.permission,
                            RolePermission::SolRecurringLimit {
                                recurring_amount: 9_007_199_254_740_993,
                                window: 3600,
                                last_reset: 42,
                                current_amount: u64::MAX,
                            }
                        ));
                    }
                    3 => {
                        assert_eq!(role.authority.authority_type, 99);
                        assert_eq!(role.authority.authority_value, "backend-authority");
                        assert_eq!(role.actions.len(), 1);
                        let action = role
                            .actions
                            .into_iter()
                            .next()
                            .ok_or("missing program action")?;
                        assert_eq!(action.action_index, 1);
                        assert!(
                            matches!(action.permission, RolePermission::Program { program_id } if program_id == program)
                        );
                    }
                    _ => return Err("unexpected role".into()),
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn roles_api_preserves_aliases_and_omitted_defaults() -> Result<(), Box<dyn Error>> {
    let address = Pubkey::new_from_array([1; 32]);
    let wallet = Pubkey::new_from_array([2; 32]);
    let payload = json!({
        "swig_config_address": address.to_string(), "wallet_address": wallet.to_string(),
        "roles": [{
            "authority_type": 7, "authority_value": "",
            "actions": [{"action_code": 7, "action_data": {}}],
        }, {"authority_type": 8, "authority_value": ""}],
        "futureMetadata": true,
    });
    for body in [payload.clone(), json!({"data": payload})] {
        let client = HttpClient::new(config().http)?;
        queue(&client, vec![Ok(response(200, body))])?;
        let result = list_roles(&client, &address, Network::Devnet).await?;
        assert_eq!(result.roles.len(), 2);
        let mut roles = result.roles.into_iter();
        let role = roles.next().ok_or("missing first role")?;
        assert_eq!(role.role_id, 0);
        assert_eq!(role.authority.authority_type, 7);
        assert_eq!(role.authority.authority_value, "");
        let action = role.actions.into_iter().next().ok_or("missing action")?;
        assert_eq!(action.action_index, 0);
        assert!(matches!(action.permission, RolePermission::All));
        let role = roles.next().ok_or("missing second role")?;
        assert_eq!(role.role_id, 0);
        assert!(role.actions.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn roles_api_rejects_malformed_shapes_without_retry() -> Result<(), Box<dyn Error>> {
    let address = Pubkey::new_from_array([1; 32]);
    let wallet = Pubkey::new_from_array([2; 32]);
    let valid = json!({"swigConfigAddress":address.to_string(),"walletAddress":wallet.to_string()});
    let mut payloads = vec![
        json!([address.to_string(), wallet.to_string(), []]),
        json!(null),
        json!(true),
        json!("sensitive-response"),
        json!({}),
        json!({"swigConfigAddress":"invalid","walletAddress":wallet.to_string()}),
        json!({"swigConfigAddress":address.to_string(),"walletAddress":null}),
    ];
    for roles in [
        json!(null),
        json!({}),
        json!([[]]),
        json!([[1, 1, address.to_string(), []]]),
        json!([{"authorityType":1,"authorityValue":address.to_string(),"actions":null}]),
        json!([{"authorityType":1,"authorityValue":address.to_string(),"actions":{}}]),
        json!([{"authorityType":1,"authorityValue":address.to_string(),"actions":[[0,7,{}]]}]),
        json!([{"authorityType":1,"authorityValue":address.to_string(),"actions":[{"actionCode":7,"actionData":[]}]}]),
        json!([{"authorityType":1,"authorityValue":address.to_string(),"actions":[{"actionCode":7,"actionData":{}},{"actionCode":99,"actionData":{}}]}]),
        json!([{"authorityType":1,"authorityValue":address.to_string(),"actions":[]},[]]),
    ] {
        let mut payload = valid.clone();
        payload["roles"] = roles;
        payloads.push(payload);
    }
    for payload in payloads {
        for body in [payload.clone(), json!({"data":payload})] {
            let client = HttpClient::new(config().http)?;
            queue(&client, vec![Ok(response(200, body))])?;
            let error = list_roles(&client, &address, Network::Devnet)
                .await
                .err()
                .ok_or("expected malformed response")?;
            assert!(matches!(
                error,
                HttpError::InvalidResponse("malformed list_roles response")
            ));
            assert_eq!(
                error.to_string(),
                "invalid response: malformed list_roles response"
            );
            assert!(error.source().is_none());
            let transport = client.test.lock().map_err(|_| "test transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn roles_api_does_not_fall_back_or_recursively_unwrap_data() -> Result<(), Box<dyn Error>> {
    let address = Pubkey::new_from_array([1; 32]);
    let payload =
        json!({"swigConfigAddress":address.to_string(),"walletAddress":address.to_string()});
    for data in [
        json!(null),
        json!([payload.clone()]),
        json!({"data":payload.clone()}),
    ] {
        let mut body = payload.clone();
        body["data"] = data;
        let client = HttpClient::new(config().http)?;
        queue(&client, vec![Ok(response(200, body))])?;
        assert!(matches!(
            list_roles(&client, &address, Network::Devnet).await,
            Err(HttpError::InvalidResponse("malformed list_roles response"))
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
    Ok(())
}

#[tokio::test]
async fn roles_api_preserves_http_errors() -> Result<(), Box<dyn Error>> {
    let mut options = config().http;
    options.retry_options.max_retries = 0;
    let client = HttpClient::new(options)?;
    let address = Pubkey::new_from_array([1; 32]);
    queue(
        &client,
        vec![
            Err(HttpError::Timeout),
            Ok(response(400, json!({"code":"PERMISSION_DENIED"}))),
            Ok(response(503, json!({"code":"UNAVAILABLE"}))),
        ],
    )?;
    assert!(matches!(
        list_roles(&client, &address, Network::Devnet).await,
        Err(HttpError::Timeout)
    ));
    assert!(matches!(
        list_roles(&client, &address, Network::Devnet).await,
        Err(HttpError::PermissionDenied)
    ));
    assert!(matches!(
        list_roles(&client, &address, Network::Devnet).await,
        Err(HttpError::Unavailable)
    ));
    let transport = client.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 3);
    assert!(transport.responses.is_empty());
    Ok(())
}
