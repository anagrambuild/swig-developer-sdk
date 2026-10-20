use solana_pubkey::Pubkey;

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
