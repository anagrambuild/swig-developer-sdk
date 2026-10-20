use base64::{
    alphabet,
    engine::{
        general_purpose::{GeneralPurpose, GeneralPurposeConfig},
        DecodePaddingMode,
    },
    Engine,
};
use http::{HeaderMap, StatusCode};
use serde_json::Value;
use thiserror::Error;

use super::{
    types::{validate_offer, validate_required},
    ExactSvmPaymentRequirement, PaymentRequiredV2, PaymentRequirement,
};
use crate::types::wire::x402::{ExactSvmRequirementWire, PaymentRequiredHeaderWire};

const PAYMENT_REQUIRED_HEADER: &str = "payment-required";

#[derive(Debug, Error)]
pub enum ParsePaymentRequiredError {
    #[error("expected HTTP 402, received {status}")]
    UnexpectedStatus { status: StatusCode },

    #[error("PAYMENT-REQUIRED header is missing")]
    MissingHeader,

    #[error("invalid PAYMENT-REQUIRED header: {0}")]
    InvalidHeader(&'static str),
}

/// Parses V2 requirements without making a request or authorizing a payment.
pub fn parse_payment_required(
    status: StatusCode,
    headers: &HeaderMap,
) -> Result<PaymentRequiredV2, ParsePaymentRequiredError> {
    if status != StatusCode::PAYMENT_REQUIRED {
        return Err(ParsePaymentRequiredError::UnexpectedStatus { status });
    }
    let header = headers
        .get(PAYMENT_REQUIRED_HEADER)
        .ok_or(ParsePaymentRequiredError::MissingHeader)?;
    if headers.get_all(PAYMENT_REQUIRED_HEADER).iter().count() != 1 {
        return Err(ParsePaymentRequiredError::InvalidHeader(
            "expected exactly one header",
        ));
    }
    let header = header
        .to_str()
        .map_err(|_| ParsePaymentRequiredError::InvalidHeader("invalid header text"))?;
    let content = header.trim_end_matches('=');
    if header.len().saturating_sub(content.len()) > 2
        || content
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'+' && byte != b'/')
        || (content.len() != header.len() && header.len() % 4 != 0)
    {
        return Err(ParsePaymentRequiredError::InvalidHeader(
            "invalid Base64 encoding",
        ));
    }
    let decoder = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new()
            .with_decode_padding_mode(DecodePaddingMode::Indifferent)
            .with_decode_allow_trailing_bits(true),
    );
    let bytes = decoder
        .decode(header)
        .map_err(|_| ParsePaymentRequiredError::InvalidHeader("invalid Base64 encoding"))?;
    let payload: Value = serde_json::from_slice(&bytes)
        .map_err(|_| ParsePaymentRequiredError::InvalidHeader("invalid V2 payment requirements"))?;
    if !payload.is_object() || !payload.get("resource").is_some_and(Value::is_object) {
        return Err(ParsePaymentRequiredError::InvalidHeader(
            "invalid V2 payment requirements",
        ));
    }
    let wire: PaymentRequiredHeaderWire = serde_json::from_slice(&bytes)
        .map_err(|_| ParsePaymentRequiredError::InvalidHeader("invalid V2 payment requirements"))?;
    let accepts = wire
        .accepts
        .into_iter()
        .map(|offer| {
            if !offer.is_object() || !offer.get("extra").is_some_and(Value::is_object) {
                return PaymentRequirement::Unsupported;
            }
            let wire: ExactSvmRequirementWire = match serde_json::from_value(offer) {
                Ok(wire) => wire,
                Err(_) => return PaymentRequirement::Unsupported,
            };
            let offer = match ExactSvmPaymentRequirement::try_from(wire) {
                Ok(offer) => offer,
                Err(_) => return PaymentRequirement::Unsupported,
            };
            match validate_offer(&offer) {
                Ok(()) => PaymentRequirement::ExactSvm(offer),
                Err(_) => PaymentRequirement::Unsupported,
            }
        })
        .collect();
    let required = PaymentRequiredV2 {
        x402_version: wire.x402_version,
        error: wire.error,
        resource: wire.resource,
        accepts,
        extensions: wire.extensions.map(Value::Object).unwrap_or(Value::Null),
    };
    validate_required(&required).map_err(ParsePaymentRequiredError::InvalidHeader)?;
    Ok(required)
}
