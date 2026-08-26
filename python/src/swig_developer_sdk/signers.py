"""Application-owned signing boundary for the Swig Developer SDK.

This module accepts prepared payloads and caller-provided signing callbacks. It
does not construct an API client or send requests to Swig's hosted API.
"""

from .evm import Eip1193Provider, create_secp256k1_evm_signing_fn
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
    PreparedTransaction,
    SignedPreparedTransaction,
    TransactionEncoding,
)

__all__ = [
    "Eip1193Provider",
    "PasskeySigningFn",
    "PasskeySigningResult",
    "PreparedTransaction",
    "PreparedTransactionSigner",
    "PreparedTransactionSigningFn",
    "Secp256k1SigningFn",
    "Secp256k1SigningFns",
    "Secp256k1SigningResult",
    "Secp256r1SigningFns",
    "SignedPreparedTransaction",
    "TransactionEncoding",
    "WebAuthnAssertion",
    "WebAuthnAssertionFn",
    "create_secp256k1_evm_signing_fn",
    "create_secp256r1_passkey_signing_fn",
    "secp256r1_der_to_raw_signature",
    "sign_prepared_swig_transaction",
    "sign_prepared_swig_transactions",
    "sign_prepared_transaction",
    "sign_prepared_transaction_with_signer",
]
