use serde::{Deserialize, Serialize};

use super::WireError;

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum EnumValue {
    Code(u32),
    Name(String),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "EnumValue")]
pub(crate) enum SignatureSchemeWire {
    #[serde(rename = "AUTHORITY_SIGNATURE_SCHEME_SECP256R1")]
    Secp256r1,
    #[serde(rename = "AUTHORITY_SIGNATURE_SCHEME_SECP256K1")]
    Secp256k1,
}

impl TryFrom<EnumValue> for SignatureSchemeWire {
    type Error = WireError;

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
            _ => Err(WireError::Invalid("unsupported authority signature scheme")),
        }
    }
}

pub(crate) fn decode_hex32(value: &str) -> Result<[u8; 32], WireError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    let mut bytes = [0; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|_| WireError::Invalid("invalid 32-byte hex value"))?;
    Ok(bytes)
}
