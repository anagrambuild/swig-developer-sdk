use solana_pubkey::Pubkey;
use std::error::Error;
use swig_developer_sdk::{
    ClientConfig, Network, PrepareSwapArgs, PrepareSwapError, PreparedTransaction, SwapOptions,
    WalletAuthority, WalletClient,
};

#[test]
fn swap_options_default_to_no_overrides() {
    let options = SwapOptions::default();
    assert!(options.slippage_bps.is_none());
    assert!(options.destination_owner.is_none());
    assert!(options.wrap_and_unwrap_sol.is_none());
    assert!(options.tip_amount_lamports.is_none());
    assert!(options.compute_unit_price_percentile.is_none());
    assert!(options.max_accounts.is_none());
    assert!(options.mode.is_none());
    assert!(options.blockhash_slots_to_expiry.is_none());
}

#[tokio::test]
async fn documented_swap_preparation_call_is_public() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(ClientConfig::new(
        "test-only-api-key".to_owned(),
        Network::Devnet,
    ))?;
    let public_key = Pubkey::new_from_array([1; 32]);
    let result: Result<PreparedTransaction, PrepareSwapError> = client
        .prepare_swap(PrepareSwapArgs {
            swig_config_address: Pubkey::new_from_array([3; 32]),
            fee_payer: public_key,
            requester_authority: WalletAuthority::Ed25519 { public_key },
            input_mint: Pubkey::new_from_array([5; 32]),
            output_mint: Pubkey::new_from_array([6; 32]),
            amount: 0,
            options: Some(SwapOptions {
                slippage_bps: Some(50),
                wrap_and_unwrap_sol: Some(true),
                ..SwapOptions::default()
            }),
        })
        .await;
    assert!(matches!(
        result,
        Err(PrepareSwapError::InvalidInput(
            "amount must be greater than zero"
        ))
    ));
    Ok(())
}
