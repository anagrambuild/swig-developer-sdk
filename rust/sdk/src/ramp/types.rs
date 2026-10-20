use solana_pubkey::Pubkey;

#[derive(Clone, Eq, PartialEq)]
pub enum CryptoAsset {
    Sol,
    Token { mint: Pubkey },
}

impl From<crate::api::types::ramp::CryptoAsset> for CryptoAsset {
    fn from(value: crate::api::types::ramp::CryptoAsset) -> Self {
        match value {
            crate::api::types::ramp::CryptoAsset::Sol {} => Self::Sol,
            crate::api::types::ramp::CryptoAsset::Token { mint } => Self::Token { mint },
        }
    }
}

impl From<CryptoAsset> for crate::api::types::ramp::CryptoAsset {
    fn from(value: CryptoAsset) -> Self {
        match value {
            CryptoAsset::Sol => Self::Sol {},
            CryptoAsset::Token { mint } => Self::Token { mint },
        }
    }
}

#[derive(Clone)]
pub struct FiatAmount {
    pub currency_code: String,

    pub minor_units: u64,
}

#[derive(Clone)]
pub struct CryptoAmount {
    pub asset: CryptoAsset,

    pub base_units: u64,
}

#[derive(Clone)]
pub struct RampRoute {
    pub provider: String,
    pub payment_method: String,
}

#[derive(Clone)]
pub struct RampLocation {
    pub country_code: String,
    pub subdivision_code: Option<String>,
}

#[derive(Clone)]
pub enum RampOrderRequest {
    Buy {
        spend: FiatAmount,
        receive: CryptoAsset,
    },
    Sell {
        sell: CryptoAmount,
        receive_fiat_currency_code: String,
    },
}

#[derive(Clone)]
pub struct RampBuyQuote {
    pub spend: FiatAmount,
    pub receive: CryptoAmount,
    pub total_fee: FiatAmount,
    pub exchange_rate: String,
}

#[derive(Clone)]
pub struct RampSellQuote {
    pub sell: CryptoAmount,
    pub receive: FiatAmount,
    pub total_fee: FiatAmount,
    pub exchange_rate: String,
}

impl From<crate::api::types::ramp::FiatAmount> for FiatAmount {
    fn from(value: crate::api::types::ramp::FiatAmount) -> Self {
        Self {
            currency_code: value.currency_code,
            minor_units: value.minor_units,
        }
    }
}

impl From<FiatAmount> for crate::api::types::ramp::FiatAmount {
    fn from(value: FiatAmount) -> Self {
        Self {
            currency_code: value.currency_code,
            minor_units: value.minor_units,
        }
    }
}

impl From<crate::api::types::ramp::CryptoAmount> for CryptoAmount {
    fn from(value: crate::api::types::ramp::CryptoAmount) -> Self {
        Self {
            asset: value.asset.into(),
            base_units: value.base_units,
        }
    }
}

impl From<CryptoAmount> for crate::api::types::ramp::CryptoAmount {
    fn from(value: CryptoAmount) -> Self {
        Self {
            asset: value.asset.into(),
            base_units: value.base_units,
        }
    }
}

impl From<crate::api::types::ramp::Route> for RampRoute {
    fn from(value: crate::api::types::ramp::Route) -> Self {
        Self {
            provider: value.provider,
            payment_method: value.payment_method,
        }
    }
}

impl From<RampRoute> for crate::api::types::ramp::Route {
    fn from(value: RampRoute) -> Self {
        Self {
            provider: value.provider,
            payment_method: value.payment_method,
        }
    }
}

impl From<crate::api::types::ramp::Location> for RampLocation {
    fn from(value: crate::api::types::ramp::Location) -> Self {
        Self {
            country_code: value.country_code,
            subdivision_code: value.subdivision_code,
        }
    }
}

impl From<RampLocation> for crate::api::types::ramp::Location {
    fn from(value: RampLocation) -> Self {
        Self {
            country_code: value.country_code,
            subdivision_code: value.subdivision_code,
        }
    }
}

impl From<crate::api::types::ramp_quote::BuyQuote> for RampBuyQuote {
    fn from(value: crate::api::types::ramp_quote::BuyQuote) -> Self {
        Self {
            spend: value.spend.into(),
            receive: value.receive.into(),
            total_fee: value.total_fee.into(),
            exchange_rate: value.exchange_rate,
        }
    }
}

impl From<RampBuyQuote> for crate::api::types::ramp_quote::BuyQuote {
    fn from(value: RampBuyQuote) -> Self {
        Self {
            spend: value.spend.into(),
            receive: value.receive.into(),
            total_fee: value.total_fee.into(),
            exchange_rate: value.exchange_rate,
        }
    }
}

impl From<crate::api::types::ramp_quote::SellQuote> for RampSellQuote {
    fn from(value: crate::api::types::ramp_quote::SellQuote) -> Self {
        Self {
            sell: value.sell.into(),
            receive: value.receive.into(),
            total_fee: value.total_fee.into(),
            exchange_rate: value.exchange_rate,
        }
    }
}

impl From<RampSellQuote> for crate::api::types::ramp_quote::SellQuote {
    fn from(value: RampSellQuote) -> Self {
        Self {
            sell: value.sell.into(),
            receive: value.receive.into(),
            total_fee: value.total_fee.into(),
            exchange_rate: value.exchange_rate,
        }
    }
}

impl From<RampOrderRequest> for crate::api::types::ramp::OrderRequest {
    fn from(value: RampOrderRequest) -> Self {
        match value {
            RampOrderRequest::Buy { spend, receive } => Self::Buy {
                spend: spend.into(),
                receive: receive.into(),
            },
            RampOrderRequest::Sell {
                sell,
                receive_fiat_currency_code,
            } => Self::Sell {
                sell: sell.into(),
                receive_fiat_currency_code,
            },
        }
    }
}
