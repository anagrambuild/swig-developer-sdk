use super::WireError;
use crate::CryptoAsset;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum CryptoAssetWire {
    Sol {},
    Token { mint: String },
}

impl TryFrom<CryptoAssetWire> for CryptoAsset {
    type Error = WireError;

    fn try_from(value: CryptoAssetWire) -> Result<Self, Self::Error> {
        match value {
            CryptoAssetWire::Sol {} => Ok(Self::Sol),
            CryptoAssetWire::Token { mint } => Ok(Self::Token {
                mint: mint
                    .parse()
                    .map_err(|_| WireError::Invalid("invalid Ramp token mint"))?,
            }),
        }
    }
}

impl From<CryptoAsset> for CryptoAssetWire {
    fn from(value: CryptoAsset) -> Self {
        match value {
            CryptoAsset::Sol => Self::Sol {},
            CryptoAsset::Token { mint } => Self::Token {
                mint: mint.to_string(),
            },
        }
    }
}
