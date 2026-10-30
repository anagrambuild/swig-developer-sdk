use crate::{
    hosted_support::{config, queue, response},
    ramp_support::{client, order, positional_order_responses},
    *,
};
use serde_json::{json, Value};
use std::error::Error;
#[tokio::test]
async fn positional_order_objects_are_rejected_without_retry() -> Result<(), Box<dyn Error>> {
    for (_, payload) in positional_order_responses()? {
        for body in [payload.clone(), json!({"data":payload})] {
            let ramp = client(Network::Mainnet)?;
            queue(&ramp.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                ramp.get_order("saved").await,
                Err(GetRampOrderError::InvalidResponse("malformed Ramp order"))
            ));
            assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn order_object_aliases_and_null_optionals_remain_accepted() -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    let mut payload = order("saved");
    let order = payload["order"].as_object_mut().ok_or("order")?;
    let created = order.remove("createdAt").ok_or("created")?;
    let updated = order.remove("updatedAt").ok_or("updated")?;
    order.insert("created_at".into(), created);
    order.insert("updated_at".into(), updated);
    order.insert("buy".into(), Value::Null);
    order.insert(
        "sell".into(),
        json!({"quote":{
            "sell":{"asset":{"sol":{}},"base_units":"12"},
            "receive":{"currency_code":"USD","minor_units":"1"},
            "total_fee":{"currency_code":"USD","minor_units":"0"},"exchange_rate":"1"
        },"launch_url":null,"deposit":null,"transfer":{
            "transfer_id":"transfer","state":"TRANSFER_STATE_PREPARED", "solana_signature":null,
            "expires_at":"2026-10-19T12:00:00Z"
        }}),
    );
    queue(&ramp.http, vec![Ok(response(200, json!({"data":payload})))])?;
    let RampOrder::Sell(order) = ramp.get_order("saved").await? else {
        return Err("sell order".into());
    };
    assert!(order.launch_url.is_none() && order.deposit.is_none());
    assert_eq!(order.quote.sell.base_units, 12);
    assert!(order.transfer.ok_or("transfer")?.state == RampTransferState::Prepared);
    Ok(())
}

#[tokio::test]
async fn lookup_retries_but_does_not_rewrite_saved_context() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Devnet)?;
    queue(
        &client.http,
        vec![
            Ok(response(503, json!({}))),
            Ok(response(200, order("saved"))),
            Ok(response(200, order("wrong"))),
        ],
    )?;
    client.get_order("saved").await?;
    assert!(matches!(
        client.get_order("saved").await,
        Err(GetRampOrderError::InvalidResponse(_))
    ));
    let test = client.http.test.lock().map_err(|_| "lock failed")?;
    assert_eq!(test.requests.len(), 3);
    assert!(test
        .requests
        .iter()
        .all(|request| request.url().query().is_none() && request.body().is_none()));
    Ok(())
}

#[tokio::test]
async fn saved_order_does_not_require_ramp_configuration() -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    queue(
        &ramp.http,
        vec![Ok(response(200, json!({"data":order("stored")})))],
    )?;
    assert!(matches!(ramp.get_order("stored").await?,RampOrder::Sell(value)if value.id=="stored"));
    let test = ramp.http.test.lock().map_err(|_| "lock")?;
    let request = test.requests.first().ok_or("request")?;
    assert_eq!(request.url().path(), "/wallet/api/ramp/orders/stored");
    assert!(request.url().query().is_none());
    assert!(request.body().is_none());
    Ok(())
}
#[tokio::test]
async fn order_read_errors_preserve_the_owning_identifier() -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    assert!(matches!(
        ramp.get_order(" ").await,
        Err(GetRampOrderError::InvalidInput(_))
    ));
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    queue(&ramp.http, vec![Ok(response(404, json!({})))])?;
    assert!(matches!(
        ramp.get_order("saved").await,
        Err(GetRampOrderError::UnexpectedServiceFailure)
    ));
    for body in [json!({"code":5}), json!({"code":"NOT_FOUND"})] {
        queue(&ramp.http, vec![Ok(response(404, body))])?;
        assert!(
            matches!(ramp.get_order("saved").await,Err(GetRampOrderError::OrderNotFound{order_id})if order_id=="saved")
        );
    }
    let mut malformed = order("saved");
    malformed["order"]["buy"] = malformed["order"]["sell"].clone();
    for body in [order("wrong"), malformed, json!({"order":{}})] {
        queue(&ramp.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            ramp.get_order("saved").await,
            Err(GetRampOrderError::InvalidResponse(_))
        ));
    }
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 6);
    Ok(())
}

#[tokio::test]
async fn deposits_require_the_quoted_asset_but_allow_repriced_amounts() -> Result<(), Box<dyn Error>>
{
    let assets = [
        json!({"sol":{}}),
        json!({"token":{"mint":solana_pubkey::Pubkey::new_from_array([8; 32]).to_string()}}),
        json!({"token":{"mint":solana_pubkey::Pubkey::new_from_array([9; 32]).to_string()}}),
    ];
    for quote_asset in &assets {
        for deposit_asset in &assets {
            let ramp = client(Network::Mainnet)?;
            let mut body = order("saved");
            body["order"]["sell"]["quote"]["sell"]["asset"] = quote_asset.clone();
            body["order"]["sell"]["deposit"] = json!({
                "address":solana_pubkey::Pubkey::new_from_array([7; 32]).to_string(),
                "amount":{"asset":deposit_asset,"baseUnits":"13"}
            });
            queue(&ramp.http, vec![Ok(response(200, body))])?;
            let result = ramp.get_order("saved").await;
            if quote_asset == deposit_asset {
                let RampOrder::Sell(order) = result? else {
                    return Err("sell order".into());
                };
                let deposit = order.deposit.ok_or("deposit")?;
                assert!(deposit.amount.asset == order.quote.sell.asset);
                assert_eq!(order.quote.sell.base_units, 12);
                assert_eq!(deposit.amount.base_units, 13);
            } else {
                assert!(matches!(
                    result,
                    Err(GetRampOrderError::InvalidResponse(
                        "Ramp deposit asset mismatch"
                    ))
                ));
            }
            assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
        }
    }
    Ok(())
}
