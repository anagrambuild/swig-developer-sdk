use solana_pubkey::Pubkey;
use std::error::Error;
use swig_developer_sdk::{
    ClientConfig, Network, PrepareAddRoleArgs, PrepareAddRoleError, PreparedTransaction,
    WalletAuthority, WalletClient, WalletPermission,
};

#[tokio::test]
async fn documented_role_preparation_call_is_public() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(ClientConfig::new(
        "test-only-api-key".to_owned(),
        Network::Devnet,
    ))?;
    let public_key = Pubkey::new_from_array([1; 32]);
    let actions: Vec<WalletPermission> = Vec::new();
    let result: Result<PreparedTransaction, PrepareAddRoleError> = client
        .prepare_add_role(PrepareAddRoleArgs {
            swig_config_address: Pubkey::new_from_array([3; 32]),
            fee_payer: public_key,
            authority: WalletAuthority::Ed25519 { public_key },
            actions,
            requester_authority: WalletAuthority::Ed25519 { public_key },
        })
        .await;
    assert!(matches!(
        result,
        Err(PrepareAddRoleError::InvalidInput(
            "actions must not be empty"
        ))
    ));
    Ok(())
}
