mod approval;
mod authority;
mod encoding;
mod error;
mod network;
mod participant;
pub(crate) mod participants;
pub(crate) mod paymaster;
pub(crate) mod permission;
pub(crate) mod policy;
pub(crate) mod ramp;
pub(crate) mod transaction;
pub(crate) mod wallet;
pub(crate) mod x402;

pub(crate) use approval::ApprovalWire;
pub(crate) use authority::{validate_authority_shape, AuthorityWire};
pub(crate) use encoding::PreparedTransactionKind;
pub(crate) use encoding::{
    decode_hex32, decode_transaction, encode_transaction, OptionalPreparedKind,
    SignatureSchemeWire, TransactionEncodingWire,
};
pub(crate) use error::WireError;
pub(crate) use network::NetworkWire;
#[cfg(test)]
pub(crate) use participant::ParticipantApprovalRequestWire;
pub(crate) use participant::ParticipantSetApprovalPlanWire;

#[cfg(test)]
#[path = "../../../tests/wire/mod.rs"]
pub(crate) mod tests;
