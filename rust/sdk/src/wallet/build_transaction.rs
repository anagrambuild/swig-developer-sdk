use crate::WalletAuthority;
use crate::{
    api::{
        error::ApiConversionError,
        prepare_transaction::prepare_transaction,
        types::{
            authority::WalletAuthority as ApiWalletAuthority,
            custom_transaction::{
                PrepareCustomTransactionRequest, SolanaAccountMeta, SolanaInstruction,
            },
        },
    },
    http::HttpError,
    wallet::prepared::validate_prepared_context,
    PreparedTransaction, WalletClient,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Clone)]
pub struct PrepareTransactionArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub instructions: Vec<Instruction>,
    pub address_lookup_table_accounts: Vec<Pubkey>,
}

#[derive(Debug, Error)]
pub enum PrepareTransactionError {
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
    /// Prepares custom instructions for execution through the wallet.
    pub async fn prepare_transaction(
        &self,
        args: PrepareTransactionArgs,
    ) -> Result<PreparedTransaction, PrepareTransactionError> {
        if args.instructions.is_empty() {
            return Err(PrepareTransactionError::InvalidInput(
                "instructions must not be empty",
            ));
        }
        let mut lookups = std::collections::BTreeSet::new();
        if args
            .address_lookup_table_accounts
            .iter()
            .any(|address| !lookups.insert(*address))
        {
            return Err(PrepareTransactionError::InvalidInput(
                "duplicate lookup table address",
            ));
        }
        if matches!(args.requester_authority, WalletAuthority::Secp256k1 { .. }) {
            return Err(PrepareTransactionError::InvalidInput(
                "unsupported requester authority",
            ));
        }
        if matches!(
            args.requester_authority,
            WalletAuthority::ParticipantSet { .. }
        ) && !args.address_lookup_table_accounts.is_empty()
        {
            return Err(PrepareTransactionError::InvalidInput(
                "participant-set requests do not support lookup tables",
            ));
        }

        let (wallet_address, _) = Pubkey::try_find_program_address(
            &[b"swig-wallet-address", args.swig_config_address.as_ref()],
            &solana_pubkey::pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB"),
        )
        .ok_or(PrepareTransactionError::InvalidInput(
            "wallet address derivation failed",
        ))?;
        for account in args
            .instructions
            .iter()
            .flat_map(|instruction| &instruction.accounts)
        {
            let requester = matches!(
                &args.requester_authority,
                WalletAuthority::Ed25519 { public_key } if *public_key == account.pubkey
            );
            if account.is_signer
                && account.pubkey != wallet_address
                && account.pubkey != args.fee_payer
                && !requester
            {
                return Err(PrepareTransactionError::InvalidInput(
                    "unsupported instruction signer",
                ));
            }
        }

        let swig_config_address = args.swig_config_address;
        let request = PrepareCustomTransactionRequest {
            network: self.network.into(),
            swig_address: swig_config_address,
            fee_payer: args.fee_payer,
            requester_authority: ApiWalletAuthority::try_from(args.requester_authority).map_err(
                |ApiConversionError::Invalid(message)| {
                    PrepareTransactionError::InvalidInput(message)
                },
            )?,
            instructions: args
                .instructions
                .into_iter()
                .map(|instruction| SolanaInstruction {
                    program_id: instruction.program_id,
                    accounts: instruction
                        .accounts
                        .into_iter()
                        .map(|account| SolanaAccountMeta {
                            pubkey: account.pubkey,
                            is_signer: account.is_signer,
                            is_writable: account.is_writable,
                        })
                        .collect(),
                    data: instruction.data,
                })
                .collect(),
            address_lookup_table_accounts: args.address_lookup_table_accounts,
        };
        let response =
            prepare_transaction(&self.http, request)
                .await
                .map_err(|error| match error {
                    HttpError::InvalidUrl | HttpError::RequestFailed => {
                        PrepareTransactionError::RequestFailed
                    }
                    HttpError::Timeout => PrepareTransactionError::RequestTimedOut,
                    HttpError::ResponseTooLarge => {
                        PrepareTransactionError::InvalidResponse("response exceeds size limit")
                    }
                    HttpError::InvalidResponse(message) => {
                        PrepareTransactionError::InvalidResponse(message)
                    }
                    HttpError::Unauthenticated => PrepareTransactionError::AuthenticationFailed,
                    HttpError::PermissionDenied => PrepareTransactionError::PermissionDenied,
                    HttpError::InvalidArgument => {
                        PrepareTransactionError::InvalidInput("service rejected the request")
                    }
                    HttpError::Unavailable => PrepareTransactionError::ServiceUnavailable,
                    HttpError::NotFound
                    | HttpError::AlreadyExists
                    | HttpError::FailedPrecondition
                    | HttpError::Aborted
                    | HttpError::ResourceExhausted
                    | HttpError::UnexpectedStatus(_) => {
                        PrepareTransactionError::UnexpectedServiceFailure
                    }
                })?;
        let prepared = PreparedTransaction::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| {
                PrepareTransactionError::InvalidResponse(message)
            },
        )?;
        validate_prepared_context(&prepared, Some(swig_config_address), self.network).map_err(
            |ApiConversionError::Invalid(message)| {
                PrepareTransactionError::InvalidResponse(message)
            },
        )?;
        Ok(prepared)
    }
}
