use solana_pubkey::Pubkey;
use std::error::Error;
use swig_developer_sdk::{
    ClientConfig, Network, PrepareOperation, PrepareTransfersArgs, PrepareTransfersError,
    PreparedTransaction, WalletAuthority, WalletClient,
};

#[tokio::test]
async fn documented_batch_preparation_call_is_public() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(ClientConfig::new(
        "test-only-api-key".to_owned(),
        Network::Devnet,
    ))?;
    let public_key = Pubkey::new_from_array([1; 32]);
    let result: Result<PreparedTransaction, PrepareTransfersError> = client
        .prepare_transfers(PrepareTransfersArgs {
            swig_config_address: Pubkey::new_from_array([3; 32]),
            fee_payer: public_key,
            requester_authority: WalletAuthority::Ed25519 { public_key },
            operations: vec![PrepareOperation::TransferSol {
                destination: Pubkey::new_from_array([2; 32]),
                amount: 0,
            }],
        })
        .await;
    assert!(matches!(
        result,
        Err(PrepareTransfersError::InvalidInput(
            "amount must be greater than zero"
        ))
    ));
    Ok(())
}
