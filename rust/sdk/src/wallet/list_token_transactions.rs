use serde_json::Value;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::{wallet::assets::TransactionListWire, WireError},
    Network, WalletClient,
};

#[derive(Clone)]
pub struct ListSwigTokenTransactionsArgs {
    pub swig_config_address: Pubkey,
    pub limit: Option<u32>,
}

#[derive(Clone)]
pub enum TransactionDirection {
    Inflow,
    Outflow,
}

#[derive(Clone)]
pub enum AssetTransfer {
    Sol {
        lamports: u64,
        ui_amount: f64,
        usd_price: f64,
        usd_value: f64,
    },
    Token {
        token_account_address: Pubkey,
        mint_address: Pubkey,
        token_program: Pubkey,
        symbol: String,
        name: String,
        amount_raw: u64,
        decimals: u32,
        ui_amount: f64,
        usd_price: f64,
        usd_value: f64,
    },
}

#[derive(Clone)]
pub struct WalletTransaction {
    pub transaction_signature: Signature,
    pub slot: u64,
    pub block_time: Option<String>,
    pub owner_address: Pubkey,
    pub wallet_address: Pubkey,
    pub is_subaccount: bool,
    pub direction: TransactionDirection,
    pub transfer: AssetTransfer,
}

#[derive(Clone)]
pub struct WalletTransactionList {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub transactions: Vec<WalletTransaction>,
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
        let network = match self.network {
            Network::Devnet => "NETWORK_DEVNET",
            Network::Mainnet => "NETWORK_MAINNET",
        };
        let mut path =
            format!("/wallet/swig/{swig_config_address}/token-transactions?network={network}");
        if let Some(limit) = args.limit {
            path.push_str(&format!("&limit={limit}"));
        }
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => {
                ListTokenTransactionsError::RequestFailed
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
        let history = normalize_list_token_transactions_response(&bytes)?;
        if history.swig_config_address != swig_config_address {
            return Err(ListTokenTransactionsError::InvalidResponse(
                "wallet identity mismatch",
            ));
        }
        Ok(history)
    }
}

fn normalize_list_token_transactions_response(
    bytes: &[u8],
) -> Result<WalletTransactionList, ListTokenTransactionsError> {
    let mut payload: Value = serde_json::from_slice(bytes).map_err(|_| {
        ListTokenTransactionsError::InvalidResponse("malformed list_token_transactions response")
    })?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(ListTokenTransactionsError::InvalidResponse(
            "malformed list_token_transactions response",
        ));
    }
    let wire: TransactionListWire = serde_json::from_value(payload).map_err(|_| {
        ListTokenTransactionsError::InvalidResponse("malformed list_token_transactions response")
    })?;
    WalletTransactionList::try_from(wire)
        .map_err(|WireError::Invalid(message)| ListTokenTransactionsError::InvalidResponse(message))
}
