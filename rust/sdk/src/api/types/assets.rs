use crate::api::error::ApiConversionError;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

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
    type Error = ApiConversionError;
    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(0) => Ok(Self::Unspecified),
            EnumValue::Code(1) => Ok(Self::Token),
            EnumValue::Code(2) => Ok(Self::NativeSol),
            EnumValue::Name(name) => match name.as_str() {
                "ASSET_KIND_UNSPECIFIED" | "unspecified" => Ok(Self::Unspecified),
                "ASSET_KIND_TOKEN" | "token" => Ok(Self::Token),
                "ASSET_KIND_NATIVE_SOL" | "native-sol" => Ok(Self::NativeSol),
                _ => Err(ApiConversionError::Invalid("unknown asset kind")),
            },
            _ => Err(ApiConversionError::Invalid("unknown asset kind")),
        }
    }
}
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsdBalance {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "swig_config_address")]
    pub swig_config_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "wallet_address")]
    pub wallet_address: Pubkey,
    #[serde(alias = "usd_value")]
    pub usd_value: f64,
}
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Asset {
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
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BalanceSummary {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "swig_config_address")]
    pub swig_config_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "wallet_address")]
    pub wallet_address: Pubkey,
    #[serde(default)]
    pub balances: Vec<Asset>,
    #[serde(alias = "total_usd_value")]
    pub total_usd_value: f64,
}
