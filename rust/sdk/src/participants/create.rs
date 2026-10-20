use crate::{
    http::HttpError,
    types::wire::{
        participants::{CreateParticipantSetRequest, ParticipantSetCreationWire},
        transaction::validate_prepared_transaction_shape,
        wallet::prepared::validate_prepared_context,
        WireError,
    },
    Network, PreparedTransaction, WalletAuthority, WalletClient,
};
use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;
#[derive(Clone)]
pub struct PrepareCreateParticipantSetArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub threshold: u32,
    pub members: Vec<WalletAuthority>,
    pub set_id: Option<[u8; 32]>,
}

#[derive(Clone)]
pub struct ParticipantSetCreationPlan {
    pub participant_set_address: Pubkey,
    pub set_id: [u8; 32],
    pub transaction: PreparedTransaction,
}

#[derive(Debug, Error)]
pub enum PrepareCreateParticipantSetError {
    #[error("threshold {threshold} is invalid for {participant_count} participants")]
    InvalidThreshold {
        threshold: u32,
        participant_count: usize,
    },

    #[error("duplicate participant at index {index}")]
    DuplicateParticipant { index: usize },

    #[error("invalid participant set input: {0}")]
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
    /// Prepares participant-set creation without collecting approvals or submitting it.
    pub async fn prepare_create_participant_set(
        &self,
        args: PrepareCreateParticipantSetArgs,
    ) -> Result<ParticipantSetCreationPlan, PrepareCreateParticipantSetError> {
        if !(1..=8).contains(&args.members.len()) {
            return Err(PrepareCreateParticipantSetError::InvalidInput(
                "expected one to eight members",
            ));
        }
        if args.threshold == 0 || args.threshold as usize > args.members.len() {
            return Err(PrepareCreateParticipantSetError::InvalidThreshold {
                threshold: args.threshold,
                participant_count: args.members.len(),
            });
        }
        for (index, member) in args.members.iter().enumerate() {
            if matches!(member, WalletAuthority::ParticipantSet { .. }) {
                return Err(PrepareCreateParticipantSetError::InvalidInput(
                    "participant sets cannot be members",
                ));
            }
            if args
                .members
                .iter()
                .take(index)
                .any(|previous| previous == member)
            {
                return Err(PrepareCreateParticipantSetError::DuplicateParticipant { index });
            }
        }
        let swig_config_address = args.swig_config_address;
        let expected_set_id = args.set_id;
        let body = CreateParticipantSetRequest::from_args(self.network, args).map_err(
            |WireError::Invalid(reason)| PrepareCreateParticipantSetError::InvalidInput(reason),
        )?;

        let body = serde_json::to_vec(&body)
            .map_err(|_| PrepareCreateParticipantSetError::RequestFailed)?;
        let bytes = self
            .http
            .post("/transaction/wallet/participant-set/create", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareCreateParticipantSetError::RequestFailed
                }
                HttpError::Timeout => PrepareCreateParticipantSetError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareCreateParticipantSetError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => {
                    PrepareCreateParticipantSetError::AuthenticationFailed
                }
                HttpError::PermissionDenied => PrepareCreateParticipantSetError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareCreateParticipantSetError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => PrepareCreateParticipantSetError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    PrepareCreateParticipantSetError::UnexpectedServiceFailure
                }
            })?;
        normalize_create_participant_set_response(
            &bytes,
            expected_set_id,
            swig_config_address,
            self.network,
        )
    }
}

fn normalize_create_participant_set_response(
    bytes: &[u8],
    expected_set_id: Option<[u8; 32]>,
    swig_config_address: Pubkey,
    network: Network,
) -> Result<ParticipantSetCreationPlan, PrepareCreateParticipantSetError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| PrepareCreateParticipantSetError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(PrepareCreateParticipantSetError::InvalidResponse(
            "malformed participant set creation",
        ));
    }
    if let Some(transaction) = payload.get("transaction") {
        validate_prepared_transaction_shape(transaction).map_err(|_| {
            PrepareCreateParticipantSetError::InvalidResponse("malformed participant set creation")
        })?;
    }
    let wire: ParticipantSetCreationWire = serde_json::from_value(payload).map_err(|_| {
        PrepareCreateParticipantSetError::InvalidResponse("malformed participant set creation")
    })?;
    let set_id = wire.decode_set_id().map_err(|WireError::Invalid(reason)| {
        PrepareCreateParticipantSetError::InvalidResponse(reason)
    })?;
    if expected_set_id.is_some_and(|expected| expected != set_id) {
        return Err(PrepareCreateParticipantSetError::InvalidResponse(
            "participant set identifier mismatch",
        ));
    }
    let transaction =
        PreparedTransaction::try_from(wire.transaction).map_err(|WireError::Invalid(reason)| {
            PrepareCreateParticipantSetError::InvalidResponse(reason)
        })?;
    validate_prepared_context(&transaction, Some(swig_config_address), network).map_err(
        |WireError::Invalid(reason)| PrepareCreateParticipantSetError::InvalidResponse(reason),
    )?;
    Ok(ParticipantSetCreationPlan {
        participant_set_address: wire.participant_set_address.parse().map_err(|_| {
            PrepareCreateParticipantSetError::InvalidResponse("invalid participant set address")
        })?,
        set_id,
        transaction,
    })
}
