"""Application-owned signing boundary for the Swig Developer SDK.

This module accepts prepared payloads and caller-provided signing callbacks. It
does not construct an API client or send requests to Swig's hosted API.
"""

from .evm import Eip1193Provider, create_secp256k1_evm_signing_fn
from .participant_signers import (
    ParticipantEd25519Signer,
    ParticipantEd25519SignFn,
    ParticipantPasskeySigner,
    ParticipantPersonalSignFn,
    ParticipantPersonalSignSigner,
    ParticipantSigner,
    ParticipantSignerType,
    create_participant_ed25519_signer,
    create_participant_passkey_signer,
    create_participant_personal_sign_signer,
    sign_participant_set_approval,
)
from .passkeys import (
    WebAuthnAssertion,
    WebAuthnAssertionFn,
    create_secp256r1_passkey_signing_fn,
    secp256r1_der_to_raw_signature,
)
from .signing import (
    PasskeySigningFn,
    PasskeySigningResult,
    PreparedTransactionSigner,
    PreparedTransactionSigningFn,
    Secp256k1SigningFn,
    Secp256k1SigningFns,
    Secp256k1SigningResult,
    Secp256r1SigningFns,
    sign_prepared_swig_transaction,
    sign_prepared_swig_transactions,
    sign_prepared_transaction,
    sign_prepared_transaction_with_signer,
)
from .transactions import (
    Ed25519ParticipantApproval,
    ParticipantApprovalRequest,
    ParticipantSetApproval,
    PreparedTransaction,
    Secp256k1ParticipantApproval,
    SignedPreparedTransaction,
    TransactionEncoding,
    WebAuthnP256ParticipantApproval,
)

__all__ = [
    "Ed25519ParticipantApproval",
    "Eip1193Provider",
    "ParticipantApprovalRequest",
    "ParticipantEd25519SignFn",
    "ParticipantEd25519Signer",
    "ParticipantPasskeySigner",
    "ParticipantPersonalSignFn",
    "ParticipantPersonalSignSigner",
    "ParticipantSetApproval",
    "ParticipantSigner",
    "ParticipantSignerType",
    "PasskeySigningFn",
    "PasskeySigningResult",
    "PreparedTransaction",
    "PreparedTransactionSigner",
    "PreparedTransactionSigningFn",
    "Secp256k1ParticipantApproval",
    "Secp256k1SigningFn",
    "Secp256k1SigningFns",
    "Secp256k1SigningResult",
    "Secp256r1SigningFns",
    "SignedPreparedTransaction",
    "TransactionEncoding",
    "WebAuthnAssertion",
    "WebAuthnAssertionFn",
    "WebAuthnP256ParticipantApproval",
    "create_participant_ed25519_signer",
    "create_participant_passkey_signer",
    "create_participant_personal_sign_signer",
    "create_secp256k1_evm_signing_fn",
    "create_secp256r1_passkey_signing_fn",
    "secp256r1_der_to_raw_signature",
    "sign_participant_set_approval",
    "sign_prepared_swig_transaction",
    "sign_prepared_swig_transactions",
    "sign_prepared_transaction",
    "sign_prepared_transaction_with_signer",
]
