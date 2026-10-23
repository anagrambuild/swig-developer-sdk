use super::{
    types::{solana_network, validate_required},
    PaymentRequiredV2, PaymentRequirement,
};
use crate::{
    http::HttpError,
    types::wire::{
        transaction::validate_prepared_transaction_shape,
        wallet::prepared::validate_prepared_context,
        x402::{
            ExactSvmBackendWire, PaymentRequiredBackendWire, PrepareX402Request,
            PrepareX402Response,
        },
        AuthorityWire, PreparedTransactionKind, WireError,
    },
    Network, PreparedTransaction, WalletAuthority, WalletClient,
};
use serde_json::Value;
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
        let authority = AuthorityWire::try_from(args.requester_authority)
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
            let wire = match ExactSvmBackendWire::try_from(offer) {
                Ok(wire) => wire,
                Err(_) if args.accepted_index == Some(index) => {
                    return Err(PrepareX402PaymentError::InvalidInput(
                        "selected payment requirement is not usable by the backend",
                    ))
                }
                Err(_) => continue,
            };
            original_indices.push(index);
            offers.push(wire);
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
        let body = PrepareX402Request {
            payment_required: PaymentRequiredBackendWire {
                x402_version: args.payment_required.x402_version,
                resource: args.payment_required.resource.clone(),
                accepts: offers,
                error: args.payment_required.error.clone(),
            },
            accepted_index,
            network: self.network.into(),
            swig_address: args.swig_config_address.to_string(),
            requester_authority: authority,
        };

        let expected_index = match body.accepted_index {
            Some(index) => index,
            None => body
                .payment_required
                .accepts
                .iter()
                .position(|offer| offer.network == solana_network(self.network))
                .and_then(|index| u32::try_from(index).ok())
                .ok_or(PrepareX402PaymentError::InvalidInput(
                    "no eligible payment requirement",
                ))?,
        };
        let bytes = self
            .http
            .post(
                "/transaction/payment/x402/prepare",
                serde_json::to_vec(&body).map_err(|_| PrepareX402PaymentError::RequestFailed)?,
            )
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareX402PaymentError::RequestFailed
                }
                HttpError::Timeout => PrepareX402PaymentError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareX402PaymentError::InvalidResponse("response exceeds size limit")
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
            &bytes,
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
    bytes: &[u8],
    original_indices: &[u32],
    expected_index: u32,
    swig_config_address: Pubkey,
    network: Network,
) -> Result<(PreparedTransaction, u32), PrepareX402PaymentError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| PrepareX402PaymentError::InvalidResponse("invalid payment preparation"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(PrepareX402PaymentError::InvalidResponse(
            "invalid payment preparation",
        ));
    }
    if let Some(transaction) = payload
        .get("preparedTransaction")
        .or_else(|| payload.get("prepared_transaction"))
    {
        validate_prepared_transaction_shape(transaction)
            .map_err(|_| PrepareX402PaymentError::InvalidResponse("invalid payment preparation"))?;
    }
    let response: PrepareX402Response = serde_json::from_value(payload)
        .map_err(|_| PrepareX402PaymentError::InvalidResponse("invalid payment preparation"))?;
    let index = usize::try_from(response.accepted_index)
        .map_err(|_| PrepareX402PaymentError::InvalidResponse("invalid accepted index"))?;
    let original_index =
        original_indices
            .get(index)
            .copied()
            .ok_or(PrepareX402PaymentError::InvalidResponse(
                "accepted index is out of range",
            ))?;
    if expected_index != response.accepted_index {
        return Err(PrepareX402PaymentError::InvalidResponse(
            "selected payment offer does not match the request",
        ));
    }
    if response.prepared_transaction.kind.0 != Some(PreparedTransactionKind::X402Payment) {
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
    validate_prepared_context(&prepared, Some(swig_config_address), network)
        .map_err(|WireError::Invalid(message)| PrepareX402PaymentError::InvalidResponse(message))?;
    Ok((prepared, original_index))
}
