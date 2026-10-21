use crate::api::{error::ApiConversionError, types::prepared_batch::PreparedBatch};
use crate::{Network, PreparedTransaction, WalletAddressInfo};
use solana_pubkey::Pubkey;
pub(crate) fn validate_prepared_context(
    prepared: &PreparedTransaction,
    swig_config_address: Option<Pubkey>,
    network: Network,
) -> Result<(), ApiConversionError> {
    if prepared.network.is_some_and(|actual| actual != network) {
        return Err(ApiConversionError::Invalid("response network mismatch"));
    }
    if let (Some(expected), Some(actual)) = (swig_config_address, &prepared.wallet) {
        if actual.swig_config_address != expected {
            return Err(ApiConversionError::Invalid("response wallet mismatch"));
        }
    }
    Ok(())
}

pub(crate) fn normalize_batch(
    batch: PreparedBatch,
    swig_config_address: Option<Pubkey>,
    network: Network,
) -> Result<PreparedTransaction, ApiConversionError> {
    if batch.transactions.len() != 1 {
        return Err(ApiConversionError::Invalid(
            "expected exactly one prepared transaction",
        ));
    }
    let batch_network = batch.network.map(Network::try_from).transpose()?;
    if batch_network.is_some_and(|actual| actual != network) {
        return Err(ApiConversionError::Invalid("response network mismatch"));
    }
    let wallet = batch.wallet.map(WalletAddressInfo::try_from).transpose()?;
    if let (Some(expected), Some(actual)) = (swig_config_address, &wallet) {
        if expected != actual.swig_config_address {
            return Err(ApiConversionError::Invalid("response wallet mismatch"));
        }
    }
    let wire = batch
        .transactions
        .into_iter()
        .next()
        .ok_or(ApiConversionError::Invalid("missing prepared transaction"))?;
    let mut prepared = PreparedTransaction::try_from(wire)?;
    if let (Some(outer), Some(inner)) = (&wallet, &prepared.wallet) {
        if outer.swig_config_address != inner.swig_config_address
            || outer.wallet_address != inner.wallet_address
        {
            return Err(ApiConversionError::Invalid("response wallet mismatch"));
        }
    }
    prepared.network = prepared.network.or(batch_network);
    prepared.wallet = prepared.wallet.or(wallet);
    validate_prepared_context(&prepared, swig_config_address, network)?;
    Ok(prepared)
}
