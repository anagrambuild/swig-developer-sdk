use super::prepared::normalize_batch;
use crate::{
    api::{error::ApiConversionError, types::authority::WalletAuthority as ApiWalletAuthority},
    http::HttpError,
    Network, WalletAddressInfo, WalletAuthority, WalletClient, WalletCreation,
};
use solana_pubkey::Pubkey;
use thiserror::Error;

/// Omission selects account index zero; reuse the same value at creation.
#[derive(Clone, Copy)]
pub enum ReservationAddressOptions {
    AccountIndex(u32),
    Salt([u8; 32]),
}

#[derive(Clone)]
pub struct ReserveWalletAddressArgs {
    pub root_authority: WalletAuthority,
    pub address_options: Option<ReservationAddressOptions>,
}

#[derive(Clone)]
pub struct PrepareCreateReservedWalletArgs {
    pub root_authority: WalletAuthority,
    pub address_options: Option<ReservationAddressOptions>,
    pub fee_payer: Pubkey,
}

#[derive(Clone)]
pub struct WalletReservation {
    pub wallet: WalletAddressInfo,
    pub network: Network,
}

#[derive(Debug, Error)]
pub enum WalletReservationError {
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
    /// Derives an address without creating or submitting a transaction.
    pub async fn reserve_address(
        &self,
        args: ReserveWalletAddressArgs,
    ) -> Result<WalletReservation, WalletReservationError> {
        let authority = reservation_authority(args.root_authority)?;
        let response = crate::api::reservation::reserve_address(
            &self.http,
            self.network.into(),
            authority,
            args.address_options,
        )
        .await?;
        let network = Network::try_from(response.network).map_err(
            |ApiConversionError::Invalid(message)| WalletReservationError::InvalidResponse(message),
        )?;
        if network != self.network {
            return Err(WalletReservationError::InvalidResponse(
                "mismatched reservation network",
            ));
        }
        Ok(WalletReservation {
            wallet: WalletAddressInfo {
                swig_config_address: response.wallet.swig_config_address,
                wallet_address: response.wallet.wallet_address,
            },
            network,
        })
    }

    /// Prepares activation for the same authority/index or salt; only the payer signs.
    pub async fn prepare_create_reserved_wallet(
        &self,
        args: PrepareCreateReservedWalletArgs,
    ) -> Result<WalletCreation, WalletReservationError> {
        let authority = reservation_authority(args.root_authority)?;
        let response = crate::api::reservation::prepare_create_reserved(
            &self.http,
            self.network.into(),
            authority,
            args.address_options,
            args.fee_payer,
        )
        .await?;
        if response.transactions.len() != 1
            || response.wallet.is_none()
            || response.network.is_none()
        {
            return Err(WalletReservationError::InvalidResponse(
                "incomplete reserved wallet creation",
            ));
        }
        let prepared = normalize_batch(response, None, self.network).map_err(
            |ApiConversionError::Invalid(message)| WalletReservationError::InvalidResponse(message),
        )?;
        if !prepared.signature_requests.is_empty()
            || prepared.participant_set_approval_plan.is_some()
        {
            return Err(WalletReservationError::InvalidResponse(
                "reserved activation must only require the fee payer",
            ));
        }
        let wallet = prepared
            .wallet
            .clone()
            .ok_or(WalletReservationError::InvalidResponse(
                "missing reserved wallet",
            ))?;
        Ok(WalletCreation {
            wallet,
            transaction: prepared,
        })
    }
}

fn reservation_authority(
    authority: WalletAuthority,
) -> Result<ApiWalletAuthority, WalletReservationError> {
    if matches!(authority, WalletAuthority::ParticipantSet { .. }) {
        return Err(WalletReservationError::InvalidInput(
            "reservation requires a direct signing-key root authority",
        ));
    }
    ApiWalletAuthority::try_from(authority).map_err(|ApiConversionError::Invalid(message)| {
        WalletReservationError::InvalidInput(message)
    })
}

impl From<HttpError> for WalletReservationError {
    fn from(error: HttpError) -> Self {
        match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => Self::RequestFailed,
            HttpError::InvalidResponse(message) => Self::InvalidResponse(message),
            HttpError::Timeout => Self::RequestTimedOut,
            HttpError::ResponseTooLarge => Self::InvalidResponse("response exceeds size limit"),
            HttpError::Unauthenticated => Self::AuthenticationFailed,
            HttpError::PermissionDenied => Self::PermissionDenied,
            HttpError::InvalidArgument => Self::InvalidInput("service rejected the request"),
            HttpError::Unavailable => Self::ServiceUnavailable,
            _ => Self::UnexpectedServiceFailure,
        }
    }
}
