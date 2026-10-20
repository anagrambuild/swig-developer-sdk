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

use solana_signature::Signature;
#[derive(Clone)]
pub enum TransactionDirection {
    Inflow,
    Outflow,
}

#[derive(Clone)]
pub enum AssetTransfer {
    Sol {
        lamports: u64,
        ui_amount: f64,
        usd_price: f64,
        usd_value: f64,
    },
    Token {
        token_account_address: Pubkey,
        mint_address: Pubkey,
        token_program: Pubkey,
        symbol: String,
        name: String,
        amount_raw: u64,
        decimals: u32,
        ui_amount: f64,
        usd_price: f64,
        usd_value: f64,
    },
}

#[derive(Clone)]
pub struct WalletTransaction {
    pub transaction_signature: Signature,
    pub slot: u64,
    pub block_time: Option<String>,
    pub owner_address: Pubkey,
    pub wallet_address: Pubkey,
    pub is_subaccount: bool,
    pub direction: TransactionDirection,
    pub transfer: AssetTransfer,
}

#[derive(Clone)]
pub struct WalletTransactionList {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub transactions: Vec<WalletTransaction>,
}

use crate::api::{
    error::ApiConversionError,
    types::assets::{Asset, AssetKind},
};
pub(crate) enum NormalizedAsset {
    Sol(SolBalance),
    Token(TokenBalance),
}
impl TryFrom<Asset> for NormalizedAsset {
    type Error = ApiConversionError;
    fn try_from(value: Asset) -> Result<Self, Self::Error> {
        if !value.ui_amount.is_finite()
            || !value.usd_price.is_finite()
            || !value.usd_value.is_finite()
        {
            return Err(ApiConversionError::Invalid("invalid asset valuation"));
        }
        match value.asset_kind {
            Some(AssetKind::NativeSol)
                if value.token_program == 0 && value.mint_address.is_empty() =>
            {
                Ok(Self::Sol(SolBalance {
                    lamports: value.amount_raw,
                    ui_amount: value.ui_amount,
                    usd_price: value.usd_price,
                    usd_value: value.usd_value,
                }))
            }
            Some(AssetKind::Token) | Some(AssetKind::Unspecified) | None => {
                let token_program = match value.token_program {
                    1 => solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    2 => solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
                    _ => {
                        return Err(ApiConversionError::Invalid(
                            "unclassifiable asset or unsupported token program",
                        ))
                    }
                };
                let mint_address = value
                    .mint_address
                    .parse()
                    .map_err(|_| ApiConversionError::Invalid("invalid token mint"))?;
                Ok(Self::Token(TokenBalance {
                    mint_address,
                    token_program,
                    symbol: value.token_symbol,
                    name: value.token_name,
                    decimals: value.decimals,
                    amount_raw: value.amount_raw,
                    ui_amount: value.ui_amount,
                    usd_price: value.usd_price,
                    usd_value: value.usd_value,
                }))
            }
            _ => Err(ApiConversionError::Invalid("inconsistent native SOL asset")),
        }
    }
}
