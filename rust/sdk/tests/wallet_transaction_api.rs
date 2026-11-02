use solana_pubkey::Pubkey;
use std::error::Error;
use swig_developer_sdk::{
    ClientConfig, Instruction, Network, PrepareTransactionArgs, PrepareTransactionError,
    PreparedTransaction, WalletAuthority, WalletClient,
};

#[tokio::test]
async fn documented_transaction_preparation_call_is_public() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(ClientConfig::new(
        "test-only-api-key".to_owned(),
        Network::Devnet,
    ))?;
    let public_key = Pubkey::new_from_array([1; 32]);
    let instructions: Vec<Instruction> = Vec::new();
    let result: Result<PreparedTransaction, PrepareTransactionError> = client
        .prepare_transaction(PrepareTransactionArgs {
            swig_config_address: Pubkey::new_from_array([3; 32]),
            fee_payer: public_key,
            requester_authority: WalletAuthority::Ed25519 { public_key },
            instructions,
            address_lookup_table_accounts: Vec::new(),
        })
        .await;
    assert!(matches!(
        result,
        Err(PrepareTransactionError::InvalidInput(
            "instructions must not be empty"
        ))
    ));
    Ok(())
}
