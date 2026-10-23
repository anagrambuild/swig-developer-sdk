use crate::WalletAuthority;
use crate::{
    http::HttpError,
    types::wire::{
        transaction::{validate_prepared_transaction_shape, PreparedTransactionWire},
        wallet::{prepared::validate_prepared_context, swap::SwapRequest},
        AuthorityWire, WireError,
    },
    PreparedTransaction, WalletClient,
};
use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Clone)]
pub enum SwapQuoteMode {
    Fast,
}

#[derive(Clone)]
pub enum ComputeUnitPricePercentile {
    Medium,
    High,
    VeryHigh,
    BasisPoints(u16),
}

#[derive(Clone)]
pub struct PrepareSwapArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount: u64,
    pub options: Option<SwapOptions>,
}

/// Optional swap settings. All fields default to `None`.
#[derive(Clone, Default)]
pub struct SwapOptions {
    pub slippage_bps: Option<u32>,
    /// Recipient owner; the backend derives the token account when needed.
    pub destination_owner: Option<Pubkey>,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub tip_amount_lamports: Option<u64>,
    pub compute_unit_price_percentile: Option<ComputeUnitPricePercentile>,
    pub max_accounts: Option<u32>,
    pub mode: Option<SwapQuoteMode>,
    pub blockhash_slots_to_expiry: Option<u32>,
}

#[derive(Debug, Error)]
pub enum PrepareSwapError {
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
    /// Prepares a Jupiter swap.
    pub async fn prepare_swap(
        &self,
        args: PrepareSwapArgs,
    ) -> Result<PreparedTransaction, PrepareSwapError> {
        if args.amount == 0 {
            return Err(PrepareSwapError::InvalidInput(
                "amount must be greater than zero",
            ));
        }
        if !matches!(
            args.requester_authority,
            WalletAuthority::Ed25519 { .. } | WalletAuthority::Secp256r1 { .. }
        ) {
            return Err(PrepareSwapError::InvalidInput(
                "unsupported requester authority",
            ));
        }
        let options = args.options.unwrap_or_default();
        if matches!(
            options.compute_unit_price_percentile,
            Some(ComputeUnitPricePercentile::BasisPoints(value)) if value > 10_000
        ) {
            return Err(PrepareSwapError::InvalidInput(
                "fee percentile exceeds 10000 basis points",
            ));
        }

        let swig_config_address = args.swig_config_address;
        let body = SwapRequest {
            network: self.network.into(),
            swig_address: swig_config_address.to_string(),
            fee_payer: args.fee_payer.to_string(),
            requester_authority: AuthorityWire::try_from(args.requester_authority)
                .map_err(|WireError::Invalid(message)| PrepareSwapError::InvalidInput(message))?,
            input_mint: args.input_mint.to_string(),
            output_mint: args.output_mint.to_string(),
            amount: args.amount,
            slippage_bps: options.slippage_bps,
            destination_account: options.destination_owner.map(|value| value.to_string()),
            wrap_and_unwrap_sol: options.wrap_and_unwrap_sol,
            tip_amount_lamports: options.tip_amount_lamports,
            compute_unit_price_percentile: options.compute_unit_price_percentile.map(|value| {
                match value {
                    ComputeUnitPricePercentile::Medium => "medium".to_owned(),
                    ComputeUnitPricePercentile::High => "high".to_owned(),
                    ComputeUnitPricePercentile::VeryHigh => "veryHigh".to_owned(),
                    ComputeUnitPricePercentile::BasisPoints(value) => value.to_string(),
                }
            }),
            max_accounts: options.max_accounts,
            mode: options.mode.map(|value| match value {
                SwapQuoteMode::Fast => "fast".to_owned(),
            }),
            blockhash_slots_to_expiry: options.blockhash_slots_to_expiry,
        };
        let body = serde_json::to_vec(&body).map_err(|_| PrepareSwapError::RequestFailed)?;
        let bytes = self
            .http
            .post("/transaction/swap/jupiter", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => PrepareSwapError::RequestFailed,
                HttpError::Timeout => PrepareSwapError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareSwapError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PrepareSwapError::AuthenticationFailed,
                HttpError::PermissionDenied => PrepareSwapError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareSwapError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => PrepareSwapError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => PrepareSwapError::UnexpectedServiceFailure,
            })?;
        let mut payload: Value = serde_json::from_slice(&bytes)
            .map_err(|_| PrepareSwapError::InvalidResponse("invalid JSON response"))?;
        if let Value::Object(fields) = &mut payload {
            if let Some(data) = fields.remove("data") {
                payload = data;
            }
        }
        validate_prepared_transaction_shape(&payload)
            .map_err(|_| PrepareSwapError::InvalidResponse("malformed prepared transaction"))?;
        let wire: PreparedTransactionWire = serde_json::from_value(payload)
            .map_err(|_| PrepareSwapError::InvalidResponse("malformed prepared transaction"))?;
        let prepared = PreparedTransaction::try_from(wire)
            .map_err(|WireError::Invalid(message)| PrepareSwapError::InvalidResponse(message))?;
        validate_prepared_context(&prepared, Some(swig_config_address), self.network)
            .map_err(|WireError::Invalid(message)| PrepareSwapError::InvalidResponse(message))?;
        Ok(prepared)
    }
}
