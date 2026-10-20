use crate::x402_test_support::{challenge, headers, required};
use crate::{x402::parse_payment_required, ParsePaymentRequiredError, PaymentRequirement};
use base64::{engine::general_purpose::STANDARD, Engine};
use http::{HeaderMap, HeaderValue, StatusCode};
use serde_json::{json, Value};
use std::error::Error;

#[test]
fn parser_preserves_duplicate_field_rejection() -> Result<(), Box<dyn Error>> {
    let valid = serde_json::to_string(&challenge())?;
    for raw in [
        valid.replace("\"x402Version\":2", "\"x402Version\":1,\"x402Version\":2"),
        valid.replace(
            "\"url\":",
            "\"url\":\"https://other.example/item\",\"url\":",
        ),
    ] {
        assert_ne!(raw, valid);
        let mut headers = HeaderMap::new();
        headers.insert(
            "payment-required",
            HeaderValue::from_str(&STANDARD.encode(raw))?,
        );
        assert!(matches!(
            parse_payment_required(StatusCode::PAYMENT_REQUIRED, &headers),
            Err(ParsePaymentRequiredError::InvalidHeader(
                "invalid V2 payment requirements"
            ))
        ));
    }
    Ok(())
}

#[test]
fn parser_rejects_positional_envelopes_and_resources() -> Result<(), Box<dyn Error>> {
    let value = challenge();
    let envelope = json!([2, null, value["resource"], value["accepts"], null]);
    let mut positional_resource = value;
    positional_resource["resource"] = json!([
        "https://resource.example/item",
        null,
        null,
        "Example",
        ["item"],
        null
    ]);
    for value in [envelope, positional_resource] {
        assert!(matches!(
            parse_payment_required(StatusCode::PAYMENT_REQUIRED, &headers(value)?),
            Err(ParsePaymentRequiredError::InvalidHeader(
                "invalid V2 payment requirements"
            ))
        ));
    }
    Ok(())
}

#[test]
fn positional_offers_and_extra_are_unsupported_without_losing_indexes() -> Result<(), Box<dyn Error>>
{
    let mut value = challenge();
    let valid = value["accepts"][0].clone();
    let positional_offer = json!([
        valid["scheme"],
        valid["network"],
        valid["amount"],
        valid["asset"],
        valid["payTo"],
        valid["maxTimeoutSeconds"],
        valid["extra"]
    ]);
    let mut positional_extra = valid.clone();
    positional_extra["extra"] = json!([
        valid["extra"]["feePayer"],
        null,
        null,
        valid["extra"]["lastValidBlockHeight"]
    ]);
    value["accepts"] = json!([positional_offer, valid.clone(), positional_extra, valid]);
    value["extensions"] = json!({"nested": [1, {"value": [true]}]});
    let parsed = required(value.clone())?;
    assert_eq!(parsed.accepts.len(), 4);
    for index in [0, 2] {
        assert!(matches!(
            parsed.accepts.get(index),
            Some(PaymentRequirement::Unsupported)
        ));
    }
    for index in [1, 3] {
        assert!(matches!(
            parsed.accepts.get(index),
            Some(PaymentRequirement::ExactSvm(_))
        ));
    }
    assert_eq!(parsed.resource.tags, Some(vec!["item".into()]));
    assert_eq!(parsed.extensions, value["extensions"]);
    Ok(())
}

#[test]
fn parses_supported_offers_without_losing_unsupported_indexes() -> Result<(), Box<dyn Error>> {
    let mut value = challenge();
    let offer = value.pointer("/accepts/0").ok_or("missing offer")?.clone();
    let mut unknown_extra = offer.clone();
    unknown_extra
        .pointer_mut("/extra")
        .and_then(Value::as_object_mut)
        .ok_or("missing extra")?
        .insert("custom".into(), json!(true));
    value["accepts"] = json!([null, {"scheme":"future"}, unknown_extra, offer]);
    value["extensions"] = json!({"nested": [1, true, {"value":"preserved"}]});
    let parsed = required(value.clone())?;
    assert_eq!(parsed.accepts.len(), 4);
    assert!(parsed
        .accepts
        .iter()
        .take(3)
        .all(|offer| matches!(offer, PaymentRequirement::Unsupported)));
    assert!(matches!(
        parsed.accepts.get(3),
        Some(PaymentRequirement::ExactSvm(_))
    ));
    assert_eq!(parsed.extensions, value["extensions"]);
    for (path, invalid) in [
        ("/accepts/3/asset", json!("not-a-public-key")),
        ("/accepts/3/extra", Value::Null),
        ("/accepts/3/maxTimeoutSeconds", json!(0)),
        ("/accepts/3/extra/lastValidBlockHeight", json!("-1")),
    ] {
        let mut invalid_value = value.clone();
        *invalid_value
            .pointer_mut(path)
            .ok_or("missing test field")? = invalid;
        assert!(matches!(
            required(invalid_value)?.accepts.get(3),
            Some(PaymentRequirement::Unsupported)
        ));
    }
    Ok(())
}

#[test]
fn parser_rejects_bad_envelope_and_headers_with_typed_errors() -> Result<(), Box<dyn Error>> {
    assert!(matches!(
        parse_payment_required(StatusCode::OK, &HeaderMap::new()),
        Err(ParsePaymentRequiredError::UnexpectedStatus {
            status: StatusCode::OK
        })
    ));
    assert!(matches!(
        parse_payment_required(StatusCode::PAYMENT_REQUIRED, &HeaderMap::new()),
        Err(ParsePaymentRequiredError::MissingHeader)
    ));
    let mut duplicate = headers(challenge())?;
    duplicate.append("payment-required", HeaderValue::from_static("e30="));
    assert!(matches!(
        parse_payment_required(StatusCode::PAYMENT_REQUIRED, &duplicate),
        Err(ParsePaymentRequiredError::InvalidHeader(
            "expected exactly one header"
        ))
    ));
    for (field, invalid) in [
        ("x402Version", json!(1)),
        ("extensions", json!([1])),
        ("accepts", json!([])),
    ] {
        let mut value = challenge();
        value[field] = invalid;
        assert!(matches!(
            parse_payment_required(StatusCode::PAYMENT_REQUIRED, &headers(value)?),
            Err(ParsePaymentRequiredError::InvalidHeader(_))
        ));
    }
    for bad in ["_", "e30===", "secret-sentinel"] {
        let mut invalid = HeaderMap::new();
        invalid.insert("payment-required", HeaderValue::from_str(bad)?);
        let error = parse_payment_required(StatusCode::PAYMENT_REQUIRED, &invalid)
            .err()
            .ok_or("expected error")?;
        assert!(matches!(error, ParsePaymentRequiredError::InvalidHeader(_)));
        assert!(!error.to_string().contains("secret-sentinel"));
    }
    Ok(())
}

#[test]
fn unusable_offer_fields_keep_their_original_slot() -> Result<(), Box<dyn Error>> {
    for (path, invalid) in [
        ("/scheme", json!("other")),
        ("/network", json!("solana:unknown")),
        ("/amount", json!(10)),
        ("/amount", json!("")),
        ("/amount", json!("1.5")),
        ("/amount", json!("-1")),
        ("/asset", json!(null)),
        ("/payTo", json!("bad-public-key")),
        ("/maxTimeoutSeconds", json!(null)),
        ("/maxTimeoutSeconds", json!(-1)),
        ("/extra/feePayer", json!("bad-public-key")),
        ("/extra/lastValidBlockHeight", json!(1)),
    ] {
        let mut value = challenge();
        let valid = value["accepts"][0].clone();
        let mut invalid_offer = valid.clone();
        *invalid_offer.pointer_mut(path).ok_or("test field")? = invalid;
        value["accepts"] = json!([valid.clone(), invalid_offer, valid]);
        let parsed = required(value)?;
        assert_eq!(parsed.accepts.len(), 3);
        assert!(matches!(
            parsed.accepts.first(),
            Some(PaymentRequirement::ExactSvm(_))
        ));
        assert!(matches!(
            parsed.accepts.get(1),
            Some(PaymentRequirement::Unsupported)
        ));
        assert!(matches!(
            parsed.accepts.get(2),
            Some(PaymentRequirement::ExactSvm(_))
        ));
    }
    for missing in ["extra", "asset", "payTo"] {
        let mut value = challenge();
        value["accepts"][0]
            .as_object_mut()
            .ok_or("offer")?
            .remove(missing);
        assert!(matches!(
            required(value)?.accepts.first(),
            Some(PaymentRequirement::Unsupported)
        ));
    }
    let mut value = challenge();
    value["accepts"][0]["extra"]
        .as_object_mut()
        .ok_or("extra")?
        .remove("feePayer");
    assert!(matches!(
        required(value)?.accepts.first(),
        Some(PaymentRequirement::Unsupported)
    ));
    Ok(())
}
