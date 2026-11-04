use solana_pubkey::Pubkey;
use std::error::Error;
use swig_developer_sdk::{
    ClientConfig, Network, PrepareTransferSolArgs, PrepareTransferSolError,
    PrepareTransferTokenArgs, PrepareTransferTokenError, PreparedTransaction, WalletAuthority,
    WalletClient,
};

#[tokio::test]
async fn documented_transfer_calls_are_public() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(ClientConfig::new(
        "test-only-api-key".to_owned(),
        Network::Devnet,
    ))?;
    let public_key = Pubkey::new_from_array([1; 32]);
    let sol: Result<PreparedTransaction, PrepareTransferSolError> = client
        .prepare_transfer_sol(PrepareTransferSolArgs {
            swig_config_address: Pubkey::new_from_array([3; 32]),
            fee_payer: public_key,
            destination: Pubkey::new_from_array([2; 32]),
            amount: 0,
            requester_authority: WalletAuthority::Ed25519 { public_key },
        })
        .await;
    assert!(matches!(sol, Err(PrepareTransferSolError::InvalidAmount)));

    let token: Result<PreparedTransaction, PrepareTransferTokenError> = client
        .prepare_transfer_token(PrepareTransferTokenArgs {
            swig_config_address: Pubkey::new_from_array([3; 32]),
            fee_payer: public_key,
            mint: Pubkey::new_from_array([5; 32]),
            destination_owner: Pubkey::new_from_array([2; 32]),
            amount: 0,
            requester_authority: WalletAuthority::Ed25519 { public_key },
        })
        .await;
    assert!(matches!(
        token,
        Err(PrepareTransferTokenError::InvalidInput(
            "amount must be greater than zero"
        ))
    ));
    Ok(())
}
