"""Compile-only consumer contract, checked by `mypy src scripts`."""

from swig_developer_sdk import PreparedYieldRoleCreation, SwigClient


async def reuse_created_yield_authority(
    client: SwigClient, role: PreparedYieldRoleCreation
) -> None:
    await client.yields.prepare_role(
        swig_config_address="config",
        fee_payer="payer",
        yield_authority=role.yield_authority,
        validator_vote_address="validator",
        max_deposit=1,
    )
    await client.yields.deposit(
        swig_config_address="config",
        fee_payer="payer",
        requester_authority=role.yield_authority,
        validator_vote_address="validator",
        amount=1,
        position_seed="deposit",
    )
    await client.yields.exit(
        swig_config_address="config",
        fee_payer="payer",
        requester_authority=role.yield_authority,
        position_address="position",
        amount="all",
    )
    await client.yields.withdraw(
        swig_config_address="config",
        fee_payer="payer",
        requester_authority=role.yield_authority,
        position_address="position",
    )
