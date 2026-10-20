use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct SwigUsdBalance {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub usd_value: f64,
}

#[derive(Clone)]
pub struct SolBalance {
    pub lamports: u64,
    pub ui_amount: f64,
    pub usd_price: f64,
    pub usd_value: f64,
}

#[derive(Clone)]
pub struct TokenBalance {
    pub mint_address: Pubkey,
    pub token_program: Pubkey,
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub amount_raw: u64,
    pub ui_amount: f64,
    pub usd_price: f64,
    pub usd_value: f64,
}

#[derive(Clone)]
pub struct WalletBalanceSummary {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub sol_balance: Option<SolBalance>,
    pub token_balances: BTreeMap<Pubkey, TokenBalance>,
    pub total_usd_value: f64,
}
