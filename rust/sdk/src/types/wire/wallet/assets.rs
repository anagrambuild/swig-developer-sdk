use crate::types::wire::WireError;
use crate::{SolBalance, SwigUsdBalance, TokenBalance, WalletBalanceSummary};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum EnumValue {
    Code(u32),
    Name(String),
}
#[derive(Deserialize)]
#[serde(try_from = "EnumValue")]
pub(crate) enum AssetKind {
    Unspecified,
    Token,
    NativeSol,
}
impl TryFrom<EnumValue> for AssetKind {
    type Error = WireError;
    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(0) => Ok(Self::Unspecified),
            EnumValue::Code(1) => Ok(Self::Token),
            EnumValue::Code(2) => Ok(Self::NativeSol),
            EnumValue::Name(name) => match name.as_str() {
                "ASSET_KIND_UNSPECIFIED" | "unspecified" => Ok(Self::Unspecified),
                "ASSET_KIND_TOKEN" | "token" => Ok(Self::Token),
                "ASSET_KIND_NATIVE_SOL" | "native-sol" => Ok(Self::NativeSol),
                _ => Err(WireError::Invalid("unknown asset kind")),
            },
            _ => Err(WireError::Invalid("unknown asset kind")),
        }
    }
}
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsdBalanceWire {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "swig_config_address")]
    pub swig_config_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "wallet_address")]
    pub wallet_address: Pubkey,
    #[serde(alias = "usd_value")]
    pub usd_value: f64,
}
impl TryFrom<UsdBalanceWire> for SwigUsdBalance {
    type Error = WireError;
    fn try_from(value: UsdBalanceWire) -> Result<Self, Self::Error> {
        if !value.usd_value.is_finite() {
            return Err(WireError::Invalid("invalid USD value"));
        }
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
            usd_value: value.usd_value,
        })
    }
}
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssetWire {
    #[serde(default, alias = "mint_address")]
    pub mint_address: String,
    #[serde(default, alias = "token_program")]
    pub token_program: u32,
    #[serde(default, alias = "token_symbol")]
    pub token_symbol: String,
    #[serde(default, alias = "token_name")]
    pub token_name: String,
    pub decimals: u32,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "amount_raw")]
    pub amount_raw: u64,
    #[serde(alias = "ui_amount")]
    pub ui_amount: f64,
    #[serde(alias = "usd_price")]
    pub usd_price: f64,
    #[serde(alias = "usd_value")]
    pub usd_value: f64,
    #[serde(alias = "asset_kind")]
    pub asset_kind: Option<AssetKind>,
}
enum NormalizedAsset {
    Sol(SolBalance),
    Token(TokenBalance),
}
impl TryFrom<AssetWire> for NormalizedAsset {
    type Error = WireError;
    fn try_from(value: AssetWire) -> Result<Self, Self::Error> {
        if !value.ui_amount.is_finite()
            || !value.usd_price.is_finite()
            || !value.usd_value.is_finite()
        {
            return Err(WireError::Invalid("invalid asset valuation"));
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
                        return Err(WireError::Invalid(
                            "unclassifiable asset or unsupported token program",
                        ))
                    }
                };
                let mint_address = value
                    .mint_address
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid token mint"))?;
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
            _ => Err(WireError::Invalid("inconsistent native SOL asset")),
        }
    }
}
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BalanceSummaryWire {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "swig_config_address")]
    pub swig_config_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "wallet_address")]
    pub wallet_address: Pubkey,
    #[serde(default)]
    pub balances: Vec<AssetWire>,
    #[serde(alias = "total_usd_value")]
    pub total_usd_value: f64,
}
impl TryFrom<BalanceSummaryWire> for WalletBalanceSummary {
    type Error = WireError;
    fn try_from(value: BalanceSummaryWire) -> Result<Self, Self::Error> {
        if !value.total_usd_value.is_finite() {
            return Err(WireError::Invalid("invalid total USD value"));
        }
        let mut sol_balance = None;
        let mut token_balances = BTreeMap::new();
        for row in value.balances {
            match NormalizedAsset::try_from(row)? {
                NormalizedAsset::Sol(balance) => {
                    if sol_balance.replace(balance).is_some() {
                        return Err(WireError::Invalid("duplicate native SOL balance"));
                    }
                }
                NormalizedAsset::Token(balance) => {
                    if token_balances
                        .insert(balance.mint_address, balance)
                        .is_some()
                    {
                        return Err(WireError::Invalid("duplicate token mint balance"));
                    }
                }
            }
        }
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
            sol_balance,
            token_balances,
            total_usd_value: value.total_usd_value,
        })
    }
}
