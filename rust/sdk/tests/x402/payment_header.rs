use crate::x402_test_support::{challenge, required};
use crate::{
    types::wire::tests::transaction::native_transaction, x402::create_x402_payment,
    CreateX402PaymentError, PaymentRequirement,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;
use std::error::Error;

fn transaction_with_size(size: usize) -> Result<VersionedTransaction, Box<dyn Error>> {
    let mut transaction = native_transaction(false);
    let solana_message::VersionedMessage::Legacy(message) = &mut transaction.message else {
        return Err("expected legacy test transaction".into());
    };
    message.instructions.first_mut().ok_or("instruction")?.data = vec![0; size];
    let overhead = bincode::serialize(&transaction)?
        .len()
        .checked_sub(size)
        .ok_or("overhead")?;
    let data_length = size.checked_sub(overhead).ok_or("size too small")?;
    let solana_message::VersionedMessage::Legacy(message) = &mut transaction.message else {
        return Err("expected legacy test transaction".into());
    };
    message.instructions.first_mut().ok_or("instruction")?.data = vec![0; data_length];
    assert_eq!(bincode::serialize(&transaction)?.len(), size);
    Ok(transaction)
}

#[test]
fn payment_header_preserves_offer_and_extensions_without_requiring_full_signatures(
) -> Result<(), Box<dyn Error>> {
    for extension in [Value::Null, json!({"nested": [1, "kept"]})] {
        let mut value = challenge();
        value["extensions"] = extension.clone();
        let transaction = native_transaction(false);
        let expected_transaction = STANDARD.encode(bincode::serialize(&transaction)?);
        let payment = create_x402_payment(required(value.clone())?, 0, transaction)?;
        let header = payment
            .payment_signature_headers
            .get("payment-signature")
            .ok_or("missing payment header")?;
        assert!(header.is_sensitive());
        let payload: Value = serde_json::from_slice(&STANDARD.decode(header.as_bytes())?)?;
        assert_eq!(payload["accepted"], value["accepts"][0]);
        assert_eq!(payload["resource"], value["resource"]);
        assert_eq!(payload["payload"]["transaction"], expected_transaction);
        assert_eq!(
            payload.get("extensions"),
            if extension.is_null() {
                None
            } else {
                Some(&extension)
            }
        );
        assert_eq!(payment.payment_payload.extensions, extension);
    }
    Ok(())
}

#[test]
fn payment_header_rejects_invalid_selection_and_size_limits() -> Result<(), Box<dyn Error>> {
    assert!(matches!(
        create_x402_payment(required(challenge())?, 1, native_transaction(false)),
        Err(CreateX402PaymentError::InvalidInput(
            "accepted index is out of range"
        ))
    ));
    let mut unsupported = required(challenge())?;
    unsupported.accepts = vec![PaymentRequirement::Unsupported];
    assert!(matches!(
        create_x402_payment(unsupported, 0, native_transaction(false)),
        Err(CreateX402PaymentError::InvalidInput(
            "selected payment requirement is unsupported"
        ))
    ));
    let mut oversized = native_transaction(false);
    if let solana_message::VersionedMessage::Legacy(message) = &mut oversized.message {
        message
            .account_keys
            .extend((2..42).map(|value| Pubkey::new_from_array([value; 32])));
    }
    assert!(
        matches!(create_x402_payment(required(challenge())?, 0, oversized), Err(CreateX402PaymentError::TransactionTooLarge { size }) if size > 1232)
    );
    let mut value = challenge();
    value["extensions"] = json!({"large": "x".repeat(8000)});
    assert!(
        matches!(create_x402_payment(required(value)?, 0, native_transaction(false)), Err(CreateX402PaymentError::HeaderTooLarge { size }) if size > 8000)
    );
    Ok(())
}

#[test]
fn payment_transaction_limit_accepts_1232_and_rejects_1233_bytes() -> Result<(), Box<dyn Error>> {
    let payment = create_x402_payment(required(challenge())?, 0, transaction_with_size(1232)?)?;
    assert_eq!(
        STANDARD
            .decode(&payment.payment_payload.payload.transaction)?
            .len(),
        1232
    );
    assert!(matches!(
        create_x402_payment(required(challenge())?, 0, transaction_with_size(1233)?),
        Err(CreateX402PaymentError::TransactionTooLarge { size: 1233 })
    ));
    Ok(())
}

#[test]
fn payment_header_limit_accepts_8000_and_rejects_8004_bytes() -> Result<(), Box<dyn Error>> {
    let mut value = challenge();
    value["extensions"] = json!({"padding":""});
    let baseline = create_x402_payment(required(value.clone())?, 0, native_transaction(false))?;
    let baseline_header = baseline
        .payment_signature_headers
        .get("payment-signature")
        .ok_or("header")?;
    let baseline_bytes = STANDARD.decode(baseline_header.as_bytes())?.len();
    let padding = 6000_usize
        .checked_sub(baseline_bytes)
        .ok_or("fixture exceeds boundary")?;
    value["extensions"]["padding"] = json!("x".repeat(padding));
    let payment = create_x402_payment(required(value.clone())?, 0, native_transaction(false))?;
    let header = payment
        .payment_signature_headers
        .get("payment-signature")
        .ok_or("header")?;
    assert_eq!(header.as_bytes().len(), 8000);
    assert_eq!(STANDARD.decode(header.as_bytes())?.len(), 6000);
    value["extensions"]["padding"] = json!("x".repeat(padding + 1));
    assert!(matches!(
        create_x402_payment(required(value)?, 0, native_transaction(false)),
        Err(CreateX402PaymentError::HeaderTooLarge { size: 8004 })
    ));
    Ok(())
}

#[test]
fn local_header_construction_revalidates_models_and_transaction_structure(
) -> Result<(), Box<dyn Error>> {
    let mut transaction = native_transaction(false);
    transaction.signatures.clear();
    assert!(matches!(
        create_x402_payment(required(challenge())?, 0, transaction),
        Err(CreateX402PaymentError::InvalidInput(_))
    ));
    let mut invalid_version = required(challenge())?;
    invalid_version.x402_version = 1;
    assert!(matches!(
        create_x402_payment(invalid_version, 0, native_transaction(false)),
        Err(CreateX402PaymentError::InvalidInput(_))
    ));
    let mut invalid_extensions = required(challenge())?;
    invalid_extensions.extensions = json!(["unsupported"]);
    assert!(matches!(
        create_x402_payment(invalid_extensions, 0, native_transaction(false)),
        Err(CreateX402PaymentError::InvalidInput(_))
    ));
    let mut invalid_timeout = required(challenge())?;
    let PaymentRequirement::ExactSvm(offer) = invalid_timeout.accepts.first_mut().ok_or("offer")?
    else {
        return Err("expected exact offer".into());
    };
    offer.max_timeout_seconds = f64::INFINITY;
    assert!(matches!(
        create_x402_payment(invalid_timeout, 0, native_transaction(false)),
        Err(CreateX402PaymentError::InvalidInput(_))
    ));
    Ok(())
}
