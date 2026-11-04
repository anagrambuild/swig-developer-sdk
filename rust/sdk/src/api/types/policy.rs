use serde::Deserialize;

use crate::api::error::ApiConversionError;

use super::{authority::WalletAuthority, permission::PolicyPermission};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Policy {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(alias = "initial_key_source")]
    pub initial_key_source: Option<PolicyKeySource>,
    pub authority: Option<WalletAuthority>,
    #[serde(default)]
    pub actions: Vec<PolicyPermission>,
    #[serde(default, alias = "guardian_enabled")]
    pub guardian_enabled: bool,
    #[serde(alias = "guardian_key_source")]
    pub guardian_key_source: Option<PolicyKeySource>,
    #[serde(alias = "guardian_authority")]
    pub guardian_authority: Option<WalletAuthority>,
    #[serde(default, alias = "guardian_delay_seconds")]
    pub guardian_delay_seconds: u32,
}

#[derive(Deserialize)]
#[serde(try_from = "PolicyKeySourceValue")]
pub(crate) enum PolicyKeySource {
    Unspecified,
    AtCreation,
    Stored,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PolicyKeySourceValue {
    Code(u32),
    Name(String),
}

impl TryFrom<PolicyKeySourceValue> for PolicyKeySource {
    type Error = ApiConversionError;

    fn try_from(value: PolicyKeySourceValue) -> Result<Self, Self::Error> {
        match value {
            PolicyKeySourceValue::Code(0) => Ok(Self::Unspecified),
            PolicyKeySourceValue::Code(1) => Ok(Self::AtCreation),
            PolicyKeySourceValue::Code(2) => Ok(Self::Stored),
            PolicyKeySourceValue::Name(name) => match name.as_str() {
                "WALLET_POLICY_KEY_SOURCE_UNSPECIFIED" => Ok(Self::Unspecified),
                "WALLET_POLICY_KEY_SOURCE_AT_CREATION" => Ok(Self::AtCreation),
                "WALLET_POLICY_KEY_SOURCE_STORED" => Ok(Self::Stored),
                _ => Err(ApiConversionError::Invalid("unknown policy key source")),
            },
            _ => Err(ApiConversionError::Invalid("unknown policy key source")),
        }
    }
}
