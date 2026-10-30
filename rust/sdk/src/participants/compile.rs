use crate::{
    api::{
        compile_participant_set_approvals::compile_participant_set_approvals,
        error::ApiConversionError,
        types::{
            participant_set::{
                CompileParticipantSetApprovalsRequest, CompileParticipantSetApprovalsResponse,
            },
            transaction::PreparedTransaction as ApiPreparedTransaction,
        },
    },
    http::HttpError,
    wallet::prepared::validate_prepared_context,
    Network, ParticipantApproval, PreparedTransaction, WalletAddressInfo, WalletClient,
};
use solana_transaction::versioned::VersionedTransaction;
use std::collections::BTreeSet;
use thiserror::Error;
#[derive(Clone)]
pub struct CompileParticipantSetApprovalsArgs {
    pub prepared_transaction: PreparedTransaction,
    pub approvals: Vec<ParticipantApproval>,
}

#[derive(Clone)]
pub struct ParticipantSetApprovalCompilation {
    pub transaction: VersionedTransaction,
    pub authorization_expiration_slot: u64,
}
#[derive(Debug, Error)]
pub enum CompileParticipantSetApprovalsError {
    #[error("prepared transaction has no participant approval plan")]
    MissingApprovalPlan,

    #[error("approval threshold requires {required}, received {received}")]
    InsufficientApprovals { required: u32, received: usize },

    #[error("invalid compilation input: {0}")]
    InvalidInput(&'static str),

    #[error("participant approval compilation was rejected")]
    CompilationRejected,

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
    /// Compiles detached approvals without submitting the transaction.
    ///
    /// The original envelope is preserved. The hosted compiler verifies
    /// proofs and freshness. Outer Solana signing follows compilation.
    pub async fn compile_participant_set_approvals(
        &self,
        args: CompileParticipantSetApprovalsArgs,
    ) -> Result<ParticipantSetApprovalCompilation, CompileParticipantSetApprovalsError> {
        let plan = args
            .prepared_transaction
            .participant_set_approval_plan
            .as_ref()
            .ok_or(CompileParticipantSetApprovalsError::MissingApprovalPlan)?;
        if args.prepared_transaction.network != Some(self.network) {
            return Err(CompileParticipantSetApprovalsError::InvalidInput(
                "prepared transaction network must match client",
            ));
        }
        let mut member_indexes = BTreeSet::new();
        for approval in &args.approvals {
            let index = match approval {
                ParticipantApproval::Ed25519 { member_index, .. }
                | ParticipantApproval::Secp256k1 { member_index, .. }
                | ParticipantApproval::WebAuthnP256 { member_index, .. } => *member_index,
            };
            if !member_indexes.insert(index) {
                return Err(CompileParticipantSetApprovalsError::InvalidInput(
                    "duplicate participant approval index",
                ));
            }
        }
        if args.approvals.len() < plan.threshold as usize {
            return Err(CompileParticipantSetApprovalsError::InsufficientApprovals {
                required: plan.threshold,
                received: args.approvals.len(),
            });
        }
        let expected_wallet = args.prepared_transaction.wallet.clone();
        let request = CompileParticipantSetApprovalsRequest {
            prepared_transaction: ApiPreparedTransaction::try_from(args.prepared_transaction)
                .map_err(|ApiConversionError::Invalid(reason)| {
                    CompileParticipantSetApprovalsError::InvalidInput(reason)
                })?,
            approvals: args.approvals.into_iter().map(Into::into).collect(),
        };

        let response = compile_participant_set_approvals(&self.http, request)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    CompileParticipantSetApprovalsError::RequestFailed
                }
                HttpError::Timeout => CompileParticipantSetApprovalsError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    CompileParticipantSetApprovalsError::InvalidResponse(
                        "response exceeds size limit",
                    )
                }
                HttpError::Unauthenticated => {
                    CompileParticipantSetApprovalsError::AuthenticationFailed
                }
                HttpError::PermissionDenied => {
                    CompileParticipantSetApprovalsError::PermissionDenied
                }
                HttpError::InvalidArgument | HttpError::FailedPrecondition => {
                    CompileParticipantSetApprovalsError::CompilationRejected
                }
                HttpError::Unavailable => CompileParticipantSetApprovalsError::ServiceUnavailable,
                HttpError::InvalidResponse(message) => {
                    CompileParticipantSetApprovalsError::InvalidResponse(message)
                }
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    CompileParticipantSetApprovalsError::UnexpectedServiceFailure
                }
            })?;
        normalize_compile_participant_set_response(response, expected_wallet, self.network)
    }
}
fn normalize_compile_participant_set_response(
    response: CompileParticipantSetApprovalsResponse,
    expected_wallet: Option<WalletAddressInfo>,
    network: Network,
) -> Result<ParticipantSetApprovalCompilation, CompileParticipantSetApprovalsError> {
    let prepared = PreparedTransaction::try_from(response.transaction).map_err(
        |ApiConversionError::Invalid(reason)| {
            CompileParticipantSetApprovalsError::InvalidResponse(reason)
        },
    )?;
    validate_prepared_context(
        &prepared,
        expected_wallet
            .as_ref()
            .map(|wallet| wallet.swig_config_address),
        network,
    )
    .map_err(|ApiConversionError::Invalid(reason)| {
        CompileParticipantSetApprovalsError::InvalidResponse(reason)
    })?;
    if let (Some(expected), Some(actual)) = (&expected_wallet, &prepared.wallet) {
        if expected.wallet_address != actual.wallet_address {
            return Err(CompileParticipantSetApprovalsError::InvalidResponse(
                "response wallet mismatch",
            ));
        }
    }
    if !prepared.signature_requests.is_empty() || prepared.participant_set_approval_plan.is_some() {
        return Err(CompileParticipantSetApprovalsError::InvalidResponse(
            "compilation retained unresolved approval requests",
        ));
    }
    Ok(ParticipantSetApprovalCompilation {
        transaction: prepared.transaction,
        authorization_expiration_slot: response.authorization_expiration_slot,
    })
}
