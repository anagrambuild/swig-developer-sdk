from swig_developer_sdk import (
    Ed25519ParticipantApproval as RootEd25519ParticipantApproval,
)
from swig_developer_sdk import (
    ParticipantApprovalRequest as RootParticipantApprovalRequest,
)
from swig_developer_sdk import ParticipantSetApproval as RootParticipantSetApproval
from swig_developer_sdk import (
    Secp256k1ParticipantApproval as RootSecp256k1ParticipantApproval,
)
from swig_developer_sdk import (
    WebAuthnP256ParticipantApproval as RootWebAuthnP256ParticipantApproval,
)
from swig_developer_sdk import signers
from swig_developer_sdk.signers import (
    Ed25519ParticipantApproval,
    ParticipantApprovalRequest,
    ParticipantSetApproval,
    PreparedTransaction,
    Secp256k1ParticipantApproval,
    SignedPreparedTransaction,
    TransactionEncoding,
    WebAuthnP256ParticipantApproval,
)


def test_signers_module_exposes_application_owned_signing_surface() -> None:
    assert callable(signers.sign_prepared_transaction)
    assert callable(signers.sign_prepared_transaction_with_signer)
    assert callable(signers.sign_prepared_swig_transaction)
    assert callable(signers.sign_prepared_swig_transactions)
    assert callable(signers.create_secp256r1_passkey_signing_fn)
    assert callable(signers.create_secp256k1_evm_signing_fn)
    assert callable(signers.create_participant_passkey_signer)
    assert callable(signers.create_participant_personal_sign_signer)
    assert callable(signers.sign_participant_set_approval)
    assert signers.Ed25519ParticipantApproval is Ed25519ParticipantApproval
    assert Ed25519ParticipantApproval is RootEd25519ParticipantApproval
    assert signers.ParticipantApprovalRequest is ParticipantApprovalRequest
    assert ParticipantApprovalRequest is RootParticipantApprovalRequest
    assert signers.ParticipantSetApproval is ParticipantSetApproval
    assert ParticipantSetApproval is RootParticipantSetApproval
    assert signers.PreparedTransaction is PreparedTransaction
    assert signers.Secp256k1ParticipantApproval is Secp256k1ParticipantApproval
    assert Secp256k1ParticipantApproval is RootSecp256k1ParticipantApproval
    assert signers.SignedPreparedTransaction is SignedPreparedTransaction
    assert signers.TransactionEncoding is TransactionEncoding
    assert signers.WebAuthnP256ParticipantApproval is WebAuthnP256ParticipantApproval
    assert WebAuthnP256ParticipantApproval is RootWebAuthnP256ParticipantApproval


def test_signers_module_does_not_expose_api_clients() -> None:
    assert not hasattr(signers, "SwigClient")
    assert not hasattr(signers, "HttpClient")
