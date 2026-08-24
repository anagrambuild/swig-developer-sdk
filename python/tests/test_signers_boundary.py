from swig_developer_sdk import signers


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


def test_signers_module_does_not_expose_api_clients() -> None:
    assert not hasattr(signers, "SwigClient")
    assert not hasattr(signers, "HttpClient")
