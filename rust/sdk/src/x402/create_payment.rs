use base64::{engine::general_purpose::STANDARD, Engine};
use http::{HeaderMap, HeaderValue};
use serde_json::Value;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

use super::{
    protocol::{self, ExactSvmRequirement, PaymentPayload},
    types::validate_required,
    ExactSvmPaymentRequirement, PaymentRequiredV2, PaymentRequirement, X402ResourceInfoV2,
};

const MAX_SOLANA_TRANSACTION_BYTES: usize = 1232;
const MAX_PAYMENT_SIGNATURE_HEADER_BYTES: usize = 8000;
const PAYMENT_SIGNATURE_HEADER: &str = "payment-signature";

pub struct ExactSvmPayload {
    pub transaction: String,
}

pub struct PaymentPayloadV2 {
    pub x402_version: u8,
    pub resource: Option<X402ResourceInfoV2>,
    pub accepted: ExactSvmPaymentRequirement,
    pub payload: ExactSvmPayload,
    pub extensions: Value,
}

pub struct X402PaymentSubmission {
    pub payment_payload: PaymentPayloadV2,
    pub payment_signature_headers: HeaderMap,
}

#[derive(Debug, Error)]
pub enum CreateX402PaymentError {
    #[error("invalid payment input: {0}")]
    InvalidInput(&'static str),

    #[error("payment transaction exceeds 1232 bytes: {size}")]
    TransactionTooLarge { size: usize },

    #[error("payment header exceeds 8000 bytes: {size}")]
    HeaderTooLarge { size: usize },

    #[error("payment encoding failed")]
    EncodingFailed,
}

/// Builds the payment header. It does not audit intent, verify signatures, or settle payment.
pub fn create_x402_payment(
    payment_required: PaymentRequiredV2,
    accepted_index: u32,
    transaction: VersionedTransaction,
) -> Result<X402PaymentSubmission, CreateX402PaymentError> {
    validate_required(&payment_required).map_err(CreateX402PaymentError::InvalidInput)?;
    let index = usize::try_from(accepted_index)
        .map_err(|_| CreateX402PaymentError::InvalidInput("invalid accepted index"))?;
    let accepted = match payment_required.accepts.get(index) {
        Some(PaymentRequirement::ExactSvm(offer)) => offer.clone(),
        Some(PaymentRequirement::Unsupported) => {
            return Err(CreateX402PaymentError::InvalidInput(
                "selected payment requirement is unsupported",
            ))
        }
        None => {
            return Err(CreateX402PaymentError::InvalidInput(
                "accepted index is out of range",
            ))
        }
    };
    transaction
        .sanitize()
        .map_err(|_| CreateX402PaymentError::InvalidInput("invalid transaction structure"))?;
    let bytes =
        bincode::serialize(&transaction).map_err(|_| CreateX402PaymentError::EncodingFailed)?;
    if bytes.len() > MAX_SOLANA_TRANSACTION_BYTES {
        return Err(CreateX402PaymentError::TransactionTooLarge { size: bytes.len() });
    }
    let transaction = STANDARD.encode(bytes);
    let wire = PaymentPayload {
        x402_version: 2,
        resource: (&payment_required.resource).into(),
        accepted: ExactSvmRequirement::from(&accepted),
        payload: protocol::ExactSvmPayload {
            transaction: transaction.clone(),
        },
        extensions: payment_required.extensions.clone(),
    };
    let header = STANDARD
        .encode(serde_json::to_vec(&wire).map_err(|_| CreateX402PaymentError::EncodingFailed)?);
    if header.len() > MAX_PAYMENT_SIGNATURE_HEADER_BYTES {
        return Err(CreateX402PaymentError::HeaderTooLarge { size: header.len() });
    }
    let mut value =
        HeaderValue::from_str(&header).map_err(|_| CreateX402PaymentError::EncodingFailed)?;
    value.set_sensitive(true);
    let mut payment_signature_headers = HeaderMap::new();
    payment_signature_headers.insert(PAYMENT_SIGNATURE_HEADER, value);
    Ok(X402PaymentSubmission {
        payment_payload: PaymentPayloadV2 {
            x402_version: 2,
            resource: Some(payment_required.resource),
            accepted,
            payload: ExactSvmPayload { transaction },
            extensions: payment_required.extensions,
        },
        payment_signature_headers,
    })
}
