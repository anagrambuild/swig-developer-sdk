use crate::{
    api::{
        authorization_locks as api, error::ApiConversionError,
        types::encoding::PreparedTransactionKind,
    },
    http::HttpError,
    wallet::prepared::validate_prepared_context,
    PreparedTransaction, WalletAuthority, WalletClient,
};
use solana_pubkey::Pubkey;
use thiserror::Error;

/// Exactly one absolute expiry or relative duration, measured in Solana slots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationLockExpiry {
    DurationSlots(u64),
    ExpirySlot(u64),
}

#[derive(Clone)]
pub struct PrepareAddAuthorizationLockArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    /// None reserves native SOL; Some reserves the wallet's token account for the mint.
    pub mint: Option<Pubkey>,
    pub amount: u64,
    pub expiry: AuthorizationLockExpiry,
}

#[derive(Clone)]
pub struct PrepareRemoveAuthorizationLockArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub lock_id: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationLock {
    pub lock_id: u32,
    pub role_id: u32,
    pub asset_account: Pubkey,
    pub mint: Option<Pubkey>,
    /// Remaining reservation; zero means the hold was fully captured.
    pub amount: u64,
    pub expiry_slot: u64,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationLocksResult {
    pub locks: Vec<AuthorizationLock>,
    pub current_slot: u64,
    pub next_lock_id: u32,
}

#[derive(Debug, Error)]
pub enum AuthorizationLockError {
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

impl From<HttpError> for AuthorizationLockError {
    fn from(error: HttpError) -> Self {
        match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => Self::RequestFailed,
            HttpError::Timeout => Self::RequestTimedOut,
            HttpError::ResponseTooLarge => Self::InvalidResponse("response exceeds size limit"),
            HttpError::InvalidResponse(message) => Self::InvalidResponse(message),
            HttpError::Unauthenticated => Self::AuthenticationFailed,
            HttpError::PermissionDenied => Self::PermissionDenied,
            HttpError::InvalidArgument => Self::InvalidInput("service rejected the request"),
            HttpError::Unavailable => Self::ServiceUnavailable,
            HttpError::NotFound
            | HttpError::AlreadyExists
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => Self::UnexpectedServiceFailure,
        }
    }
}

impl WalletClient {
    /// Prepare a hold. The role needs ManageAuthorizationLocks even when it has All.
    pub async fn prepare_add_authorization_lock(
        &self,
        args: PrepareAddAuthorizationLockArgs,
    ) -> Result<PreparedTransaction, AuthorizationLockError> {
        if args.amount == 0 {
            return Err(AuthorizationLockError::InvalidInput(
                "amount must be positive",
            ));
        }
        let expiry = match args.expiry {
            AuthorizationLockExpiry::DurationSlots(0) | AuthorizationLockExpiry::ExpirySlot(0) => {
                return Err(AuthorizationLockError::InvalidInput(
                    "expiry must be positive",
                ))
            }
            AuthorizationLockExpiry::DurationSlots(slots) => {
                api::AuthorizationLockExpiry::DurationSlots(slots)
            }
            AuthorizationLockExpiry::ExpirySlot(slot) => {
                api::AuthorizationLockExpiry::ExpirySlot(slot)
            }
        };
        let response = api::prepare_add_authorization_lock(
            &self.http,
            api::AddAuthorizationLockRequest {
                network: self.network.into(),
                swig_address: args.swig_config_address,
                fee_payer: args.fee_payer,
                requester_authority: args.requester_authority.try_into().map_err(
                    |ApiConversionError::Invalid(message)| {
                        AuthorizationLockError::InvalidInput(message)
                    },
                )?,
                mint: args.mint,
                amount: args.amount,
                expiry,
            },
        )
        .await?;
        if response
            .kind
            .is_some_and(|kind| kind != PreparedTransactionKind::AddAuthorizationLock)
        {
            return Err(AuthorizationLockError::InvalidResponse(
                "unexpected prepared transaction kind",
            ));
        }
        let prepared = PreparedTransaction::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| AuthorizationLockError::InvalidResponse(message),
        )?;
        validate_prepared_context(&prepared, Some(args.swig_config_address), self.network)
            .map_err(|ApiConversionError::Invalid(message)| {
                AuthorizationLockError::InvalidResponse(message)
            })?;
        Ok(prepared)
    }

    /// Prepare early release by the holder, or pruning by an eligible role after expiry/capture.
    pub async fn prepare_remove_authorization_lock(
        &self,
        args: PrepareRemoveAuthorizationLockArgs,
    ) -> Result<PreparedTransaction, AuthorizationLockError> {
        let response = api::prepare_remove_authorization_lock(
            &self.http,
            api::RemoveAuthorizationLockRequest {
                network: self.network.into(),
                swig_address: args.swig_config_address,
                fee_payer: args.fee_payer,
                requester_authority: args.requester_authority.try_into().map_err(
                    |ApiConversionError::Invalid(message)| {
                        AuthorizationLockError::InvalidInput(message)
                    },
                )?,
                lock_id: args.lock_id,
            },
        )
        .await?;
        if response
            .kind
            .is_some_and(|kind| kind != PreparedTransactionKind::RemoveAuthorizationLock)
        {
            return Err(AuthorizationLockError::InvalidResponse(
                "unexpected prepared transaction kind",
            ));
        }
        let prepared = PreparedTransaction::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| AuthorizationLockError::InvalidResponse(message),
        )?;
        validate_prepared_context(&prepared, Some(args.swig_config_address), self.network)
            .map_err(|ApiConversionError::Invalid(message)| {
                AuthorizationLockError::InvalidResponse(message)
            })?;
        Ok(prepared)
    }

    /// Read holds at the service's current slot; include expired and captured records when requested.
    pub async fn get_authorization_locks(
        &self,
        swig_config_address: &Pubkey,
        include_expired: bool,
    ) -> Result<AuthorizationLocksResult, AuthorizationLockError> {
        let response = api::get_authorization_locks(
            &self.http,
            swig_config_address,
            self.network,
            include_expired,
        )
        .await?;
        let mut locks = Vec::with_capacity(response.locks.len());
        let mut previous = None;
        for lock in response.locks {
            if lock.active != (lock.amount > 0 && lock.expiry_slot > response.current_slot)
                || lock.lock_id >= response.next_lock_id
                || previous.is_some_and(|id| id >= lock.lock_id)
                || (!include_expired && !lock.active)
            {
                return Err(AuthorizationLockError::InvalidResponse(
                    "inconsistent authorization lock state",
                ));
            }
            previous = Some(lock.lock_id);
            locks.push(AuthorizationLock {
                lock_id: lock.lock_id,
                role_id: lock.role_id,
                asset_account: lock.asset_account,
                mint: lock.mint,
                amount: lock.amount,
                expiry_slot: lock.expiry_slot,
                active: lock.active,
            });
        }
        Ok(AuthorizationLocksResult {
            locks,
            current_slot: response.current_slot,
            next_lock_id: response.next_lock_id,
        })
    }
}
