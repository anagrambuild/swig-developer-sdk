use super::{
    network::Network,
    transaction::{validate_prepared_transaction_shape, PreparedTransaction, WalletAddressInfo},
};
use crate::api::error::ApiConversionError;
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/api/prepared_batch.rs"]
mod tests;
#[derive(Deserialize)]
pub(crate) struct PreparedBatch {
    pub wallet: Option<WalletAddressInfo>,
    pub network: Option<Network>,
    pub transactions: Vec<PreparedTransaction>,
}

pub(crate) fn validate_prepared_batch_shape(value: &Value) -> Result<(), ApiConversionError> {
    let fields = value
        .as_object()
        .ok_or(ApiConversionError::Invalid("malformed transaction batch"))?;
    if fields
        .get("wallet")
        .is_some_and(|wallet| !wallet.is_null() && !wallet.is_object())
    {
        return Err(ApiConversionError::Invalid("malformed transaction batch"));
    }
    if let Some(transactions) = fields.get("transactions").and_then(Value::as_array) {
        for transaction in transactions {
            validate_prepared_transaction_shape(transaction)?;
        }
    }
    Ok(())
}
