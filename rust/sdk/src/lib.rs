//! Swig Developer SDK.

mod api;
mod config;
mod http;
mod participants;
mod paymaster;
mod ramp;
mod signing;
mod types;
mod wallet;

pub use config::{
    ClientConfig, ClientError, HttpConfig, RampConfig, RampEnvironment, RetryOptions,
};
pub use participants::{
    CompileParticipantSetApprovalsArgs, CompileParticipantSetApprovalsError,
    ParticipantSetApprovalCompilation, ParticipantSetCreationPlan, PrepareCreateParticipantSetArgs,
    PrepareCreateParticipantSetError,
};
pub use paymaster::{
    GetPaymasterBalanceError, PaymasterBalance, PaymasterClient, PaymasterKind, SponsorBundleError,
    SponsorError, SponsorSignedTransactionArgs, SponsorSignedTransactionBundleArgs,
    SubmittedTransaction, SubmittedTransactionBundle,
};
pub use ramp::*;
pub use signing::{
    Ed25519ParticipantSigner, ParticipantApprovalError, ParticipantSigner,
    Secp256k1ParticipantSigner, Secp256r1ParticipantSigner,
};
pub use signing::{
    Secp256k1TransactionSigner, Secp256r1TransactionSigner, TransactionSignError, TransactionSigner,
};
pub use solana_instruction::{AccountMeta, Instruction};
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

pub use wallet::{PrepareCreateWalletArgs, PrepareCreateWalletError, WalletCreation};

#[cfg(test)]
#[path = "../tests/hosted/wallet_preparation_support.rs"]
mod wallet_preparation_support;

#[cfg(test)]
#[path = "../tests/hosted/wallet_creation.rs"]
mod wallet_creation_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_transfers.rs"]
mod wallet_transfer_tests;

pub use wallet::{
    PrepareTransferSolArgs, PrepareTransferSolError, PrepareTransferTokenArgs,
    PrepareTransferTokenError,
};

pub use wallet::{PrepareOperation, PrepareTransfersArgs, PrepareTransfersError};

#[cfg(test)]
#[path = "../tests/hosted/wallet_batch_preparation.rs"]
mod wallet_batch_preparation_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_swaps.rs"]
mod wallet_swap_tests;

pub use wallet::{
    ComputeUnitPricePercentile, PrepareSwapArgs, PrepareSwapError, SwapOptions, SwapQuoteMode,
};

pub use wallet::{PrepareTransactionArgs, PrepareTransactionError};

#[cfg(test)]
#[path = "../tests/hosted/wallet_custom_transactions.rs"]
mod wallet_custom_transaction_tests;

#[cfg(test)]
#[path = "../tests/hosted/wallet_add_role.rs"]
mod wallet_add_role_tests;

pub use wallet::{PrepareAddRoleArgs, PrepareAddRoleError};
#[cfg(test)]
#[path = "../tests/signing/mod.rs"]
mod signing_tests;

#[cfg(test)]
#[path = "../tests/hosted/participant_creation.rs"]
mod participant_creation_tests;

pub use types::ParticipantApproval;
#[cfg(test)]
#[path = "../tests/hosted/participant_compilation.rs"]
mod participant_compilation_tests;

#[cfg(test)]
#[path = "../tests/hosted/paymaster_balances.rs"]
mod paymaster_balance_tests;

#[cfg(test)]
#[path = "../tests/hosted/paymaster_sponsor.rs"]
mod paymaster_sponsor_tests;

#[cfg(test)]
#[path = "../tests/hosted/paymaster_bundle.rs"]
mod paymaster_bundle_tests;

#[cfg(test)]
#[path = "../tests/hosted/ramp_options.rs"]
mod ramp_options_tests;
