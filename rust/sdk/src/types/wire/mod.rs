mod authority;
mod encoding;
mod error;
mod participant;
pub(crate) mod permission;
pub(crate) mod policy;
pub(crate) mod transaction;
pub(crate) mod wallet;

pub(crate) use authority::{validate_authority_shape, AuthorityWire};
pub(crate) use encoding::{decode_hex32, SignatureSchemeWire};
pub(crate) use error::WireError;
#[cfg(test)]
pub(crate) use participant::{ParticipantApprovalRequestWire, ParticipantSetApprovalPlanWire};

#[cfg(test)]
#[path = "../../../tests/wire/mod.rs"]
mod tests;
