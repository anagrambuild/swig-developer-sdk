use super::{
    network::Network,
    ramp::{Environment, Location, OrderRequest, Route},
};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrderContext {
    pub customer_id: String,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_config_address: Pubkey,
    pub network: Network,
    pub location: Location,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateOrderRequest {
    pub request_id: String,
    pub configuration_id: String,
    pub environment: Environment,
    pub context: OrderContext,
    pub route: Route,
    #[serde(flatten)]
    pub order: OrderRequest,
}
