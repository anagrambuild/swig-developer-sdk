//! Swig Developer SDK.

mod config;
mod http;
mod types;
mod wallet;

pub use config::{ClientConfig, ClientError, HttpConfig, RetryOptions};
pub use types::{Network, WalletAuthority, WalletPermission};
pub use wallet::{
    GetPolicyError, Policy, PolicyAuthority, ProgramScopeNumericType, ProgramScopeType,
    SwigRoleAuthority, SwigRolePermission, WalletClient,
};

#[cfg(test)]
#[path = "../tests/transport/mod.rs"]
mod transport_tests;

#[cfg(test)]
#[path = "../tests/transport/support.rs"]
mod test_support;

#[cfg(test)]
#[path = "../tests/hosted/support.rs"]
mod hosted_support;

#[cfg(test)]
#[path = "../tests/hosted/policy.rs"]
mod policy_tests;
