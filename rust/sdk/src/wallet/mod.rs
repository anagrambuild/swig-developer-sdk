mod client;
mod create;
mod get_policy;
mod get_usd_balance;
mod list_roles;
mod list_token_balances;
mod list_token_transactions;
mod prepare;
mod transfer;

pub use client::WalletClient;
pub use create::{PrepareCreateWalletArgs, PrepareCreateWalletError, WalletCreation};
pub use get_policy::{GetPolicyError, Policy};
pub use list_roles::ListRolesError;

pub use get_usd_balance::GetUsdBalanceError;
pub use list_token_balances::ListTokenBalancesError;

pub use list_token_transactions::{ListSwigTokenTransactionsArgs, ListTokenTransactionsError};

pub(crate) mod prepared;

pub use transfer::{
    PrepareTransferSolArgs, PrepareTransferSolError, PrepareTransferTokenArgs,
    PrepareTransferTokenError,
};

pub use prepare::{PrepareOperation, PrepareTransfersArgs, PrepareTransfersError};
