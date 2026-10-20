mod authority;
mod error;
pub(crate) mod permission;

pub(crate) use authority::AuthorityWire;
pub(crate) use error::WireError;

#[cfg(test)]
#[path = "../../../tests/wire/mod.rs"]
mod tests;
