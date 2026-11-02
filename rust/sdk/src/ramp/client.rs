use crate::{http::HttpClient, ClientConfig, ClientError, Network, RampConfig};
use std::sync::Arc;
#[derive(Clone)]
pub struct RampClient {
    pub(crate) network: Network,
    pub(crate) http: Arc<HttpClient>,
    pub(crate) ramp_config: Option<RampConfig>,
}
impl RampClient {
    pub fn new(
        client_config: ClientConfig,
        ramp_config: Option<RampConfig>,
    ) -> Result<Self, ClientError> {
        if ramp_config
            .as_ref()
            .is_some_and(|config| config.configuration_id.trim().is_empty())
        {
            return Err(ClientError::MissingRampConfigurationId);
        }
        Ok(Self {
            network: client_config.network,
            http: Arc::new(HttpClient::new(client_config.http)?),
            ramp_config,
        })
    }
}
