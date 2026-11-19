use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

use crate::{
    api::sign::sign, http::HttpError, types::transaction::decode_transaction_bytes, PaymasterClient,
};

pub struct SignTransactionArgs {
    pub transaction: VersionedTransaction,
    pub external_id: Option<String>,
}

pub struct SignedTransaction {
    pub request_id: String,
    pub signed_transaction: VersionedTransaction,
}

#[derive(Debug, Error)]
pub enum PaymasterSignError {
    #[error("invalid signing input: {0}")]
    InvalidInput(&'static str),
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("permission denied")]
    PermissionDenied,
    #[error("paymaster not found")]
    PaymasterNotFound,
    #[error("paymaster precondition failed")]
    FailedPrecondition,
    #[error("paymaster quota exhausted")]
    ResourceExhausted,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("request timed out")]
    RequestTimedOut,
    #[error("request could not be completed")]
    RequestFailed,
    #[error("invalid signing response: {0}")]
    InvalidResponse(&'static str),
    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl PaymasterClient {
    /// Requests the fee payer's signature once, leaving broadcast to the caller.
    pub async fn sign(
        &self,
        args: SignTransactionArgs,
    ) -> Result<SignedTransaction, PaymasterSignError> {
        let SignTransactionArgs {
            transaction,
            external_id,
        } = args;
        transaction
            .sanitize()
            .map_err(|_| PaymasterSignError::InvalidInput("invalid transaction"))?;
        let mut bytes = bincode::serialize(&transaction)
            .map_err(|_| PaymasterSignError::InvalidInput("transaction serialization failed"))?;
        let response = sign(&self.http, &bytes, self.network.into(), external_id)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PaymasterSignError::RequestFailed
                }
                HttpError::Timeout => PaymasterSignError::RequestTimedOut,
                HttpError::InvalidResponse(message) => PaymasterSignError::InvalidResponse(message),
                HttpError::ResponseTooLarge => {
                    PaymasterSignError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PaymasterSignError::AuthenticationFailed,
                HttpError::PermissionDenied => PaymasterSignError::PermissionDenied,
                HttpError::InvalidArgument => PaymasterSignError::InvalidInput("signing rejected"),
                HttpError::NotFound => PaymasterSignError::PaymasterNotFound,
                HttpError::FailedPrecondition => PaymasterSignError::FailedPrecondition,
                HttpError::ResourceExhausted => PaymasterSignError::ResourceExhausted,
                HttpError::Unavailable => PaymasterSignError::ServiceUnavailable,
                HttpError::AlreadyExists | HttpError::Aborted | HttpError::UnexpectedStatus(_) => {
                    PaymasterSignError::UnexpectedServiceFailure
                }
            })?;
        if response.request_id.trim().is_empty() {
            return Err(PaymasterSignError::InvalidResponse("missing request ID"));
        }

        // Replacing the fee payer's signature cannot change the serialized length.
        let size = bs58::decode(response.signed_transaction)
            .onto(bytes.as_mut_slice())
            .map_err(|_| PaymasterSignError::InvalidResponse("invalid signed transaction"))?;
        if size != bytes.len() {
            return Err(PaymasterSignError::InvalidResponse(
                "invalid signed transaction length",
            ));
        }
        let signed_transaction = decode_transaction_bytes(&bytes)
            .map_err(|_| PaymasterSignError::InvalidResponse("invalid signed transaction"))?;
        if signed_transaction.message != transaction.message {
            return Err(PaymasterSignError::InvalidResponse(
                "transaction message changed",
            ));
        }
        let (payer_signature, other_signatures) =
            signed_transaction.signatures.split_first().ok_or(
                PaymasterSignError::InvalidResponse("missing fee payer signature"),
            )?;
        if transaction.signatures.get(1..) != Some(other_signatures) {
            return Err(PaymasterSignError::InvalidResponse(
                "nonpayer signatures changed",
            ));
        }
        let payer = transaction
            .message
            .static_account_keys()
            .first()
            .ok_or(PaymasterSignError::InvalidInput("missing fee payer"))?;
        if !payer_signature.verify(payer.as_ref(), &transaction.message.serialize()) {
            return Err(PaymasterSignError::InvalidResponse(
                "invalid fee payer signature",
            ));
        }

        Ok(SignedTransaction {
            request_id: response.request_id,
            signed_transaction,
        })
    }
}
