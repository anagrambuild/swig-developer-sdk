//! Swig Developer SDK.

mod config;
mod http;
mod signing;
mod types;
mod wallet;

pub use config::{ClientConfig, ClientError, HttpConfig, RetryOptions};
pub use signing::{
    Secp256k1TransactionSigner, Secp256r1TransactionSigner, TransactionSignError, TransactionSigner,
};
pub use solana_instruction::{AccountMeta, Instruction};
pub use types::{
    ClientSignatureRequest, Network, ParticipantApprovalRequest, ParticipantSetApprovalPlan,
    PreparedTransaction, RecentBlockhash, WalletAddressInfo, WalletAuthority, WalletPermission,
};
pub use wallet::{
    AssetTransfer, ComputeUnitPricePercentile, GetPolicyError, GetUsdBalanceError, ListRolesError,
    ListSwigTokenTransactionsArgs, ListTokenBalancesError, ListTokenTransactionsError, Policy,
    PolicyAuthority, PrepareAddRoleArgs, PrepareAddRoleError, PrepareCreateWalletArgs,
    PrepareCreateWalletError, PrepareOperation, PrepareSwapArgs, PrepareSwapError,
    PrepareTransactionArgs, PrepareTransactionError, PrepareTransferSolArgs,
    PrepareTransferSolError, PrepareTransferTokenArgs, PrepareTransferTokenError,
    PrepareTransfersArgs, PrepareTransfersError, ProgramScopeNumericType, ProgramScopeType,
    SolBalance, SwapOptions, SwapQuoteMode, SwigRole, SwigRoleAction, SwigRoleAuthority,
    SwigRoleList, SwigRolePermission, SwigUsdBalance, TokenBalance, TransactionDirection,
    WalletBalanceSummary, WalletClient, WalletCreation, WalletTransaction, WalletTransactionList,
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

#[cfg(test)]
#[path = "../tests/hosted/wallet_preparation_support.rs"]
mod wallet_preparation_support;

#[cfg(test)]
#[path = "../tests/hosted/wallet_creation.rs"]
mod wallet_creation_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_transfers.rs"]
mod wallet_transfer_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_batch_preparation.rs"]
mod wallet_batch_preparation_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_swaps.rs"]
mod wallet_swap_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_custom_transactions.rs"]
mod wallet_custom_transaction_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_add_role.rs"]
mod wallet_add_role_tests;

#[cfg(test)]
#[path = "../tests/signing/mod.rs"]
mod signing_tests;
