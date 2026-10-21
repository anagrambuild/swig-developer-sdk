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
pub(crate) enum TransactionEncoding {
    #[serde(rename = "TRANSACTION_ENCODING_BASE64")]
    Base64,
}

impl TryFrom<EnumValue> for TransactionEncoding {
    type Error = ApiConversionError;

    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(1) => Ok(Self::Base64),
            EnumValue::Name(name)
                if matches!(name.as_str(), "TRANSACTION_ENCODING_BASE64" | "base64") =>
            {
                Ok(Self::Base64)
            }
            _ => Err(ApiConversionError::Invalid(
                "unsupported transaction encoding",
            )),
        }
    }
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

#[derive(Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(try_from = "EnumValue")]
pub(crate) enum PreparedTransactionKind {
    Unspecified,
    CreateSwigWallet,
    AddAuthority,
    ConfigureRecovery,
    CreateParticipantSet,
    X402Payment,
    SetRentClaimer,
}

impl TryFrom<EnumValue> for PreparedTransactionKind {
    type Error = ApiConversionError;
    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(0) => Ok(Self::Unspecified),
            EnumValue::Code(1) => Ok(Self::CreateSwigWallet),
            EnumValue::Code(2) => Ok(Self::AddAuthority),
            EnumValue::Code(3) => Ok(Self::ConfigureRecovery),
            EnumValue::Code(4) => Ok(Self::CreateParticipantSet),
            EnumValue::Code(5) => Ok(Self::X402Payment),
            EnumValue::Code(6) => Ok(Self::SetRentClaimer),
            EnumValue::Name(name) => match name.as_str() {
                "PREPARED_TRANSACTION_KIND_UNSPECIFIED" => Ok(Self::Unspecified),
                "PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET" | "create-swig-wallet" => {
                    Ok(Self::CreateSwigWallet)
                }
                "PREPARED_TRANSACTION_KIND_ADD_AUTHORITY" | "add-authority" => {
                    Ok(Self::AddAuthority)
                }
                "PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY" | "configure-recovery" => {
                    Ok(Self::ConfigureRecovery)
                }
                "PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET" | "create-participant-set" => {
                    Ok(Self::CreateParticipantSet)
                }
                "PREPARED_TRANSACTION_KIND_X402_PAYMENT" | "x402-payment" => Ok(Self::X402Payment),
                "PREPARED_TRANSACTION_KIND_SET_RENT_CLAIMER" | "set-rent-claimer" => {
                    Ok(Self::SetRentClaimer)
                }
                _ => Err(ApiConversionError::Invalid(
                    "unsupported prepared transaction kind",
                )),
            },
            _ => Err(ApiConversionError::Invalid(
                "unsupported prepared transaction kind",
            )),
        }
    }
}
