use serde::{Deserialize, Serialize};

use crate::api::error::ApiConversionError;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(try_from = "NetworkValue")]
pub(crate) enum Network {
    #[serde(rename = "NETWORK_DEVNET")]
    Devnet,
    #[serde(rename = "NETWORK_MAINNET")]
    Mainnet,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NetworkValue {
    Code(u32),
    Name(String),
}

impl TryFrom<NetworkValue> for Network {
    type Error = ApiConversionError;

    fn try_from(value: NetworkValue) -> Result<Self, Self::Error> {
        match value {
            NetworkValue::Code(1) => Ok(Self::Devnet),
            NetworkValue::Code(2) => Ok(Self::Mainnet),
            NetworkValue::Name(name) if name == "NETWORK_DEVNET" => Ok(Self::Devnet),
            NetworkValue::Name(name) if name == "NETWORK_MAINNET" => Ok(Self::Mainnet),
            _ => Err(ApiConversionError::Invalid("invalid network")),
        }
    }
}
