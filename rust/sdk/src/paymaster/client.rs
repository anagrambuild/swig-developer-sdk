use std::sync::Arc;

use crate::{http::HttpClient, ClientConfig, ClientError, Network};

#[derive(Clone)]
pub struct PaymasterClient {
    pub(crate) http: Arc<HttpClient>,
    pub(crate) network: Network,
}

impl PaymasterClient {
    /// Validates configuration without making a hosted request.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let ClientConfig { http, network } = config;
        Ok(Self {
            http: Arc::new(HttpClient::new(http)?),
            network,
        })
    }
}
