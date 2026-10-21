use serde::{Deserialize, Serialize};

use crate::{types::wire::WireError, Network};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(try_from = "NetworkValue")]
pub(crate) enum NetworkWire {
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

impl TryFrom<NetworkValue> for NetworkWire {
    type Error = WireError;

    fn try_from(value: NetworkValue) -> Result<Self, Self::Error> {
        match value {
            NetworkValue::Code(1) => Ok(Self::Devnet),
            NetworkValue::Code(2) => Ok(Self::Mainnet),
            NetworkValue::Name(name) if name == "NETWORK_DEVNET" => Ok(Self::Devnet),
            NetworkValue::Name(name) if name == "NETWORK_MAINNET" => Ok(Self::Mainnet),
            _ => Err(WireError::Invalid("invalid network")),
        }
    }
}

impl TryFrom<NetworkWire> for Network {
    type Error = WireError;

    fn try_from(value: NetworkWire) -> Result<Self, Self::Error> {
        Ok(match value {
            NetworkWire::Devnet => Self::Devnet,
            NetworkWire::Mainnet => Self::Mainnet,
        })
    }
}

impl From<Network> for NetworkWire {
    fn from(value: Network) -> Self {
        match value {
            Network::Devnet => Self::Devnet,
            Network::Mainnet => Self::Mainnet,
        }
    }
}
