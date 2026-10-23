use crate::{
    hosted_support::{config, queue, response},
    ramp_support::{client, order, positional_order_responses, transfer},
    *,
};
use serde_json::{json, Value};
use solana_signature::Signature;
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

#[test]
fn transfer_decoding_requires_consistent_state_and_native_signature() -> Result<(), Box<dyn Error>>
{
    let signature = Signature::from([4; 64]);
    for state in ["SUBMITTED", "LANDED", "FAILED"] {
        let mut value = transfer("transfer");
        value["state"] = json!(format!("TRANSFER_STATE_{state}"));
        assert!(serde_json::from_value::<RampTransfer>(value.clone()).is_err());
        value["solanaSignature"] = json!("bad-signature");
        assert!(serde_json::from_value::<RampTransfer>(value.clone()).is_err());
        value["solanaSignature"] = json!(signature.to_string());
        let decoded: RampTransfer = serde_json::from_value(value.clone())?;
        assert_eq!(serde_json::to_value(decoded)?, value);
    }
    for state in ["PREPARED", "EXPIRED"] {
        let mut value = transfer("transfer");
        value["state"] = json!(format!("TRANSFER_STATE_{state}"));
        assert!(serde_json::from_value::<RampTransfer>(value.clone()).is_ok());
        value["solanaSignature"] = Value::Null;
        assert!(serde_json::from_value::<RampTransfer>(value.clone()).is_ok());
        value["solanaSignature"] = json!("");
        assert!(serde_json::from_value::<RampTransfer>(value.clone()).is_err());
        value["solanaSignature"] = json!(signature.to_string());
        assert!(serde_json::from_value::<RampTransfer>(value).is_err());
    }
    let mut value = transfer("transfer");
    value["state"] = json!("TRANSFER_STATE_UNSPECIFIED");
    assert!(serde_json::from_value::<RampTransfer>(value).is_err());
    Ok(())
}

#[test]
fn transfer_states_reject_objects_and_incomplete_metadata() {
    for invalid in [
        json!(null),
        json!(0),
        json!("TRANSFER_STATE_UNSPECIFIED"),
        json!({"TRANSFER_STATE_PREPARED":null}),
        json!({"TRANSFER_STATE_EXPIRED":null}),
        json!({"TRANSFER_STATE_SUBMITTED":null}),
        json!({"TRANSFER_STATE_LANDED":null}),
        json!({"TRANSFER_STATE_FAILED":null}),
    ] {
        let mut value = transfer("transfer");
        value["state"] = invalid;
        assert!(serde_json::from_value::<RampTransfer>(value).is_err());
    }
    for field in ["transferId", "expiresAt"] {
        let mut value = transfer("transfer");
        value[field] = json!("");
        assert!(serde_json::from_value::<RampTransfer>(value).is_err());
    }
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
#[test]
fn order_models_preserve_buy_sell_and_reject_invalid_status_or_metadata(
) -> Result<(), Box<dyn Error>> {
    let mut value = order("saved")["order"].clone();
    let decoded: RampOrder = serde_json::from_value(value.clone())?;
    assert!(matches!(decoded,RampOrder::Sell(value)if value.launch_url.is_none()));
    for status in [
        "RAMP_ORDER_STATUS_CREATING",
        "RAMP_ORDER_STATUS_CREATION_UNCERTAIN",
        "RAMP_ORDER_STATUS_AWAITING_CUSTOMER",
        "RAMP_ORDER_STATUS_AWAITING_TRANSFER",
        "RAMP_ORDER_STATUS_PROCESSING",
        "RAMP_ORDER_STATUS_SETTLING",
        "RAMP_ORDER_STATUS_SETTLED",
        "RAMP_ORDER_STATUS_DECLINED",
        "RAMP_ORDER_STATUS_CANCELLED",
        "RAMP_ORDER_STATUS_FAILED",
        "RAMP_ORDER_STATUS_REFUNDED",
    ] {
        let status: RampOrderStatus = serde_json::from_value(json!(status))?;
        assert!(serde_json::to_value(status)?.is_string());
    }
    for status in [
        json!("RAMP_ORDER_STATUS_UNSPECIFIED"),
        Value::Null,
        json!(0),
        json!("other"),
        json!({"RAMP_ORDER_STATUS_SETTLED":null}),
    ] {
        assert!(serde_json::from_value::<RampOrderStatus>(status).is_err());
    }
    for field in ["id", "createdAt", "updatedAt"] {
        let mut invalid = value.clone();
        invalid[field] = json!("");
        assert!(serde_json::from_value::<RampOrder>(invalid).is_err());
    }
    value["buy"] = json!({"quote":{"spend":{"currencyCode":"USD","minorUnits":"1"},"receive":{"asset":{"sol":{}},"baseUnits":"2"},"totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"}});
    assert!(serde_json::from_value::<RampOrder>(value.clone()).is_err());
    value.as_object_mut().ok_or("object")?.remove("sell");
    assert!(matches!(
        serde_json::from_value::<RampOrder>(value)?,
        RampOrder::Buy(_)
    ));
    Ok(())
}
