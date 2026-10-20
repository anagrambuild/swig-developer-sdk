use crate::types::wire::{
    permission::PolicyPermissionWire, policy::PolicyAuthorityWire, WireError,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PolicyWire {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub authority: Option<PolicyAuthorityWire>,
    #[serde(default)]
    pub actions: Vec<PolicyPermissionWire>,
    #[serde(default, alias = "guardian_enabled")]
    pub guardian_enabled: bool,
    #[serde(alias = "initial_key_source")]
    pub initial_key_source: Option<KeySourceWire>,
    #[serde(alias = "guardian_authority")]
    pub guardian_authority: Option<PolicyAuthorityWire>,
    #[serde(alias = "guardian_key_source")]
    pub guardian_key_source: Option<KeySourceWire>,
    #[serde(default, alias = "guardian_delay_seconds")]
    pub guardian_delay_seconds: u32,
}

#[derive(Deserialize)]
#[serde(try_from = "KeySourceValue")]
pub(crate) enum KeySourceWire {
    Unspecified,
    AtCreation,
    Stored,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum KeySourceValue {
    Code(u32),
    Name(String),
}
impl TryFrom<KeySourceValue> for KeySourceWire {
    type Error = WireError;
    fn try_from(value: KeySourceValue) -> Result<Self, Self::Error> {
        match value {
            KeySourceValue::Code(0) => Ok(Self::Unspecified),
            KeySourceValue::Code(1) => Ok(Self::AtCreation),
            KeySourceValue::Code(2) => Ok(Self::Stored),
            KeySourceValue::Name(name) => match name.as_str() {
                "WALLET_POLICY_KEY_SOURCE_UNSPECIFIED" => Ok(Self::Unspecified),
                "WALLET_POLICY_KEY_SOURCE_AT_CREATION" => Ok(Self::AtCreation),
                "WALLET_POLICY_KEY_SOURCE_STORED" => Ok(Self::Stored),
                _ => Err(WireError::Invalid("unknown policy key source")),
            },
            _ => Err(WireError::Invalid("unknown policy key source")),
        }
    }
}
