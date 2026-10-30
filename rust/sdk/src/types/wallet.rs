use solana_pubkey::Pubkey;

#[derive(Clone)]
pub struct WalletAddressInfo {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
}

use crate::api::{
    error::ApiConversionError, types::transaction::WalletAddressInfo as ApiWalletAddressInfo,
};
impl TryFrom<ApiWalletAddressInfo> for WalletAddressInfo {
    type Error = ApiConversionError;

    fn try_from(value: ApiWalletAddressInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
        })
    }
}

impl From<WalletAddressInfo> for ApiWalletAddressInfo {
    fn from(value: WalletAddressInfo) -> Self {
        Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
        }
    }
}
