use super::decode_response;
use crate::{
    api::types::ramp_order::{Order, OrderStatus, Transfer},
    ramp_support::{order, transfer},
    RampOrder, RampOrderStatus, RampTransfer, RampTransferState,
};
use serde_json::{json, Value};
use solana_signature::Signature;
use std::error::Error;
fn decode_transfer(value: Value) -> Result<RampTransfer, Box<dyn Error>> {
    Ok(RampTransfer::try_from(serde_json::from_value::<Transfer>(
        value,
    )?)?)
}
fn decode_order(value: Value) -> Result<RampOrder, Box<dyn Error>> {
    Ok(RampOrder::try_from(serde_json::from_value::<Order>(
        value,
    )?)?)
}
#[test]
fn transfer_decoding_requires_consistent_state_and_native_signature() -> Result<(), Box<dyn Error>>
{
    let signature = Signature::from([4; 64]);
    for (state, signature, expected) in [
        (
            "SUBMITTED",
            Signature::from([4; 64]),
            RampTransferState::Submitted {
                signature: Signature::from([4; 64]),
            },
        ),
        (
            "LANDED",
            Signature::from([5; 64]),
            RampTransferState::Landed {
                signature: Signature::from([5; 64]),
            },
        ),
        (
            "FAILED",
            Signature::from([6; 64]),
            RampTransferState::Failed {
                signature: Signature::from([6; 64]),
            },
        ),
    ] {
        let mut value = transfer("transfer");
        value["state"] = json!(format!("TRANSFER_STATE_{state}"));
        assert!(decode_transfer(value.clone()).is_err());
        value["solanaSignature"] = json!("bad-signature");
        assert!(decode_transfer(value.clone()).is_err());
        value["solanaSignature"] = json!(signature.to_string());
        let raw: Transfer = serde_json::from_value(value.clone())?;
        assert_eq!(serde_json::to_value(raw.clone())?, value);
        let decoded = RampTransfer::try_from(raw)?;
        assert!(decoded.state == expected, "{state}");
        assert_eq!(decoded.transfer_id, "transfer");
        assert_eq!(decoded.expires_at, "2026-10-19T12:00:00Z");
    }
    for (state, expected) in [
        ("PREPARED", RampTransferState::Prepared),
        ("EXPIRED", RampTransferState::Expired),
    ] {
        let mut value = transfer("transfer");
        value["state"] = json!(format!("TRANSFER_STATE_{state}"));
        assert!(decode_transfer(value.clone())?.state == expected, "{state}");
        value["solanaSignature"] = Value::Null;
        assert!(decode_transfer(value.clone())?.state == expected, "{state}");
        value["solanaSignature"] = json!("");
        assert!(decode_transfer(value.clone()).is_err());
        value["solanaSignature"] = json!(signature.to_string());
        assert!(decode_transfer(value).is_err());
    }
    let mut value = transfer("transfer");
    value["state"] = json!("TRANSFER_STATE_UNSPECIFIED");
    assert!(decode_transfer(value).is_err());
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
        assert!(decode_transfer(value).is_err());
    }
    for field in ["transferId", "expiresAt"] {
        let mut value = transfer("transfer");
        value[field] = json!("");
        assert!(decode_transfer(value).is_err());
    }
}

#[test]
fn order_models_preserve_buy_sell_and_reject_invalid_status_or_metadata(
) -> Result<(), Box<dyn Error>> {
    let mut value = order("saved")["order"].clone();
    let decoded = decode_order(value.clone())?;
    assert!(matches!(decoded,RampOrder::Sell(value)if value.launch_url.is_none()));
    for (status, expected) in [
        ("CREATING", RampOrderStatus::Creating),
        ("CREATION_UNCERTAIN", RampOrderStatus::CreationUncertain),
        ("AWAITING_CUSTOMER", RampOrderStatus::AwaitingCustomer),
        ("AWAITING_TRANSFER", RampOrderStatus::AwaitingTransfer),
        ("PROCESSING", RampOrderStatus::Processing),
        ("SETTLING", RampOrderStatus::Settling),
        ("SETTLED", RampOrderStatus::Settled),
        ("DECLINED", RampOrderStatus::Declined),
        ("CANCELLED", RampOrderStatus::Cancelled),
        ("FAILED", RampOrderStatus::Failed),
        ("REFUNDED", RampOrderStatus::Refunded),
    ] {
        let mut response = value.clone();
        response["status"] = json!(format!("RAMP_ORDER_STATUS_{status}"));
        let RampOrder::Sell(decoded) = decode_order(response)? else {
            return Err("sell order".into());
        };
        assert!(decoded.status == expected, "{status}");
    }
    for status in [
        json!("RAMP_ORDER_STATUS_UNSPECIFIED"),
        Value::Null,
        json!(0),
        json!("other"),
        json!({"RAMP_ORDER_STATUS_SETTLED":null}),
    ] {
        assert!(serde_json::from_value::<OrderStatus>(status).is_err());
    }
    for field in ["id", "createdAt", "updatedAt"] {
        let mut invalid = value.clone();
        invalid[field] = json!("");
        assert!(decode_order(invalid).is_err());
    }
    value["buy"] = json!({"quote":{"spend":{"currencyCode":"USD","minorUnits":"1"},"receive":{"asset":{"sol":{}},"baseUnits":"2"},"totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"}});
    assert!(decode_order(value.clone()).is_err());
    value.as_object_mut().ok_or("object")?.remove("sell");
    assert!(matches!(decode_order(value)?, RampOrder::Buy(_)));
    Ok(())
}

#[test]
fn api_preserves_order_and_transfer_metadata_before_sdk_validation() -> Result<(), Box<dyn Error>> {
    let mut payload = order("saved");
    payload["order"]["id"] = json!("");
    payload["order"]["sell"]["launchUrl"] = json!("");
    let raw = decode_response(&serde_json::to_vec(&payload)?)?;
    assert_eq!(raw.order.id, "");
    assert_eq!(
        raw.order
            .sell
            .as_ref()
            .and_then(|sell| sell.launch_url.as_deref()),
        Some("")
    );
    assert!(RampOrder::try_from(raw.order).is_err());
    let mut raw_transfer = transfer("transfer");
    raw_transfer["state"] = json!("TRANSFER_STATE_SUBMITTED");
    let decoded: Transfer = serde_json::from_value(raw_transfer)?;
    assert!(decoded.solana_signature.is_none());
    assert!(RampTransfer::try_from(decoded).is_err());
    Ok(())
}
