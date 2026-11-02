use crate::{
    api::{
        create_participant_set::create_participant_set,
        error::ApiConversionError,
        types::{
            authority::WalletAuthority as ApiWalletAuthority,
            participant_set::{
                CreateParticipantSetRequest, CreateParticipantSetResponse, ParticipantSetId,
            },
        },
    },
    http::HttpError,
    wallet::prepared::validate_prepared_context,
    Network, PreparedTransaction, WalletAuthority, WalletClient,
};
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
        let request = CreateParticipantSetRequest {
            network: self.network.into(),
            swig_address: args.swig_config_address,
            fee_payer: args.fee_payer,
            threshold: args.threshold,
            members: args
                .members
                .into_iter()
                .map(ApiWalletAuthority::try_from)
                .collect::<Result<_, _>>()
                .map_err(|ApiConversionError::Invalid(reason)| {
                    PrepareCreateParticipantSetError::InvalidInput(reason)
                })?,
            set_id: args.set_id.map(ParticipantSetId),
        };
        let response = create_participant_set(&self.http, request)
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
                HttpError::InvalidResponse(message) => {
                    PrepareCreateParticipantSetError::InvalidResponse(message)
                }
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
            response,
            expected_set_id,
            swig_config_address,
            self.network,
        )
    }
}

fn normalize_create_participant_set_response(
    response: CreateParticipantSetResponse,
    expected_set_id: Option<[u8; 32]>,
    swig_config_address: Pubkey,
    network: Network,
) -> Result<ParticipantSetCreationPlan, PrepareCreateParticipantSetError> {
    let set_id = response.set_id.0;
    if expected_set_id.is_some_and(|expected| expected != set_id) {
        return Err(PrepareCreateParticipantSetError::InvalidResponse(
            "participant set identifier mismatch",
        ));
    }
    let transaction = PreparedTransaction::try_from(response.transaction).map_err(
        |ApiConversionError::Invalid(reason)| {
            PrepareCreateParticipantSetError::InvalidResponse(reason)
        },
    )?;
    validate_prepared_context(&transaction, Some(swig_config_address), network).map_err(
        |ApiConversionError::Invalid(reason)| {
            PrepareCreateParticipantSetError::InvalidResponse(reason)
        },
    )?;
    Ok(ParticipantSetCreationPlan {
        participant_set_address: response.participant_set_address,
        set_id,
        transaction,
    })
}
