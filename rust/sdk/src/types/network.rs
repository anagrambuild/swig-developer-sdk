/// The Solana network selected by a hosted client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Network {
    Devnet,
    Mainnet,
}

use crate::api::{error::ApiConversionError, types::network::Network as ApiNetwork};
impl TryFrom<ApiNetwork> for Network {
    type Error = ApiConversionError;

    fn try_from(value: ApiNetwork) -> Result<Self, Self::Error> {
        Ok(match value {
            ApiNetwork::Devnet => Self::Devnet,
            ApiNetwork::Mainnet => Self::Mainnet,
        })
    }
}

impl From<Network> for ApiNetwork {
    fn from(value: Network) -> Self {
        match value {
            Network::Devnet => Self::Devnet,
            Network::Mainnet => Self::Mainnet,
        }
    }
}
