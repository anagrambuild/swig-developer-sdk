from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest


@pytest.mark.parametrize(
    ("source", "valid"),
    [
        (
            """from swig_developer_sdk import (
    SwigClient, WalletAuthority, SponsorSignedTransactionArgs,
)
from swig_developer_sdk.signers import sign_prepared_transaction

authority: WalletAuthority = {"ed25519": {"public_key": "key"}}
legacy: WalletAuthority = {"secp256r1": {"publicKey": "key"}}
participant: WalletAuthority = {"participant_set": {"address": "set", "role_id": 1}}
proof: WalletAuthority = {"program_exec_proof": {"role_id": 1, "zk_proof": "proof"}}

async def use_sdk() -> str:
    async with SwigClient(api_key="key", network="devnet") as client:
        wallet = client.wallets.use("wallet", requester_authority=authority)
        prepared = await wallet.transfer.sol(
            fee_payer="payer", destination="dest", amount=1,
        )
        submitted = await client.transactions.sponsor(transaction=prepared.transaction)
        await client.transactions.sponsor(
            SponsorSignedTransactionArgs(transaction="YQ=="),
        )
        await client.transactions.sponsor_bundle(
            transactions=["YQ=="], network="mainnet",
        )
        return submitted.signature
""",
            True,
        ),
        (
            """from swig_developer_sdk import Ed25519Authority
invalid: Ed25519Authority = {"ed25519": {"publik_key": "key"}}
""",
            False,
        ),
        (
            """from swig_developer_sdk import ProgramExecAuthority
invalid: ProgramExecAuthority = {
    "program_exec_proof": {"role_id": "1", "zk_proof": "proof"},
}
""",
            False,
        ),
        (
            """from swig_developer_sdk import SwigClient
async def invalid(client: SwigClient) -> None:
    await client.transactions.sponsor()
""",
            False,
        ),
        (
            """from swig_developer_sdk import SwigClient, SponsorSignedTransactionArgs
async def invalid(client: SwigClient) -> None:
    await client.transactions.sponsor(
        SponsorSignedTransactionArgs(transaction="YQ=="), transaction="Yg==",
    )
""",
            False,
        ),
    ],
)
def test_consumer_type_contract(tmp_path: Path, source: str, valid: bool) -> None:
    """Check public imports without the repository's mypy_path override."""
    consumer = tmp_path / "consumer.py"
    consumer.write_text(source)
    config = tmp_path / "mypy.ini"
    config.write_text("[mypy]\nstrict = True\ndisallow_any_expr = True\n")
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--config-file",
            str(config),
            "--no-incremental",
            str(consumer),
        ],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == (0 if valid else 1), result.stdout + result.stderr
    if not valid:
        assert "consumer.py:" in result.stdout
        assert "import-untyped" not in result.stdout
