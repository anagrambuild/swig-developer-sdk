use super::ramp::{CryptoAsset, Direction, Environment};
use serde::{Deserialize, Serialize};

pub(crate) struct GetOptionsRequest {
    pub configuration_id: String,
    pub environment: Environment,
    pub direction: Direction,
    pub country_code: Option<String>,
    pub fiat_currency_code: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Subdivision {
    pub code: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Country {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub subdivisions: Vec<Subdivision>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FiatCurrencyOption {
    #[serde(alias = "currency_code")]
    pub currency_code: String,
    pub exponent: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssetOption {
    pub asset: CryptoAsset,
    pub name: String,
    #[serde(alias = "icon_url", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub decimals: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Options {
    #[serde(default)]
    pub countries: Vec<Country>,
    #[serde(default, alias = "fiat_currencies")]
    pub fiat_currencies: Vec<FiatCurrencyOption>,
    #[serde(default, alias = "payment_methods")]
    pub payment_methods: Vec<String>,
    #[serde(default)]
    pub assets: Vec<AssetOption>,
}
