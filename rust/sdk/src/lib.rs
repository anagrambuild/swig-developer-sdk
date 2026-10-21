//! Swig Developer SDK.

mod api;
mod config;
mod http;
mod types;
mod wallet;

pub use config::{ClientConfig, ClientError, HttpConfig, RetryOptions};
pub use types::{
    Network, ProgramScopeNumericType, ProgramScopeType, SwigRole, SwigRoleAction,
    SwigRoleAuthority, SwigRoleList, SwigRolePermission, WalletAuthority, WalletPermission,
};
pub use types::{SolBalance, SwigUsdBalance, TokenBalance, WalletBalanceSummary};
pub use wallet::{
    GetPolicyError, GetUsdBalanceError, ListRolesError, ListTokenBalancesError, Policy,
    WalletClient,
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

#[cfg(test)]
#[path = "../tests/hosted/roles.rs"]
mod role_tests;

#[cfg(test)]
#[path = "../tests/hosted/balances.rs"]
mod balance_tests;

#[cfg(test)]
#[path = "../tests/hosted/history.rs"]
mod history_tests;

pub use types::{AssetTransfer, TransactionDirection, WalletTransaction, WalletTransactionList};
pub use wallet::{ListSwigTokenTransactionsArgs, ListTokenTransactionsError};

pub use types::{ClientSignatureRequest, ParticipantApprovalRequest, ParticipantSetApprovalPlan};

pub use types::{PreparedTransaction, RecentBlockhash, WalletAddressInfo};
