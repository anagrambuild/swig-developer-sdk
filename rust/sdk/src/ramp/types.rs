use crate::types::wire::ramp::{CryptoAssetWire, RampOrderWire, RampTransferWire};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
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
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub enum RampOrderStatus {
    #[serde(rename = "RAMP_ORDER_STATUS_CREATING")]
    Creating,
    #[serde(rename = "RAMP_ORDER_STATUS_CREATION_UNCERTAIN")]
    CreationUncertain,
    #[serde(rename = "RAMP_ORDER_STATUS_AWAITING_CUSTOMER")]
    AwaitingCustomer,
    #[serde(rename = "RAMP_ORDER_STATUS_AWAITING_TRANSFER")]
    AwaitingTransfer,
    #[serde(rename = "RAMP_ORDER_STATUS_PROCESSING")]
    Processing,
    #[serde(rename = "RAMP_ORDER_STATUS_SETTLING")]
    Settling,
    #[serde(rename = "RAMP_ORDER_STATUS_SETTLED")]
    Settled,
    #[serde(rename = "RAMP_ORDER_STATUS_DECLINED")]
    Declined,
    #[serde(rename = "RAMP_ORDER_STATUS_CANCELLED")]
    Cancelled,
    #[serde(rename = "RAMP_ORDER_STATUS_FAILED")]
    Failed,
    #[serde(rename = "RAMP_ORDER_STATUS_REFUNDED")]
    Refunded,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct RampDeposit {
    #[serde_as(as = "DisplayFromStr")]
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "RampTransferWire", into = "RampTransferWire")]
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "RampOrderWire", into = "RampOrderWire")]
pub enum RampOrder {
    Buy(RampBuyOrder),
    Sell(RampSellOrder),
}

impl TryFrom<String> for RampOrderStatus {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "RAMP_ORDER_STATUS_CREATING" => Ok(Self::Creating),
            "RAMP_ORDER_STATUS_CREATION_UNCERTAIN" => Ok(Self::CreationUncertain),
            "RAMP_ORDER_STATUS_AWAITING_CUSTOMER" => Ok(Self::AwaitingCustomer),
            "RAMP_ORDER_STATUS_AWAITING_TRANSFER" => Ok(Self::AwaitingTransfer),
            "RAMP_ORDER_STATUS_PROCESSING" => Ok(Self::Processing),
            "RAMP_ORDER_STATUS_SETTLING" => Ok(Self::Settling),
            "RAMP_ORDER_STATUS_SETTLED" => Ok(Self::Settled),
            "RAMP_ORDER_STATUS_DECLINED" => Ok(Self::Declined),
            "RAMP_ORDER_STATUS_CANCELLED" => Ok(Self::Cancelled),
            "RAMP_ORDER_STATUS_FAILED" => Ok(Self::Failed),
            "RAMP_ORDER_STATUS_REFUNDED" => Ok(Self::Refunded),
            _ => Err("invalid Ramp order status"),
        }
    }
}
