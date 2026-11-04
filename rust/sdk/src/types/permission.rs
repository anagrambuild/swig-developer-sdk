use solana_pubkey::Pubkey;

use crate::api::types::permission::{AddRoleAction, PolicyPermission};

/// A permission in a wallet policy or an add-role request.
#[derive(Clone, Eq, PartialEq)]
pub enum WalletPermission {
    All,
    AllButManageAuthority,
    ManageAuthority,
    SolLimit {
        amount: u64,
    },
    SolRecurringLimit {
        recurring_amount: u64,
        window: u64,
    },
    SolDestinationLimit {
        amount: u64,
        destination: Pubkey,
    },
    SolRecurringDestinationLimit {
        recurring_amount: u64,
        window: u64,
        destination: Pubkey,
    },
    TokenLimit {
        mint: Pubkey,
        amount: u64,
    },
    TokenRecurringLimit {
        mint: Pubkey,
        recurring_amount: u64,
        window: u64,
    },
    TokenDestinationLimit {
        mint: Pubkey,
        amount: u64,
        destination: Pubkey,
    },
    TokenRecurringDestinationLimit {
        mint: Pubkey,
        recurring_amount: u64,
        window: u64,
        destination: Pubkey,
    },
    Program {
        program_id: Pubkey,
    },
    ProgramAll,
    ProgramCurated,
    StakeLimit {
        amount: u64,
    },
    StakeRecurringLimit {
        recurring_amount: u64,
        window: u64,
    },
    StakeAll,
    SubAccount,
}

impl From<PolicyPermission> for WalletPermission {
    fn from(value: PolicyPermission) -> Self {
        match value {
            PolicyPermission::All {} => Self::All,
            PolicyPermission::AllButManageAuthority {} => Self::AllButManageAuthority,
            PolicyPermission::ManageAuthority {} => Self::ManageAuthority,
            PolicyPermission::SolLimit { amount } => Self::SolLimit { amount },
            PolicyPermission::SolRecurringLimit {
                recurring_amount,
                window,
            } => Self::SolRecurringLimit {
                recurring_amount,
                window,
            },
            PolicyPermission::SolDestinationLimit {
                amount,
                destination,
            } => Self::SolDestinationLimit {
                amount,
                destination,
            },
            PolicyPermission::SolRecurringDestinationLimit {
                recurring_amount,
                window,
                destination,
            } => Self::SolRecurringDestinationLimit {
                recurring_amount,
                window,
                destination,
            },
            PolicyPermission::TokenLimit { mint, amount } => Self::TokenLimit { mint, amount },
            PolicyPermission::TokenRecurringLimit {
                mint,
                recurring_amount,
                window,
            } => Self::TokenRecurringLimit {
                mint,
                recurring_amount,
                window,
            },
            PolicyPermission::TokenDestinationLimit {
                mint,
                amount,
                destination,
            } => Self::TokenDestinationLimit {
                mint,
                amount,
                destination,
            },
            PolicyPermission::TokenRecurringDestinationLimit {
                mint,
                recurring_amount,
                window,
                destination,
            } => Self::TokenRecurringDestinationLimit {
                mint,
                recurring_amount,
                window,
                destination,
            },
            PolicyPermission::Program { program_id } => Self::Program { program_id },
            PolicyPermission::ProgramAll {} => Self::ProgramAll,
            PolicyPermission::ProgramCurated {} => Self::ProgramCurated,
            PolicyPermission::StakeLimit { amount } => Self::StakeLimit { amount },
            PolicyPermission::StakeRecurringLimit {
                recurring_amount,
                window,
            } => Self::StakeRecurringLimit {
                recurring_amount,
                window,
            },
            PolicyPermission::StakeAll {} => Self::StakeAll,
            PolicyPermission::SubAccount {} => Self::SubAccount,
        }
    }
}

impl From<WalletPermission> for AddRoleAction {
    fn from(value: WalletPermission) -> Self {
        match value {
            WalletPermission::All => Self::All {},
            WalletPermission::AllButManageAuthority => Self::AllButManageAuthority {},
            WalletPermission::ManageAuthority => Self::ManageAuthority {},
            WalletPermission::SolLimit { amount } => Self::SolLimit { amount },
            WalletPermission::SolRecurringLimit {
                recurring_amount,
                window,
            } => Self::SolRecurringLimit {
                recurring_amount,
                window,
            },
            WalletPermission::SolDestinationLimit {
                amount,
                destination,
            } => Self::SolDestinationLimit {
                amount,
                destination,
            },
            WalletPermission::SolRecurringDestinationLimit {
                recurring_amount,
                window,
                destination,
            } => Self::SolRecurringDestinationLimit {
                recurring_amount,
                window,
                destination,
            },
            WalletPermission::TokenLimit { mint, amount } => Self::TokenLimit { mint, amount },
            WalletPermission::TokenRecurringLimit {
                mint,
                recurring_amount,
                window,
            } => Self::TokenRecurringLimit {
                mint,
                recurring_amount,
                window,
            },
            WalletPermission::TokenDestinationLimit {
                mint,
                amount,
                destination,
            } => Self::TokenDestinationLimit {
                mint,
                amount,
                destination,
            },
            WalletPermission::TokenRecurringDestinationLimit {
                mint,
                recurring_amount,
                window,
                destination,
            } => Self::TokenRecurringDestinationLimit {
                mint,
                recurring_amount,
                window,
                destination,
            },
            WalletPermission::Program { program_id } => Self::Program { program_id },
            WalletPermission::ProgramAll => Self::ProgramAll {},
            WalletPermission::ProgramCurated => Self::ProgramCurated {},
            WalletPermission::StakeLimit { amount } => Self::StakeLimit { amount },
            WalletPermission::StakeRecurringLimit {
                recurring_amount,
                window,
            } => Self::StakeRecurringLimit {
                recurring_amount,
                window,
            },
            WalletPermission::StakeAll => Self::StakeAll {},
            WalletPermission::SubAccount => Self::SubAccount {},
        }
    }
}
