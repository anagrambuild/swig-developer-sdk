use crate::types::wire::ramp::CryptoAssetWire;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CryptoAssetWire", into = "CryptoAssetWire")]
pub enum CryptoAsset {
    Sol,
    Token { mint: Pubkey },
}
#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiatAmount {
    #[serde(alias = "currency_code")]
    pub currency_code: String,
    #[serde(alias = "minor_units")]
    #[serde_as(as = "DisplayFromStr")]
    pub minor_units: u64,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoAmount {
    pub asset: CryptoAsset,
    #[serde(alias = "base_units")]
    #[serde_as(as = "DisplayFromStr")]
    pub base_units: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampRoute {
    pub provider: String,
    #[serde(alias = "payment_method")]
    pub payment_method: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampLocation {
    #[serde(alias = "country_code")]
    pub country_code: String,
    #[serde(alias = "subdivision_code", skip_serializing_if = "Option::is_none")]
    pub subdivision_code: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RampOrderRequest {
    Buy {
        spend: FiatAmount,
        receive: CryptoAsset,
    },
    Sell {
        sell: CryptoAmount,
        #[serde(alias = "receive_fiat_currency_code")]
        receive_fiat_currency_code: String,
    },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampBuyQuote {
    pub spend: FiatAmount,
    pub receive: CryptoAmount,
    #[serde(alias = "total_fee")]
    pub total_fee: FiatAmount,
    #[serde(alias = "exchange_rate")]
    pub exchange_rate: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampSellQuote {
    pub sell: CryptoAmount,
    pub receive: FiatAmount,
    #[serde(alias = "total_fee")]
    pub total_fee: FiatAmount,
    #[serde(alias = "exchange_rate")]
    pub exchange_rate: String,
}
