use super::{validate_prepared_batch_shape, PreparedBatch};
use crate::{
    api::{
        error::ApiConversionError,
        types::transaction::PreparedTransaction as ApiPreparedTransaction,
    },
    types::transaction::tests::prepared,
    wallet::prepared::normalize_batch,
    Network,
};
use serde_json::json;
use solana_pubkey::Pubkey;

#[test]
fn api_preserves_batch_members_and_both_metadata_levels() -> Result<(), Box<dyn std::error::Error>>
{
    let transaction = serde_json::to_value(ApiPreparedTransaction::try_from(prepared(false))?)?;
    let value = json!({"wallet":transaction["wallet"],"network":"NETWORK_DEVNET","transactions":[transaction.clone(),transaction]});
    validate_prepared_batch_shape(&value)?;
    let batch: PreparedBatch = serde_json::from_value(value)?;
    assert_eq!(batch.transactions.len(), 2);
    assert!(batch.wallet.is_some());
    assert!(batch
        .transactions
        .iter()
        .all(|item| item.wallet.is_some() && item.network.is_some()));
    assert!(matches!(
        normalize_batch(batch, None, Network::Devnet),
        Err(ApiConversionError::Invalid(
            "expected exactly one prepared transaction"
        ))
    ));
    Ok(())
}

#[test]
fn single_batch_preserves_inner_metadata_or_uses_outer_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    let transaction = serde_json::to_value(ApiPreparedTransaction::try_from(prepared(false))?)?;
    for (inner_wallet, inner_network) in
        [(true, true), (false, true), (true, false), (false, false)]
    {
        let mut value = json!({"wallet":transaction["wallet"],"network":"NETWORK_DEVNET","transactions":[transaction]});
        let fields = value["transactions"][0]
            .as_object_mut()
            .ok_or("expected transaction object")?;
        if !inner_wallet {
            fields.remove("wallet");
        }
        if !inner_network {
            fields.remove("network");
        }
        validate_prepared_batch_shape(&value)?;
        let batch: PreparedBatch = serde_json::from_value(value)?;
        let actual = normalize_batch(
            batch,
            Some(Pubkey::new_from_array([4; 32])),
            Network::Devnet,
        )?;
        let wallet = actual.wallet.ok_or("missing normalized wallet")?;
        assert_eq!(wallet.swig_config_address, Pubkey::new_from_array([4; 32]));
        assert_eq!(wallet.wallet_address, Pubkey::new_from_array([5; 32]));
        assert!(actual.network == Some(Network::Devnet));
        assert_eq!(actual.transaction, prepared(false).transaction);
    }
    Ok(())
}

#[test]
fn sdk_checks_each_metadata_level_after_api_decoding() -> Result<(), Box<dyn std::error::Error>> {
    let transaction = serde_json::to_value(ApiPreparedTransaction::try_from(prepared(false))?)?;
    let base = json!({"wallet":transaction["wallet"],"network":"NETWORK_DEVNET","transactions":[transaction]});
    for (path, replacement, expected) in [
        (
            "/network",
            json!("NETWORK_MAINNET"),
            "response network mismatch",
        ),
        (
            "/transactions/0/network",
            json!("NETWORK_MAINNET"),
            "response network mismatch",
        ),
        (
            "/transactions/0/wallet/swigConfigAddress",
            json!(Pubkey::new_from_array([6; 32]).to_string()),
            "response wallet mismatch",
        ),
        (
            "/transactions/0/wallet/walletAddress",
            json!(Pubkey::new_from_array([6; 32]).to_string()),
            "response wallet mismatch",
        ),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(path).ok_or("missing fixture field")? = replacement;
        validate_prepared_batch_shape(&value)?;
        let batch: PreparedBatch = serde_json::from_value(value)?;
        assert!(
            matches!(normalize_batch(batch, None, Network::Devnet),
            Err(ApiConversionError::Invalid(message)) if message == expected),
            "wrong error for {path}"
        );
    }
    Ok(())
}

#[test]
fn batch_guard_rejects_positional_members() {
    assert!(matches!(
        validate_prepared_batch_shape(&json!({"transactions":[[]]})),
        Err(ApiConversionError::Invalid(
            "malformed prepared transaction"
        ))
    ));
    assert!(matches!(
        validate_prepared_batch_shape(&json!([])),
        Err(ApiConversionError::Invalid("malformed transaction batch"))
    ));
}
