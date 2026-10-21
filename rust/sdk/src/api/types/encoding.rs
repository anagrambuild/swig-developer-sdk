use serde::{Deserialize, Serialize};

use crate::api::error::ApiConversionError;

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum EnumValue {
    Code(u32),
    Name(String),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "EnumValue")]
pub(crate) enum SignatureScheme {
    #[serde(rename = "AUTHORITY_SIGNATURE_SCHEME_SECP256R1")]
    Secp256r1,
    #[serde(rename = "AUTHORITY_SIGNATURE_SCHEME_SECP256K1")]
    Secp256k1,
}

impl TryFrom<EnumValue> for SignatureScheme {
    type Error = ApiConversionError;

    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(1) => Ok(Self::Secp256r1),
            EnumValue::Name(name)
                if matches!(
                    name.as_str(),
                    "AUTHORITY_SIGNATURE_SCHEME_SECP256R1" | "secp256r1"
                ) =>
            {
                Ok(Self::Secp256r1)
            }
            EnumValue::Code(2) => Ok(Self::Secp256k1),
            EnumValue::Name(name)
                if matches!(
                    name.as_str(),
                    "AUTHORITY_SIGNATURE_SCHEME_SECP256K1" | "secp256k1"
                ) =>
            {
                Ok(Self::Secp256k1)
            }
            _ => Err(ApiConversionError::Invalid(
                "unsupported authority signature scheme",
            )),
        }
    }
}
