use crate::api::{self, error::ApiConversionError};
use solana_pubkey::Pubkey;
use solana_signature::Signature;

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

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RampOrderStatus {
    Creating,
    CreationUncertain,
    AwaitingCustomer,
    AwaitingTransfer,
    Processing,
    Settling,
    Settled,
    Declined,
    Cancelled,
    Failed,
    Refunded,
}

#[derive(Clone)]
pub struct RampDeposit {
    pub address: Pubkey,
    pub amount: CryptoAmount,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RampTransferState {
    Prepared,
    Submitted { signature: Signature },
    Landed { signature: Signature },
    Failed { signature: Signature },
    Expired,
}

#[derive(Clone)]
pub struct RampTransfer {
    pub transfer_id: String,
    pub state: RampTransferState,
    pub expires_at: String,
}

#[derive(Clone)]
pub struct RampBuyOrder {
    pub id: String,
    pub status: RampOrderStatus,
    pub created_at: String,
    pub updated_at: String,
    pub quote: RampBuyQuote,
    pub launch_url: Option<String>,
}

#[derive(Clone)]
pub struct RampSellOrder {
    pub id: String,
    pub status: RampOrderStatus,
    pub created_at: String,
    pub updated_at: String,
    pub quote: RampSellQuote,
    pub launch_url: Option<String>,
    pub deposit: Option<RampDeposit>,
    pub transfer: Option<RampTransfer>,
}

#[derive(Clone)]
pub enum RampOrder {
    Buy(RampBuyOrder),
    Sell(RampSellOrder),
}

impl TryFrom<api::types::ramp_order::Transfer> for RampTransfer {
    type Error = ApiConversionError;

    fn try_from(value: api::types::ramp_order::Transfer) -> Result<Self, Self::Error> {
        if value.transfer_id.is_empty() || value.expires_at.is_empty() {
            return Err(ApiConversionError::Invalid("incomplete Ramp transfer"));
        }
        let signature = value.solana_signature;
        let state = match (value.state, signature) {
            (api::types::ramp_order::TransferState::Prepared, None) => RampTransferState::Prepared,
            (api::types::ramp_order::TransferState::Expired, None) => RampTransferState::Expired,
            (api::types::ramp_order::TransferState::Submitted, Some(signature)) => {
                RampTransferState::Submitted { signature }
            }
            (api::types::ramp_order::TransferState::Landed, Some(signature)) => {
                RampTransferState::Landed { signature }
            }
            (api::types::ramp_order::TransferState::Failed, Some(signature)) => {
                RampTransferState::Failed { signature }
            }
            _ => {
                return Err(ApiConversionError::Invalid(
                    "Ramp transfer state and signature disagree",
                ))
            }
        };
        Ok(Self {
            transfer_id: value.transfer_id,
            state,
            expires_at: value.expires_at,
        })
    }
}

impl TryFrom<api::types::ramp_order::Order> for RampOrder {
    type Error = ApiConversionError;

    fn try_from(value: api::types::ramp_order::Order) -> Result<Self, Self::Error> {
        if value.id.is_empty() || value.created_at.is_empty() || value.updated_at.is_empty() {
            return Err(ApiConversionError::Invalid("incomplete Ramp order"));
        }
        match (value.buy, value.sell) {
            (Some(details), None) => Ok(Self::Buy(RampBuyOrder {
                id: value.id,
                status: value.status.into(),
                created_at: value.created_at,
                updated_at: value.updated_at,
                quote: details.quote.into(),
                launch_url: details.launch_url.filter(|url| !url.is_empty()),
            })),
            (None, Some(details)) => Ok(Self::Sell(RampSellOrder {
                id: value.id,
                status: value.status.into(),
                created_at: value.created_at,
                updated_at: value.updated_at,
                quote: details.quote.into(),
                launch_url: details.launch_url.filter(|url| !url.is_empty()),
                deposit: details.deposit.map(Into::into),
                transfer: details.transfer.map(TryInto::try_into).transpose()?,
            })),
            _ => Err(ApiConversionError::Invalid(
                "Ramp order requires exactly one direction",
            )),
        }
    }
}

impl From<api::types::ramp_order::OrderStatus> for RampOrderStatus {
    fn from(value: api::types::ramp_order::OrderStatus) -> Self {
        match value {
            api::types::ramp_order::OrderStatus::Creating => Self::Creating,
            api::types::ramp_order::OrderStatus::CreationUncertain => Self::CreationUncertain,
            api::types::ramp_order::OrderStatus::AwaitingCustomer => Self::AwaitingCustomer,
            api::types::ramp_order::OrderStatus::AwaitingTransfer => Self::AwaitingTransfer,
            api::types::ramp_order::OrderStatus::Processing => Self::Processing,
            api::types::ramp_order::OrderStatus::Settling => Self::Settling,
            api::types::ramp_order::OrderStatus::Settled => Self::Settled,
            api::types::ramp_order::OrderStatus::Declined => Self::Declined,
            api::types::ramp_order::OrderStatus::Cancelled => Self::Cancelled,
            api::types::ramp_order::OrderStatus::Failed => Self::Failed,
            api::types::ramp_order::OrderStatus::Refunded => Self::Refunded,
        }
    }
}
impl From<api::types::ramp_order::Deposit> for RampDeposit {
    fn from(value: api::types::ramp_order::Deposit) -> Self {
        Self {
            address: value.address,
            amount: value.amount.into(),
        }
    }
}
