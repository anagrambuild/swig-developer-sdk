use std::{collections::BTreeSet, ops::Range};

use solana_message::{compiled_instruction::CompiledInstruction, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;

use crate::{ClientSignatureRequest, PreparedTransaction, TransactionSignError, WalletAuthority};

const SWIG_PROGRAM: Pubkey = Pubkey::from_str_const("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");
const SECP256R1_PROGRAM: Pubkey =
    Pubkey::from_str_const("Secp256r1SigVerify1111111111111111111111111");
const SECP256K1_PAYLOAD_SIZE: usize = 77;
const SECP256R1_SIGNATURE: Range<usize> = 49..113;
const SECP256R1_PUBLIC_KEY: Range<usize> = 16..49;
const SECP256R1_MESSAGE: Range<usize> = 113..145;

pub(super) struct SigningTarget {
    instruction: usize,
    signature: Range<usize>,
    pub(super) message_hash: [u8; 32],
}

/// Resolve every mutable signature range before any private-key operation.
pub(super) fn preflight(
    prepared: &PreparedTransaction,
    authority: &WalletAuthority,
) -> Result<Vec<SigningTarget>, TransactionSignError> {
    if prepared.participant_set_approval_plan.is_some() {
        return Err(TransactionSignError::ParticipantApprovalsPending);
    }
    if prepared.signature_requests.is_empty() {
        return Ok(Vec::new());
    }
    let transaction = &prepared.transaction;
    transaction.sanitize().map_err(|_| {
        TransactionSignError::InvalidPreparation("invalid Solana transaction structure")
    })?;
    if prepared
        .recent_blockhash
        .as_ref()
        .is_some_and(|hash| hash != transaction.message.recent_blockhash())
    {
        return Err(TransactionSignError::InvalidPreparation(
            "recent blockhash does not match the transaction",
        ));
    }
    if transaction
        .signatures
        .iter()
        .any(|signature| signature.as_ref().iter().any(|byte| *byte != 0))
    {
        return Err(TransactionSignError::OuterSignaturesPresent);
    }
    if prepared
        .signature_requests
        .iter()
        .any(|request| &request.authority != authority)
    {
        return Err(TransactionSignError::AuthorityMismatch);
    }

    let mut claimed = BTreeSet::new();
    let mut targets = Vec::with_capacity(prepared.signature_requests.len());
    for request in &prepared.signature_requests {
        let target = match authority {
            WalletAuthority::Secp256k1 { .. } => secp256k1_target(transaction, request)?,
            WalletAuthority::Secp256r1 { public_key } => {
                secp256r1_target(transaction, public_key, request)?
            }
            _ => return Err(TransactionSignError::AuthorityMismatch),
        };
        if !claimed.insert(target.instruction) {
            return Err(TransactionSignError::DuplicateSigningTarget);
        }
        targets.push(target);
    }
    Ok(targets)
}

pub(super) fn patch_signature(
    transaction: &mut VersionedTransaction,
    target: &SigningTarget,
    signature: &[u8],
) -> Result<(), TransactionSignError> {
    let instructions = match &mut transaction.message {
        VersionedMessage::Legacy(message) => &mut message.instructions,
        VersionedMessage::V0(message) => &mut message.instructions,
    };
    let slot = instructions
        .get_mut(target.instruction)
        .and_then(|instruction| instruction.data.get_mut(target.signature.clone()))
        .ok_or(TransactionSignError::SigningTargetNotFound)?;
    if slot.len() != signature.len() {
        return Err(TransactionSignError::InvalidSignature);
    }
    slot.copy_from_slice(signature);
    Ok(())
}

fn program_id(
    transaction: &VersionedTransaction,
    instruction: &CompiledInstruction,
) -> Result<Pubkey, TransactionSignError> {
    transaction
        .message
        .static_account_keys()
        .get(usize::from(instruction.program_id_index))
        .copied()
        .ok_or(TransactionSignError::InvalidPreparation(
            "instruction program ID must be a static account",
        ))
}

fn secp256k1_target(
    transaction: &VersionedTransaction,
    request: &ClientSignatureRequest,
) -> Result<SigningTarget, TransactionSignError> {
    let mut matching = None;
    for (index, instruction) in transaction.message.instructions().iter().enumerate() {
        if program_id(transaction, instruction)? != SWIG_PROGRAM
            || instruction.data.get(..2) != Some(11_u16.to_le_bytes().as_slice())
        {
            continue;
        }
        let data = &instruction.data;
        if data.len() < 8 {
            return Err(TransactionSignError::InvalidPreparation(
                "truncated Swig SignV2 header",
            ));
        }
        let payload = 8_usize.checked_add(usize::from(read_u16(data, 2)?)).ok_or(
            TransactionSignError::InvalidPreparation("invalid Swig authority offset"),
        )?;
        let end = payload.checked_add(SECP256K1_PAYLOAD_SIZE).ok_or(
            TransactionSignError::InvalidPreparation("invalid Swig authority offset"),
        )?;
        let Some(authority_payload) = data.get(payload..end) else {
            // Other authority variants also use SignV2 with shorter payloads.
            continue;
        };
        if authority_payload.get(..8) != Some(request.slot.to_le_bytes().as_slice())
            || authority_payload.get(8..12) != Some(request.counter.to_le_bytes().as_slice())
        {
            continue;
        }
        if data.len() != end {
            return Err(TransactionSignError::InvalidPreparation(
                "raw secp256k1 signing requires an unprefixed authority payload",
            ));
        }
        let signature_start =
            payload
                .checked_add(12)
                .ok_or(TransactionSignError::InvalidPreparation(
                    "invalid Swig signature offset",
                ))?;
        let target = SigningTarget {
            instruction: index,
            signature: signature_start..end,
            message_hash: request.message_hash,
        };
        if matching.replace(target).is_some() {
            return Err(TransactionSignError::AmbiguousSigningTarget);
        }
    }
    matching.ok_or(TransactionSignError::SigningTargetNotFound)
}

fn secp256r1_target(
    transaction: &VersionedTransaction,
    public_key: &[u8; 33],
    request: &ClientSignatureRequest,
) -> Result<SigningTarget, TransactionSignError> {
    let mut matching = None;
    for (index, instruction) in transaction.message.instructions().iter().enumerate() {
        if program_id(transaction, instruction)? != SECP256R1_PROGRAM {
            continue;
        }
        let data = &instruction.data;
        if data.get(SECP256R1_PUBLIC_KEY) != Some(public_key.as_slice())
            || data.get(SECP256R1_MESSAGE) != Some(request.message_hash.as_slice())
        {
            continue;
        }
        if data.get(..2) != Some([1, 0].as_slice())
            || read_u16(data, 2)? != 49
            || read_u16(data, 4)? != u16::MAX
            || read_u16(data, 6)? != 16
            || read_u16(data, 8)? != u16::MAX
            || read_u16(data, 10)? != 113
            || read_u16(data, 12)? != 32
            || read_u16(data, 14)? != u16::MAX
        {
            return Err(TransactionSignError::InvalidPreparation(
                "invalid secp256r1 precompile offsets",
            ));
        }
        let target = SigningTarget {
            instruction: index,
            signature: SECP256R1_SIGNATURE,
            message_hash: request.message_hash,
        };
        if matching.replace(target).is_some() {
            return Err(TransactionSignError::AmbiguousSigningTarget);
        }
    }
    matching.ok_or(TransactionSignError::SigningTargetNotFound)
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, TransactionSignError> {
    let end = offset
        .checked_add(2)
        .ok_or(TransactionSignError::InvalidPreparation(
            "invalid instruction offset",
        ))?;
    let bytes: [u8; 2] = data
        .get(offset..end)
        .ok_or(TransactionSignError::InvalidPreparation(
            "truncated instruction data",
        ))?
        .try_into()
        .map_err(|_| {
            TransactionSignError::InvalidPreparation("invalid instruction offset width")
        })?;
    Ok(u16::from_le_bytes(bytes))
}
