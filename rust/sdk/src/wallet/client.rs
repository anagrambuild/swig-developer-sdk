use std::sync::Arc;

use crate::{http::HttpClient, ClientConfig, ClientError, Network};

/// A reusable hosted wallet client. Clones share the connection pool.
#[derive(Clone)]
pub struct WalletClient {
    pub(crate) http: Arc<HttpClient>,
    #[allow(
        dead_code,
        reason = "Consumed by wallet operations in the next stacked change"
    )]
    pub(crate) network: Network,
}

impl WalletClient {
    /// Validates configuration without making a hosted request.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let ClientConfig { http, network } = config;
        Ok(Self {
            http: Arc::new(HttpClient::new(http)?),
            network,
        })
    }
}
