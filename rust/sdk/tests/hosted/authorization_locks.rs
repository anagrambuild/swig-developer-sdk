use crate::{
    hosted_support::{config, queue, response},
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

fn add_args() -> PrepareAddAuthorizationLockArgs {
    PrepareAddAuthorizationLockArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        requester_authority: authority(),
        mint: None,
        amount: u64::MAX,
        expiry: AuthorizationLockExpiry::DurationSlots(150),
    }
}

#[tokio::test]
async fn lock_preparations_preserve_typed_payloads_and_current_kinds() -> Result<(), Box<dyn Error>>
{
    for (mint, expiry, field, value) in [
        (
            None,
            AuthorizationLockExpiry::DurationSlots(150),
            "durationSlots",
            "150",
        ),
        (
            Some(key(5)),
            AuthorizationLockExpiry::ExpirySlot(u64::MAX),
            "expirySlot",
            "18446744073709551615",
        ),
    ] {
        let client = WalletClient::new(config())?;
        let mut add = prepared()?;
        add["kind"] = json!(10);
        let mut remove = prepared()?;
        remove["kind"] = json!("PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK");
        queue(
            &client.http,
            vec![
                Ok(response(200, json!({"data":{"transaction":add}}))),
                Ok(response(200, json!({"transaction":remove}))),
            ],
        )?;
        client
            .prepare_add_authorization_lock(PrepareAddAuthorizationLockArgs {
                mint,
                expiry,
                ..add_args()
            })
            .await?;
        client
            .prepare_remove_authorization_lock(PrepareRemoveAuthorizationLockArgs {
                swig_config_address: key(3),
                fee_payer: key(1),
                requester_authority: authority(),
                lock_id: u32::MAX,
            })
            .await?;
        let transport = client.http.test.lock().map_err(|_| "transport lock")?;
        for (index, path) in [(0, "add"), (1, "remove")] {
            let request = transport.requests.get(index).ok_or("request missing")?;
            assert_eq!(
                request.url().path(),
                format!("/transaction/wallet/authorization-lock/{path}")
            );
            assert_eq!(request.method(), reqwest::Method::POST);
            let body: Value = serde_json::from_slice(
                request
                    .body()
                    .and_then(|b| b.as_bytes())
                    .ok_or("body missing")?,
            )?;
            assert_eq!(body["swigAddress"], key(3).to_string());
            assert_eq!(body["network"], "NETWORK_DEVNET");
            assert_eq!(
                body["requesterAuthority"],
                json!({"ed25519":{"publicKey":key(1).to_string()}})
            );
            if index == 0 {
                assert_eq!(body["amount"], u64::MAX.to_string());
                assert_eq!(body[field], value);
                assert_eq!(
                    body.get("mint"),
                    mint.map(|key| json!(key.to_string())).as_ref()
                );
                assert_eq!(
                    body.get(if field == "expirySlot" {
                        "durationSlots"
                    } else {
                        "expirySlot"
                    }),
                    None
                );
            } else {
                assert_eq!(body["lockId"], u32::MAX);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn rejects_zero_amount_or_expiry_before_io() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    for args in [
        PrepareAddAuthorizationLockArgs {
            amount: 0,
            ..add_args()
        },
        PrepareAddAuthorizationLockArgs {
            expiry: AuthorizationLockExpiry::DurationSlots(0),
            ..add_args()
        },
        PrepareAddAuthorizationLockArgs {
            expiry: AuthorizationLockExpiry::ExpirySlot(0),
            ..add_args()
        },
    ] {
        assert!(matches!(
            client.prepare_add_authorization_lock(args).await,
            Err(AuthorizationLockError::InvalidInput(_))
        ));
    }
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
async fn rejects_yield_and_wrong_lock_kinds_and_wallet_identity() -> Result<(), Box<dyn Error>> {
    for kind in [7, 8, 9, 11] {
        let client = WalletClient::new(config())?;
        let mut transaction = prepared()?;
        transaction["kind"] = json!(kind);
        queue(
            &client.http,
            vec![Ok(response(200, json!({"transaction":transaction})))],
        )?;
        assert!(matches!(
            client.prepare_add_authorization_lock(add_args()).await,
            Err(AuthorizationLockError::InvalidResponse(_))
        ));
    }
    let client = WalletClient::new(config())?;
    let mut transaction = prepared()?;
    transaction["wallet"]["swigConfigAddress"] = json!(key(9).to_string());
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transaction":transaction})))],
    )?;
    assert!(matches!(
        client.prepare_add_authorization_lock(add_args()).await,
        Err(AuthorizationLockError::InvalidResponse(_))
    ));
    Ok(())
}

fn locks() -> Value {
    json!({"currentSlot":"100","nextLockId":3,"locks":[
        {"lockId":0,"roleId":1,"assetAccount":key(4).to_string(),"amount":u64::MAX.to_string(),"expirySlot":"101","active":true},
        {"lockId":1,"roleId":1,"assetAccount":key(5).to_string(),"mint":key(6).to_string(),"amount":"0","expirySlot":"101","active":false},
        {"lockId":2,"roleId":1,"assetAccount":key(4).to_string(),"amount":"10","expirySlot":"100","active":false}
    ]})
}

#[tokio::test]
async fn reads_active_captured_and_expired_holds() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(200, json!({"data":locks()})))],
    )?;
    let result = client.get_authorization_locks(&key(3), true).await?;
    assert_eq!(result.locks.len(), 3);
    assert_eq!(result.locks.first().ok_or("lock")?.amount, u64::MAX);
    assert_eq!(result.locks.get(1).ok_or("lock")?.amount, 0);
    let transport = client.http.test.lock().map_err(|_| "transport lock")?;
    let request = transport.requests.first().ok_or("request")?;
    assert_eq!(
        request.url().path(),
        format!("/transaction/wallet/{}/authorization-locks", key(3))
    );
    assert_eq!(
        request.url().query(),
        Some("network=NETWORK_DEVNET&include_expired=true")
    );
    Ok(())
}

#[tokio::test]
async fn rejects_malformed_overflowing_and_inconsistent_lock_responses(
) -> Result<(), Box<dyn Error>> {
    let mut payloads = vec![
        json!([]),
        json!({"locks":[],"currentSlot":"18446744073709551616"}),
        json!({"locks":[[]],"currentSlot":"0"}),
    ];
    for (field, value) in [
        ("amount", json!("18446744073709551616")),
        ("amount", json!(-1)),
        ("lockId", json!(4294967296_u64)),
        ("active", json!(false)),
        ("roleId", json!(-1)),
        ("assetAccount", json!("bad")),
    ] {
        let mut payload = locks();
        payload["locks"][0][field] = value;
        payloads.push(payload);
    }
    let mut duplicate = locks();
    duplicate["locks"][1]["lockId"] = json!(0);
    payloads.push(duplicate);
    let mut next = locks();
    next["nextLockId"] = json!(2);
    payloads.push(next);
    for payload in payloads {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(matches!(
            client.get_authorization_locks(&key(3), true).await,
            Err(AuthorizationLockError::InvalidResponse(_))
        ));
    }
    Ok(())
}

#[test]
fn reads_manage_lock_permission_with_canonical_code_and_u64_bound() -> Result<(), Box<dyn Error>> {
    use crate::api::types::role_permission::RolePermission;
    let wire = json!({"actionCode":27,"actionData":{"max_duration_slots":u64::MAX.to_string()}});
    let role: RolePermission = serde_json::from_value(wire.clone())?;
    assert_eq!(serde_json::to_value(role.clone())?, wire);
    assert!(matches!(
        SwigRolePermission::from(role),
        SwigRolePermission::ManageAuthorizationLocks {
            max_duration_slots: u64::MAX
        }
    ));
    Ok(())
}
