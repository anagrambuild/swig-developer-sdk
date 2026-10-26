use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

// Stored policy actions use the template's internally tagged JSON format.
#[serde_as]
#[derive(Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum PolicyPermission {
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

#[cfg(test)]
#[path = "../../../tests/api/permission.rs"]
mod tests;
