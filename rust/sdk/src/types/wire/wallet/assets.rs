use crate::types::wire::WireError;
use crate::{
    AssetTransfer, SolBalance, SwigUsdBalance, TokenBalance, TransactionDirection,
    WalletBalanceSummary, WalletTransaction, WalletTransactionList,
};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
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
#[derive(Deserialize)]
#[serde(try_from = "EnumValue")]
pub(crate) struct DirectionWire(pub TransactionDirection);
impl TryFrom<EnumValue> for DirectionWire {
    type Error = WireError;
    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(1) => Ok(Self(TransactionDirection::Inflow)),
            EnumValue::Code(2) => Ok(Self(TransactionDirection::Outflow)),
            EnumValue::Name(name) => match name.as_str() {
                "SWIG_TOKEN_TRANSACTION_DIRECTION_INFLOW" | "inflow" => {
                    Ok(Self(TransactionDirection::Inflow))
                }
                "SWIG_TOKEN_TRANSACTION_DIRECTION_OUTFLOW" | "outflow" => {
                    Ok(Self(TransactionDirection::Outflow))
                }
                _ => Err(WireError::Invalid("unknown transaction direction")),
            },
            _ => Err(WireError::Invalid("unknown transaction direction")),
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
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WalletTransactionWire {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "transaction_signature")]
    pub transaction_signature: Signature,
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    #[serde(alias = "block_time")]
    pub block_time: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "owner_address")]
    pub owner_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "wallet_address")]
    pub wallet_address: Pubkey,
    #[serde(default, alias = "is_subaccount")]
    pub is_subaccount: bool,
    #[serde(default, alias = "token_account_address")]
    pub token_account_address: String,
    pub direction: DirectionWire,
    #[serde(flatten)]
    pub asset: AssetWire,
}
impl TryFrom<WalletTransactionWire> for WalletTransaction {
    type Error = WireError;
    fn try_from(value: WalletTransactionWire) -> Result<Self, Self::Error> {
        let transfer = match NormalizedAsset::try_from(value.asset)? {
            NormalizedAsset::Sol(balance) => AssetTransfer::Sol {
                lamports: balance.lamports,
                ui_amount: balance.ui_amount,
                usd_price: balance.usd_price,
                usd_value: balance.usd_value,
            },
            NormalizedAsset::Token(balance) => AssetTransfer::Token {
                token_account_address: value
                    .token_account_address
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid token account address"))?,
                mint_address: balance.mint_address,
                token_program: balance.token_program,
                symbol: balance.symbol,
                name: balance.name,
                amount_raw: balance.amount_raw,
                decimals: balance.decimals,
                ui_amount: balance.ui_amount,
                usd_price: balance.usd_price,
                usd_value: balance.usd_value,
            },
        };
        Ok(Self {
            transaction_signature: value.transaction_signature,
            slot: value.slot,
            block_time: value.block_time,
            owner_address: value.owner_address,
            wallet_address: value.wallet_address,
            is_subaccount: value.is_subaccount,
            direction: value.direction.0,
            transfer,
        })
    }
}
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionListWire {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "swig_config_address")]
    pub swig_config_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "wallet_address")]
    pub wallet_address: Pubkey,
    #[serde(default)]
    pub transactions: Vec<WalletTransactionWire>,
}
impl TryFrom<TransactionListWire> for WalletTransactionList {
    type Error = WireError;
    fn try_from(value: TransactionListWire) -> Result<Self, Self::Error> {
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
            transactions: value
                .transactions
                .into_iter()
                .map(WalletTransaction::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}
