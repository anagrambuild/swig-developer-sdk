//! Swig Developer SDK.

#[allow(
    dead_code,
    reason = "Consumed by typed endpoints in the next stack slice"
)]
mod api;
mod config;
mod http;
mod types;
mod wallet;

pub use config::{ClientConfig, ClientError, HttpConfig, RetryOptions};
pub use types::{Network, WalletAuthority, WalletPermission};
pub use wallet::WalletClient;

#[cfg(test)]
#[path = "../tests/transport/mod.rs"]
mod transport_tests;

#[cfg(test)]
#[path = "../tests/transport/support.rs"]
mod test_support;
