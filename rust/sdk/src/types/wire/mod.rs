mod authority;
mod error;
pub(crate) mod permission;
pub(crate) mod policy;
pub(crate) mod wallet;

pub(crate) use authority::{validate_authority_shape, AuthorityWire};
pub(crate) use error::WireError;

#[cfg(test)]
#[path = "../../../tests/wire/mod.rs"]
mod tests;
