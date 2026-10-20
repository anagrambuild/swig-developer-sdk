use swig_developer_sdk::{ClientConfig, ClientError, Network, PaymasterClient};

#[test]
fn paymaster_constructor_validates_configuration_and_can_be_cloned() {
    assert!(matches!(
        PaymasterClient::new(ClientConfig::new(String::new(), Network::Devnet)),
        Err(ClientError::InvalidApiKey)
    ));
    let client = PaymasterClient::new(ClientConfig::new("test-key".into(), Network::Mainnet));
    assert!(client.is_ok());
    if let Ok(client) = client {
        let _shared = client.clone();
    }
}
