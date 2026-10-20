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
