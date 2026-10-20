use crate::{
    hosted_support::config, ClientError, CryptoAmount, CryptoAsset, Network, RampClient,
    RampConfig, RampEnvironment, RampLocation, RampOrderRequest,
};
use serde_json::{json, Value};
pub(crate) fn client(network: Network) -> Result<RampClient, ClientError> {
    let mut config = config();
    config.network = network;
    RampClient::new(
        config,
        Some(RampConfig {
            configuration_id: "configuration".into(),
            environment: RampEnvironment::Sandbox,
        }),
    )
}
pub(crate) fn sell_request() -> RampOrderRequest {
    RampOrderRequest::Sell {
        sell: CryptoAmount {
            asset: CryptoAsset::Sol,
            base_units: u64::MAX,
        },
        receive_fiat_currency_code: "USD".into(),
    }
}

pub(crate) fn location() -> RampLocation {
    RampLocation {
        country_code: "US".into(),
        subdivision_code: Some("US-CA".into()),
    }
}

pub(crate) fn quote() -> Value {
    json!({
        "sell":{"asset":{"sol":{}},"baseUnits":"12"},
        "receive":{"currencyCode":"USD","minorUnits":"1"},
        "totalFee":{"currencyCode":"USD","minorUnits":"0"},
        "exchangeRate":"0.123"
    })
}
