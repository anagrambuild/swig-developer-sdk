use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use crate::WalletPermission;

// Stored policy actions use the template's internally tagged JSON format.
#[serde_as]
#[derive(Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum PolicyPermissionWire {
    All {},
    AllButManageAuthority {},
    ManageAuthority {},
    SolLimit {
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
    SolRecurringLimit {
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
    },
    SolDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    SolRecurringDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    TokenLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
    TokenRecurringLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
    },
    TokenDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    TokenRecurringDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    Program {
        #[serde_as(as = "DisplayFromStr")]
        program_id: Pubkey,
    },
    ProgramAll {},
    ProgramCurated {},
    StakeLimit {
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
    StakeRecurringLimit {
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
    },
    StakeAll {},
    SubAccount {},
}

impl From<PolicyPermissionWire> for WalletPermission {
    fn from(value: PolicyPermissionWire) -> Self {
        match value {
            PolicyPermissionWire::All {} => Self::All,
            PolicyPermissionWire::AllButManageAuthority {} => Self::AllButManageAuthority,
            PolicyPermissionWire::ManageAuthority {} => Self::ManageAuthority,
            PolicyPermissionWire::SolLimit { amount } => Self::SolLimit { amount },
            PolicyPermissionWire::SolRecurringLimit {
                recurring_amount,
                window,
            } => Self::SolRecurringLimit {
                recurring_amount,
                window,
            },
            PolicyPermissionWire::SolDestinationLimit {
                amount,
                destination,
            } => Self::SolDestinationLimit {
                amount,
                destination,
            },
            PolicyPermissionWire::SolRecurringDestinationLimit {
                recurring_amount,
                window,
                destination,
            } => Self::SolRecurringDestinationLimit {
                recurring_amount,
                window,
                destination,
            },
            PolicyPermissionWire::TokenLimit { mint, amount } => Self::TokenLimit { mint, amount },
            PolicyPermissionWire::TokenRecurringLimit {
                mint,
                recurring_amount,
                window,
            } => Self::TokenRecurringLimit {
                mint,
                recurring_amount,
                window,
            },
            PolicyPermissionWire::TokenDestinationLimit {
                mint,
                amount,
                destination,
            } => Self::TokenDestinationLimit {
                mint,
                amount,
                destination,
            },
            PolicyPermissionWire::TokenRecurringDestinationLimit {
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
            PolicyPermissionWire::Program { program_id } => Self::Program { program_id },
            PolicyPermissionWire::ProgramAll {} => Self::ProgramAll,
            PolicyPermissionWire::ProgramCurated {} => Self::ProgramCurated,
            PolicyPermissionWire::StakeLimit { amount } => Self::StakeLimit { amount },
            PolicyPermissionWire::StakeRecurringLimit {
                recurring_amount,
                window,
            } => Self::StakeRecurringLimit {
                recurring_amount,
                window,
            },
            PolicyPermissionWire::StakeAll {} => Self::StakeAll,
            PolicyPermissionWire::SubAccount {} => Self::SubAccount,
        }
    }
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum AddRoleActionWire {
    All {},
    AllButManageAuthority {},
    ManageAuthority {},
    SolLimit {
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
    SolRecurringLimit {
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
    },
    SolDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    SolRecurringDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    TokenLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
    TokenRecurringLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
    },
    TokenDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    TokenRecurringDestinationLimit {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
    },
    Program {
        #[serde_as(as = "DisplayFromStr")]
        program_id: Pubkey,
    },
    ProgramAll {},
    ProgramCurated {},
    StakeLimit {
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
    StakeRecurringLimit {
        #[serde_as(as = "DisplayFromStr")]
        recurring_amount: u64,
        #[serde_as(as = "DisplayFromStr")]
        window: u64,
    },
    StakeAll {},
    SubAccount {},
}

impl From<WalletPermission> for AddRoleActionWire {
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
