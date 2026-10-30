use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    api::{
        error::ApiConversionError,
        types::assets::{
            TransactionDirection as ApiTransactionDirection, TransactionList,
            WalletTransaction as ApiWalletTransaction,
        },
    },
    http::HttpError,
    types::assets::NormalizedAsset,
    AssetTransfer, TransactionDirection, WalletClient, WalletTransaction, WalletTransactionList,
};

#[derive(Clone)]
pub struct ListSwigTokenTransactionsArgs {
    pub swig_config_address: Pubkey,
    pub limit: Option<u32>,
}

/// Failures returned by `list_token_transactions`.
#[derive(Debug, Error)]
pub enum ListTokenTransactionsError {
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),

    #[error("authentication failed")]
    AuthenticationFailed,

    #[error("permission denied")]
    PermissionDenied,

    #[error("service unavailable")]
    ServiceUnavailable,

    #[error("request timed out")]
    RequestTimedOut,

    #[error("request could not be completed")]
    RequestFailed,

    #[error("invalid response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl WalletClient {
    /// Lists indexed token and SOL transfers in the order returned by the service.
    pub async fn list_token_transactions(
        &self,
        args: ListSwigTokenTransactionsArgs,
    ) -> Result<WalletTransactionList, ListTokenTransactionsError> {
        let swig_config_address = args.swig_config_address;
        let response = crate::api::list_token_transactions::list_token_transactions(
            &self.http,
            &swig_config_address,
            self.network,
            args.limit,
        )
        .await
        .map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => {
                ListTokenTransactionsError::RequestFailed
            }
            HttpError::InvalidResponse(message) => {
                ListTokenTransactionsError::InvalidResponse(message)
            }
            HttpError::Timeout => ListTokenTransactionsError::RequestTimedOut,
            HttpError::ResponseTooLarge => {
                ListTokenTransactionsError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => ListTokenTransactionsError::AuthenticationFailed,
            HttpError::PermissionDenied => ListTokenTransactionsError::PermissionDenied,
            HttpError::InvalidArgument => {
                ListTokenTransactionsError::InvalidInput("service rejected the request")
            }
            HttpError::Unavailable => ListTokenTransactionsError::ServiceUnavailable,
            HttpError::NotFound
            | HttpError::AlreadyExists
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => {
                ListTokenTransactionsError::UnexpectedServiceFailure
            }
        })?;
        let history = WalletTransactionList::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| {
                ListTokenTransactionsError::InvalidResponse(message)
            },
        )?;
        if history.swig_config_address != swig_config_address {
            return Err(ListTokenTransactionsError::InvalidResponse(
                "wallet identity mismatch",
            ));
        }
        Ok(history)
    }
}

impl TryFrom<ApiWalletTransaction> for WalletTransaction {
    type Error = ApiConversionError;
    fn try_from(value: ApiWalletTransaction) -> Result<Self, Self::Error> {
        let transfer = match NormalizedAsset::try_from(value.asset)? {
            NormalizedAsset::Sol(balance) => AssetTransfer::Sol {
                lamports: balance.lamports,
                ui_amount: balance.ui_amount,
                usd_price: balance.usd_price,
                usd_value: balance.usd_value,
            },
            NormalizedAsset::Token(balance) => AssetTransfer::Token {
                token_account_address: value
                    .token_account_address
                    .parse()
                    .map_err(|_| ApiConversionError::Invalid("invalid token account address"))?,
                mint_address: balance.mint_address,
                token_program: balance.token_program,
                symbol: balance.symbol,
                name: balance.name,
                amount_raw: balance.amount_raw,
                decimals: balance.decimals,
                ui_amount: balance.ui_amount,
                usd_price: balance.usd_price,
                usd_value: balance.usd_value,
            },
        };
        Ok(Self {
            transaction_signature: value.transaction_signature,
            slot: value.slot,
            block_time: value.block_time,
            owner_address: value.owner_address,
            wallet_address: value.wallet_address,
            is_subaccount: value.is_subaccount,
            direction: match value.direction {
                ApiTransactionDirection::Inflow => TransactionDirection::Inflow,
                ApiTransactionDirection::Outflow => TransactionDirection::Outflow,
            },
            transfer,
        })
    }
}
impl TryFrom<TransactionList> for WalletTransactionList {
    type Error = ApiConversionError;
    fn try_from(value: TransactionList) -> Result<Self, Self::Error> {
        if value
            .transactions
            .iter()
            .any(|transaction| transaction.wallet_address != value.wallet_address)
        {
            return Err(ApiConversionError::Invalid(
                "history row wallet identity mismatch",
            ));
        }
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
            transactions: value
                .transactions
                .into_iter()
                .map(WalletTransaction::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}
