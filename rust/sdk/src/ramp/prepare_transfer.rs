use crate::{
    api::{
        self,
        error::ApiConversionError,
        types::{
            authority::WalletAuthority as ApiWalletAuthority,
            ramp_prepare_transfer::{PrepareResponse, PrepareTransferRequest},
        },
    },
    http::HttpError,
    Network, PreparedTransaction, RampClient, RampDeposit, RampTransfer, RampTransferState,
    WalletAuthority,
};
use solana_pubkey::Pubkey;
use solana_transaction::versioned::TransactionVersion;
use thiserror::Error;
#[derive(Clone)]
pub struct PrepareRampTransferArgs {
    pub order_id: String,
    pub requester_authority: WalletAuthority,
    pub fee_payer: Pubkey,
}

#[derive(Clone)]
pub struct PreparedRampTransfer {
    pub transfer: RampTransfer,
    pub prepared_transaction: PreparedTransaction,
    pub deposit: RampDeposit,
    pub requester_authority: WalletAuthority,
}

#[derive(Debug, Error)]
pub enum PrepareRampTransferError {
    #[error("the current order or transfer state does not permit this operation")]
    PreconditionFailed,
    #[error("invalid Ramp transfer input: {0}")]
    InvalidInput(&'static str),

    #[error("Ramp transfers require an Ed25519 or secp256r1 requester")]
    UnsupportedRequester,

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

    #[error("invalid prepared Ramp transfer: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl RampClient {
    pub async fn prepare_transfer(
        &self,
        args: PrepareRampTransferArgs,
    ) -> Result<PreparedRampTransfer, PrepareRampTransferError> {
        if args.order_id.trim().is_empty() {
            return Err(PrepareRampTransferError::InvalidInput(
                "order ID is required",
            ));
        }
        if !matches!(
            &args.requester_authority,
            WalletAuthority::Ed25519 { .. } | WalletAuthority::Secp256r1 { .. }
        ) {
            return Err(PrepareRampTransferError::UnsupportedRequester);
        }
        let requester_authority = ApiWalletAuthority::try_from(args.requester_authority.clone())
            .map_err(|_| PrepareRampTransferError::InvalidInput("invalid requester authority"))?;
        let request = PrepareTransferRequest {
            order_id: args.order_id,
            requester_authority,
            fee_payer: args.fee_payer,
        };
        let response = api::prepare_ramp_transfer::prepare_ramp_transfer(&self.http, request)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareRampTransferError::RequestFailed
                }
                HttpError::Timeout => PrepareRampTransferError::RequestTimedOut,
                HttpError::InvalidResponse(message) => {
                    PrepareRampTransferError::InvalidResponse(message)
                }
                HttpError::ResponseTooLarge => {
                    PrepareRampTransferError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PrepareRampTransferError::AuthenticationFailed,
                HttpError::PermissionDenied => PrepareRampTransferError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareRampTransferError::InvalidInput("service rejected the request")
                }
                HttpError::FailedPrecondition => PrepareRampTransferError::PreconditionFailed,
                HttpError::Unavailable => PrepareRampTransferError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    PrepareRampTransferError::UnexpectedServiceFailure
                }
            })?;
        normalize_prepare_transfer_response(response, args.requester_authority, args.fee_payer)
    }
}
fn normalize_prepare_transfer_response(
    response: PrepareResponse,
    requester_authority: WalletAuthority,
    fee_payer: Pubkey,
) -> Result<PreparedRampTransfer, PrepareRampTransferError> {
    let prepared = response.prepared_transfer;
    let transfer = RampTransfer::try_from(prepared.transfer)
        .map_err(|_| PrepareRampTransferError::InvalidResponse("malformed Ramp preparation"))?;
    if transfer.state != RampTransferState::Prepared || prepared.deposit.amount.base_units == 0 {
        return Err(PrepareRampTransferError::InvalidResponse(
            "invalid prepared transfer or deposit",
        ));
    }
    let prepared_transaction = PreparedTransaction::try_from(prepared.prepared_transaction)
        .map_err(|ApiConversionError::Invalid(reason)| {
            PrepareRampTransferError::InvalidResponse(reason)
        })?;
    validate_prepared(&prepared_transaction, &requester_authority, fee_payer)
        .map_err(PrepareRampTransferError::InvalidResponse)?;
    Ok(PreparedRampTransfer {
        transfer,
        prepared_transaction,
        deposit: prepared.deposit.into(),
        requester_authority,
    })
}
fn validate_prepared(
    prepared: &PreparedTransaction,
    authority: &WalletAuthority,
    fee_payer: Pubkey,
) -> Result<(), &'static str> {
    if prepared.network != Some(Network::Mainnet)
        || prepared.transaction.version() != TransactionVersion::LEGACY
        || prepared.participant_set_approval_plan.is_some()
    {
        return Err("Ramp requires a mainnet legacy transaction without participant approvals");
    }
    if prepared.transaction.message.static_account_keys().first() != Some(&fee_payer) {
        return Err("Ramp fee payer mismatch");
    }
    if prepared
        .recent_blockhash
        .is_some_and(|blockhash| &blockhash != prepared.transaction.message.recent_blockhash())
    {
        return Err("Ramp recent blockhash mismatch");
    }
    match authority {
        WalletAuthority::Ed25519 { public_key } if prepared.signature_requests.is_empty() => {
            let required = usize::from(
                prepared
                    .transaction
                    .message
                    .header()
                    .num_required_signatures,
            );
            if !prepared
                .transaction
                .message
                .static_account_keys()
                .iter()
                .take(required)
                .any(|key| key == public_key)
            {
                return Err("Ed25519 requester is not a required transaction signer");
            }
            Ok(())
        }
        WalletAuthority::Secp256r1 { .. } if prepared.signature_requests.len() == 1 => {
            if prepared
                .signature_requests
                .first()
                .is_some_and(|request| &request.authority == authority)
            {
                Ok(())
            } else {
                Err("Ramp requester mismatch")
            }
        }
        _ => Err("Ramp signature requests do not match requester"),
    }
}
