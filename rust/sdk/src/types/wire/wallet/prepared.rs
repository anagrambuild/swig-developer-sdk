use crate::types::wire::{
    transaction::{
        validate_prepared_transaction_shape, PreparedTransactionWire, WalletAddressInfoWire,
    },
    NetworkWire, WireError,
};
use crate::{Network, PreparedTransaction, WalletAddressInfo};
use serde::Deserialize;
use serde_json::Value;
use solana_pubkey::Pubkey;

#[derive(Deserialize)]
pub(crate) struct PreparedBatchWire {
    pub wallet: Option<WalletAddressInfoWire>,
    pub network: Option<NetworkWire>,
    pub transactions: Vec<PreparedTransactionWire>,
}

pub(crate) fn validate_prepared_batch_shape(value: &Value) -> Result<(), WireError> {
    let fields = value
        .as_object()
        .ok_or(WireError::Invalid("malformed transaction batch"))?;
    if fields
        .get("wallet")
        .is_some_and(|wallet| !wallet.is_null() && !wallet.is_object())
    {
        return Err(WireError::Invalid("malformed transaction batch"));
    }
    if let Some(transactions) = fields.get("transactions").and_then(Value::as_array) {
        for transaction in transactions {
            validate_prepared_transaction_shape(transaction)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_prepared_context(
    prepared: &PreparedTransaction,
    swig_config_address: Option<Pubkey>,
    network: Network,
) -> Result<(), WireError> {
    if prepared.network.is_some_and(|actual| actual != network) {
        return Err(WireError::Invalid("response network mismatch"));
    }
    if let (Some(expected), Some(actual)) = (swig_config_address, &prepared.wallet) {
        if actual.swig_config_address != expected {
            return Err(WireError::Invalid("response wallet mismatch"));
        }
    }
    Ok(())
}

pub(crate) fn normalize_batch(
    batch: PreparedBatchWire,
    swig_config_address: Option<Pubkey>,
    network: Network,
) -> Result<PreparedTransaction, WireError> {
    if batch.transactions.len() != 1 {
        return Err(WireError::Invalid(
            "expected exactly one prepared transaction",
        ));
    }
    let batch_network = batch.network.map(Network::try_from).transpose()?;
    if batch_network.is_some_and(|actual| actual != network) {
        return Err(WireError::Invalid("response network mismatch"));
    }
    let wallet = batch.wallet.map(WalletAddressInfo::try_from).transpose()?;
    if let (Some(expected), Some(actual)) = (swig_config_address, &wallet) {
        if expected != actual.swig_config_address {
            return Err(WireError::Invalid("response wallet mismatch"));
        }
    }
    let wire = batch
        .transactions
        .into_iter()
        .next()
        .ok_or(WireError::Invalid("missing prepared transaction"))?;
    let mut prepared = PreparedTransaction::try_from(wire)?;
    if let (Some(outer), Some(inner)) = (&wallet, &prepared.wallet) {
        if outer.swig_config_address != inner.swig_config_address
            || outer.wallet_address != inner.wallet_address
        {
            return Err(WireError::Invalid("response wallet mismatch"));
        }
    }
    prepared.network = prepared.network.or(batch_network);
    prepared.wallet = prepared.wallet.or(wallet);
    validate_prepared_context(&prepared, swig_config_address, network)?;
    Ok(prepared)
}
