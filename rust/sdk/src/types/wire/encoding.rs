use base64::{engine::general_purpose::STANDARD, Engine};
use bincode::Options;
use serde::{Deserialize, Serialize};
use solana_transaction::versioned::VersionedTransaction;

use super::WireError;

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum EnumValue {
    Code(u32),
    Name(String),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "EnumValue")]
pub(crate) enum TransactionEncodingWire {
    #[serde(rename = "TRANSACTION_ENCODING_BASE64")]
    Base64,
}

impl TryFrom<EnumValue> for TransactionEncodingWire {
    type Error = WireError;

    fn try_from(value: EnumValue) -> Result<Self, Self::Error> {
        match value {
            EnumValue::Code(1) => Ok(Self::Base64),
            EnumValue::Name(name)
                if matches!(name.as_str(), "TRANSACTION_ENCODING_BASE64" | "base64") =>
            {
                Ok(Self::Base64)
            }
            _ => Err(WireError::Invalid("unsupported transaction encoding")),
        }
    }
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

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum PreparedTransactionKind {
    CreateSwigWallet,
    AddAuthority,
    ConfigureRecovery,
    CreateParticipantSet,
    X402Payment,
    SetRentClaimer,
}

#[derive(Clone, Default, Deserialize)]
#[serde(try_from = "Option<EnumValue>")]
pub(crate) struct OptionalPreparedKind(pub Option<PreparedTransactionKind>);

impl TryFrom<Option<EnumValue>> for OptionalPreparedKind {
    type Error = WireError;

    fn try_from(value: Option<EnumValue>) -> Result<Self, Self::Error> {
        let code = match value {
            None => 0,
            Some(EnumValue::Code(code)) => code,
            Some(EnumValue::Name(name)) => match name.as_str() {
                "PREPARED_TRANSACTION_KIND_UNSPECIFIED" => 0,
                "PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET" | "create-swig-wallet" => 1,
                "PREPARED_TRANSACTION_KIND_ADD_AUTHORITY" | "add-authority" => 2,
                "PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY" | "configure-recovery" => 3,
                "PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET" | "create-participant-set" => 4,
                "PREPARED_TRANSACTION_KIND_X402_PAYMENT" | "x402-payment" => 5,
                "PREPARED_TRANSACTION_KIND_SET_RENT_CLAIMER" | "set-rent-claimer" => 6,
                _ => return Err(WireError::Invalid("unsupported prepared transaction kind")),
            },
        };
        let kind = match code {
            0 => None,
            1 => Some(PreparedTransactionKind::CreateSwigWallet),
            2 => Some(PreparedTransactionKind::AddAuthority),
            3 => Some(PreparedTransactionKind::ConfigureRecovery),
            4 => Some(PreparedTransactionKind::CreateParticipantSet),
            5 => Some(PreparedTransactionKind::X402Payment),
            6 => Some(PreparedTransactionKind::SetRentClaimer),
            _ => return Err(WireError::Invalid("unsupported prepared transaction kind")),
        };
        Ok(Self(kind))
    }
}

pub(crate) fn decode_hex32(value: &str) -> Result<[u8; 32], WireError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    let mut bytes = [0; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|_| WireError::Invalid("invalid 32-byte hex value"))?;
    Ok(bytes)
}

pub(crate) fn decode_transaction(value: &str) -> Result<VersionedTransaction, WireError> {
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| WireError::Invalid("invalid Base64 transaction"))?;
    if STANDARD.encode(&bytes) != value {
        return Err(WireError::Invalid("noncanonical Base64 transaction"));
    }
    let byte_limit = u64::try_from(bytes.len())
        .map_err(|_| WireError::Invalid("transaction exceeds decoder limit"))?;
    let transaction: VersionedTransaction = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(byte_limit)
        .reject_trailing_bytes()
        .deserialize(&bytes)
        .map_err(|_| WireError::Invalid("invalid Solana transaction"))?;
    transaction
        .sanitize()
        .map_err(|_| WireError::Invalid("invalid Solana transaction structure"))?;
    Ok(transaction)
}

pub(crate) fn encode_transaction(transaction: &VersionedTransaction) -> Result<String, WireError> {
    transaction
        .sanitize()
        .map_err(|_| WireError::Invalid("invalid Solana transaction structure"))?;
    let bytes = bincode::serialize(transaction)
        .map_err(|_| WireError::Invalid("transaction serialization failed"))?;
    Ok(STANDARD.encode(bytes))
}
