use super::{
    types::{solana_network, validate_required},
    ExactSvmPaymentRequirement, PaymentRequiredV2, PaymentRequirement,
};
use crate::{
    api::{
        error::ApiConversionError,
        prepare_x402_payment::prepare_x402_payment,
        types::{
            authority::WalletAuthority as ApiWalletAuthority,
            encoding::PreparedTransactionKind,
            x402::{self as api, PrepareX402PaymentRequest, PrepareX402PaymentResponse},
        },
    },
    http::HttpError,
    wallet::prepared::validate_prepared_context,
    Network, PreparedTransaction, WalletAuthority, WalletClient,
};
use solana_pubkey::Pubkey;
use thiserror::Error;

pub struct PrepareX402PaymentArgs {
    pub swig_config_address: Pubkey,
    pub requester_authority: WalletAuthority,
    pub payment_required: PaymentRequiredV2,
    pub accepted_index: Option<u32>,
}

#[derive(Clone)]
pub struct X402PaymentPreparation {
    pub prepared_transaction: PreparedTransaction,
    pub payment_required: PaymentRequiredV2,
    pub accepted_index: u32,
}

#[derive(Debug, Error)]
pub enum PrepareX402PaymentError {
    #[error("invalid payment input: {0}")]
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
    #[error("invalid payment response: {0}")]
    InvalidResponse(&'static str),
    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl WalletClient {
    /// Prepares a payment once. The resource service owns submission and settlement.
    pub async fn prepare_x402_payment(
        &self,
        args: PrepareX402PaymentArgs,
    ) -> Result<X402PaymentPreparation, PrepareX402PaymentError> {
        validate_required(&args.payment_required).map_err(PrepareX402PaymentError::InvalidInput)?;
        if !matches!(
            args.requester_authority,
            WalletAuthority::Ed25519 { .. } | WalletAuthority::Secp256r1 { .. }
        ) {
            return Err(PrepareX402PaymentError::InvalidInput(
                "payment requester must use Ed25519 or secp256r1",
            ));
        }
        let authority = ApiWalletAuthority::try_from(args.requester_authority)
            .map_err(|_| PrepareX402PaymentError::InvalidInput("invalid requester authority"))?;
        let mut offers = Vec::new();
        let mut original_indices = Vec::new();
        for (index, requirement) in args.payment_required.accepts.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| {
                PrepareX402PaymentError::InvalidInput("too many payment requirements")
            })?;
            let PaymentRequirement::ExactSvm(offer) = requirement else {
                continue;
            };
            let request = match prepare_offer(offer) {
                Ok(wire) => wire,
                Err(_) if args.accepted_index == Some(index) => {
                    return Err(PrepareX402PaymentError::InvalidInput(
                        "selected payment requirement is not usable by the backend",
                    ))
                }
                Err(_) => continue,
            };
            original_indices.push(index);
            offers.push(request);
        }
        if !offers
            .iter()
            .any(|offer| offer.network == solana_network(self.network))
        {
            return Err(PrepareX402PaymentError::InvalidInput(
                "no eligible payment requirement for the client's network",
            ));
        }
        let accepted_index = args
            .accepted_index
            .map(|original| {
                let index = original_indices
                    .iter()
                    .position(|index| *index == original)
                    .ok_or(PrepareX402PaymentError::InvalidInput(
                        "selected payment requirement is unsupported or out of range",
                    ))?;
                if offers
                    .get(index)
                    .is_none_or(|offer| offer.network != solana_network(self.network))
                {
                    return Err(PrepareX402PaymentError::InvalidInput(
                        "selected payment network does not match the client",
                    ));
                }
                u32::try_from(index)
                    .map_err(|_| PrepareX402PaymentError::InvalidInput("invalid accepted index"))
            })
            .transpose()?;
        // The backend accepts typed offers, not unsupported placeholders. Restore
        // the original index after its response so the caller's challenge is intact.
        let resource = &args.payment_required.resource;
        let request = PrepareX402PaymentRequest {
            payment_required: api::PaymentRequired {
                x402_version: u32::from(args.payment_required.x402_version),
                resource: api::ResourceInfo {
                    url: resource.url.clone(),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    service_name: resource.service_name.clone(),
                    tags: resource.tags.clone().unwrap_or_default(),
                    icon_url: resource.icon_url.clone(),
                },
                accepts: offers,
                error: args.payment_required.error.clone(),
            },
            accepted_index,
            network: self.network.into(),
            swig_address: args.swig_config_address,
            requester_authority: authority,
        };

        let expected_index = match request.accepted_index {
            Some(index) => index,
            None => request
                .payment_required
                .accepts
                .iter()
                .position(|offer| offer.network == solana_network(self.network))
                .and_then(|index| u32::try_from(index).ok())
                .ok_or(PrepareX402PaymentError::InvalidInput(
                    "no eligible payment requirement",
                ))?,
        };
        let response =
            prepare_x402_payment(&self.http, request)
                .await
                .map_err(|error| match error {
                    HttpError::InvalidUrl | HttpError::RequestFailed => {
                        PrepareX402PaymentError::RequestFailed
                    }
                    HttpError::Timeout => PrepareX402PaymentError::RequestTimedOut,
                    HttpError::ResponseTooLarge => {
                        PrepareX402PaymentError::InvalidResponse("response exceeds size limit")
                    }
                    HttpError::InvalidResponse(message) => {
                        PrepareX402PaymentError::InvalidResponse(message)
                    }
                    HttpError::Unauthenticated => PrepareX402PaymentError::AuthenticationFailed,
                    HttpError::PermissionDenied => PrepareX402PaymentError::PermissionDenied,
                    HttpError::InvalidArgument => {
                        PrepareX402PaymentError::InvalidInput("service rejected payment input")
                    }
                    HttpError::Unavailable => PrepareX402PaymentError::ServiceUnavailable,
                    _ => PrepareX402PaymentError::UnexpectedServiceFailure,
                })?;
        let (prepared_transaction, accepted_index) = normalize_prepare_x402_response(
            response,
            &original_indices,
            expected_index,
            args.swig_config_address,
            self.network,
        )?;
        Ok(X402PaymentPreparation {
            prepared_transaction,
            payment_required: args.payment_required,
            accepted_index,
        })
    }
}

fn normalize_prepare_x402_response(
    response: PrepareX402PaymentResponse,
    original_indices: &[u32],
    expected_index: u32,
    swig_config_address: Pubkey,
    network: Network,
) -> Result<(PreparedTransaction, u32), PrepareX402PaymentError> {
    let accepted_index =
        response
            .accepted_index
            .ok_or(PrepareX402PaymentError::InvalidResponse(
                "invalid payment preparation",
            ))?;
    let index = usize::try_from(accepted_index)
        .map_err(|_| PrepareX402PaymentError::InvalidResponse("invalid accepted index"))?;
    let original_index =
        original_indices
            .get(index)
            .copied()
            .ok_or(PrepareX402PaymentError::InvalidResponse(
                "accepted index is out of range",
            ))?;
    if expected_index != accepted_index {
        return Err(PrepareX402PaymentError::InvalidResponse(
            "selected payment offer does not match the request",
        ));
    }
    if response.prepared_transaction.kind != Some(PreparedTransactionKind::X402Payment) {
        return Err(PrepareX402PaymentError::InvalidResponse(
            "prepared transaction is not an x402 payment",
        ));
    }
    let prepared = PreparedTransaction::try_from(response.prepared_transaction).map_err(|_| {
        PrepareX402PaymentError::InvalidResponse("invalid prepared payment transaction")
    })?;
    if prepared.network != Some(network) {
        return Err(PrepareX402PaymentError::InvalidResponse(
            "prepared payment network does not match the client",
        ));
    }
    if prepared
        .wallet
        .as_ref()
        .is_none_or(|wallet| wallet.swig_config_address != swig_config_address)
    {
        return Err(PrepareX402PaymentError::InvalidResponse(
            "prepared payment wallet does not match the request",
        ));
    }
    validate_prepared_context(&prepared, Some(swig_config_address), network).map_err(
        |ApiConversionError::Invalid(message)| PrepareX402PaymentError::InvalidResponse(message),
    )?;
    Ok((prepared, original_index))
}

fn prepare_offer(
    value: &ExactSvmPaymentRequirement,
) -> Result<api::PaymentRequirement, PrepareX402PaymentError> {
    if value.amount.starts_with('0')
        || value.amount.is_empty()
        || !value.amount.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(PrepareX402PaymentError::InvalidInput(
            "payment amount must be a canonical positive u64",
        ));
    }
    value
        .amount
        .parse::<u64>()
        .map_err(|_| PrepareX402PaymentError::InvalidInput("payment amount exceeds u64"))?;
    if value
        .extra
        .memo
        .as_ref()
        .is_some_and(|memo| memo.len() > 256)
    {
        return Err(PrepareX402PaymentError::InvalidInput(
            "payment memo exceeds 256 bytes",
        ));
    }
    let seconds = value.max_timeout_seconds;
    if !seconds.is_finite()
        || seconds <= 0.0
        || seconds.fract() != 0.0
        || seconds > f64::from(u32::MAX)
    {
        return Err(PrepareX402PaymentError::InvalidInput(
            "payment timeout must be a positive u32",
        ));
    }
    let last_valid_block_height = value
        .extra
        .last_valid_block_height
        .as_ref()
        .map(|height| {
            if height.is_empty() || !height.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(PrepareX402PaymentError::InvalidInput(
                    "invalid last valid block height",
                ));
            }
            height.parse::<u64>().map_err(|_| {
                PrepareX402PaymentError::InvalidInput("last valid block height exceeds u64")
            })
        })
        .transpose()?;
    Ok(api::PaymentRequirement {
        scheme: value.scheme.clone(),
        network: value.network.clone(),
        amount: value.amount.clone(),
        asset: value.asset,
        pay_to: value.pay_to,
        max_timeout_seconds: seconds as u32,
        extra: api::ExactSvmExtra {
            fee_payer: value.extra.fee_payer,
            memo: value.extra.memo.clone(),
            recent_blockhash: value.extra.recent_blockhash.clone(),
            last_valid_block_height,
        },
    })
}
