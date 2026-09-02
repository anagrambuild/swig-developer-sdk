# swig-developer-sdk

Python SDK for preparing Swig wallet operations on a server, with a separate
`swig_developer_sdk.signers` module for application-owned signing. The signer
module inserts signatures with `solders` and makes no hosted API requests. No
signing material is sent to the API.

- Version: `0.8.0`
- Source: <https://github.com/anagrambuild/swig-developer-sdk>
- Default API base URL: `https://api.onswig.com`

```bash
pip install swig-developer-sdk
```

This is the Python parity package for `@swig-wallet/developer-sdk`. Every
client method is `async`; the transport is `httpx`.

## How it works

1. Your server creates a `SwigClient` with an API key.
2. Your server prepares a wallet operation and receives one or more unsigned
   transactions.
3. Your client signs any transaction whose `signature_requests` is non-empty.
4. Your app submits the transactions in order, directly or through Swig's
   paymaster sponsorship.

The API key stays on the server. Browser code calls a route in your own app —
see [Proxy handler](#proxy-handler).

## Create a client

Create an API key from the [Swig dashboard](https://dashboard.onswig.com).

```python
from swig_developer_sdk import SwigClient

swig = SwigClient(api_key="sk_...", network="devnet")
```

Requests authenticate with `Authorization: Bearer <api-key>`. Override the base
URL for non-production deployments:

```python
from swig_developer_sdk import RetryOptions, SwigClient

swig = SwigClient(
    api_key="sk_...",
    base_url="http://localhost:8080",
    network="devnet",
    retry_options=RetryOptions(max_retries=3, retry_delay=1.0),
)
```

### Retry behavior

- `GET` requests use the configured retry policy.
- `POST` requests do **not** retry by default, because replaying a preparation
  or submission request can duplicate work.
- Sponsorship `POST` requests retry only when `idempotency_key` is set. A
  matching retry returns the original paymaster response.
- `4xx` responses raise `SwigDeveloperSdkError` immediately.

## Create a wallet

```python
created = await swig.wallets.create(
    fee_payer=fee_payer,
    initial_user={"ed25519": {"publicKey": user_public_key}},
)
```

Use `secp256r1` for a passkey initial user or `secp256k1` for an EVM key. Pass
`policy_id` to create from a portal policy instead of an inline authority. A
network is required, either on the call or as the client default.

The result splits the prepared transactions:

| Field | What to do |
| --- | --- |
| `wallet` | `swig_config_address`, `wallet_address`, and resolved `network` |
| `transactions` | submit in this exact order |
| `client_authority_transactions` | get a client authority signature first |
| `fee_payer_only_transactions` | send or sponsor without a client authority signature |
| `creation_transaction` | the create transaction itself |

A prepared transaction needs a client authority signature when
`signature_requests` is non-empty.

## Attach to an existing wallet

```python
wallet = swig.wallets.use(
    "SWIG_CONFIG_ADDRESS",
    requester_authority={"ed25519": {"publicKey": user_public_key}},
)
```

`swig.wallets.use` also accepts a `WalletReference`, and
`swig.wallets.from_idp_session(session)` builds the same handle from an IdP
session.

### Configure and use a ParticipantSet

```python
from swig_developer_sdk import ProgramAction, SolLimitAction

created_set = await swig.participant_sets.create(
    swig_config_address=swig_config_address,
    fee_payer=fee_payer,
    threshold=2,
    members=(
        {"ed25519": {"publicKey": recovery_public_key}},
        {"secp256r1": {"publicKey": client_public_key}},
        {"secp256k1": {"publicKey": server_public_key}},
    ),
)

wallet = swig.wallets.use(
    swig_config_address,
    requester_authority=requester_authority,
)
add_role = await wallet.roles.add(
    fee_payer=fee_payer,
    authority={
        "participantSet": {"address": created_set.participant_set_address}
    },
    actions=(
        SolLimitAction(amount=1_000_000),
        ProgramAction(program_id=program_id),
    ),
)
```

After submitting the setup transactions, prepare an action with a ParticipantSet
requester and compile detached approvals:

```python
participant_wallet = swig.wallets.use(
    swig_config_address,
    requester_authority={
        "participantSet": {"address": created_set.participant_set_address}
    },
)
prepared = await participant_wallet.transfer.sol(
    fee_payer=fee_payer,
    destination=destination,
    amount=1_000_000,
)
compiled = await swig.transactions.compile_participant_set_approvals(
    prepared_transaction=prepared,
    approvals=approvals,
)
```

The plan uses one shared nonce, and each member approval signs its returned
challenge. Compilation returns `compiled.transaction` plus
`compiled.authorization_expiration_slot`; it does not sponsor, submit, or
broadcast the result.

### Prepare an x402 payment

Pass the resource server's `402 Payment Required` response directly to the
wallet. When `accepted_index` is omitted, Swig selects the first eligible exact
Solana requirement and returns its original array index.

```python
import httpx

from swig_developer_sdk import create_x402_payment
from swig_developer_sdk.signers import sign_prepared_transaction

async with httpx.AsyncClient() as http:
    challenge = await http.get(resource_url)
    prepared = await wallet.x402.prepare_from_response(challenge)
    signed = await sign_prepared_transaction(
        prepared.prepared_transaction,
        sign_transaction=sign_transaction,
    )
    payment = create_x402_payment(prepared, signed)
    response = await http.get(
        resource_url,
        headers=payment.payment_signature_headers,
    )
```

To choose a specific offer from the original `accepts` array:

```python
prepared = await wallet.x402.prepare_from_response(
    challenge,
    accepted_index=accepted_index,
)
```

Use `sign_prepared_transaction` for Ed25519 authorities and
`sign_prepared_swig_transaction` for Secp256r1/passkey authorities.

### Prepare transfers and swaps

```python
prepared = await wallet.transfer.sol(
    fee_payer=fee_payer,
    destination=destination,
    amount=1_000_000,
)

prepared_token = await wallet.transfer.token(
    fee_payer=fee_payer,
    mint=mint,
    destination_owner=destination_owner,
    amount=10_000,
)

prepared_swap = await wallet.swap.jupiter(
    fee_payer=fee_payer,
    input_mint=input_mint,
    output_mint=output_mint,
    amount=10_000,
    slippage_bps=100,
    destination_account=destination_account,
    wrap_and_unwrap_sol=True,
)
```

`wallet.transfer(...)` and `wallet.swap(...)` are callable like their
TypeScript counterparts; `spl_token` is an alias for `token`.

### Prepare grouped operations

```python
from swig_developer_sdk import TransferSolOperation, TransferTokenOperation

prepared = await wallet.prepare(
    fee_payer=fee_payer,
    operations=(
        TransferSolOperation(destination=destination, amount=1_000_000),
        TransferTokenOperation(
            mint=mint,
            destination_owner=destination_owner,
            amount=10_000,
        ),
    ),
)
```

### Build a custom transaction

```python
from swig_developer_sdk import SolanaAccountMeta, SolanaInstructionInput

prepared = await wallet.build_transaction(
    fee_payer=fee_payer,
    instructions=(
        SolanaInstructionInput(
            program_id=program_id,
            accounts=(SolanaAccountMeta(pubkey=destination, is_writable=True),),
            data=instruction_data,
        ),
    ),
    address_lookup_table_accounts=(lookup_table_address,),
)
```

The returned transaction is still signed and submitted by your application.

## Read wallet state

```python
usd = await wallet.get_usd_balance()
# usd.swig_config_address, usd.wallet_address, usd.usd_value

tokens = await wallet.list_token_balances()
# tokens.balances[].mint_address, asset_kind, ui_amount, usd_value

activity = await wallet.list_token_transactions(limit=25)
# activity.transactions[].transaction_signature, direction, asset_kind, ui_amount
```

### Roles

`list_roles()` returns the on-chain roles attached to the Swig, which is how
you inspect who currently holds authority and what each authority may do.

```python
result = await wallet.list_roles()

for role in result.roles:
    print(role.role_id, role.authority_type, role.authority_value)
    for action in role.actions:
        print(action.action_index, action.action_code, action.action_data)
```

`authority_value` is the authority's public key material and `authority_type`
is the protocol authority type discriminant. `action_data` is the raw
per-action payload, so read it against the protocol's action definitions rather
than assuming a fixed shape.

Policy metadata is a separate read:

```python
policy = await swig.wallets.get_policy(policy_id)
```

## Fiat ramps

`swig.ramp` covers both directions. Direction is carried by the buy or sell
order you pass, so there is no separate on-ramp or off-ramp client. Every call
requires a `configuration_id` and an `environment` of `"sandbox"` or
`"production"`; `get_order`, `prepare_transfer`, and `submit_transfer` identify
the order in the route and take no environment.

Amounts are integers in the smallest unit the thing has — `minor_units` for
fiat (cents for USD, whole yen for JPY) and `base_units` for crypto. They cross
the wire as decimal strings, so a value above 2^53 survives; pass an `int` or
`str` and read a `str` back. Crypto is a `NativeSolAsset()` or
`SplTokenAsset(mint=...)`.

| Client method                | Route                                                      |
| ---------------------------- | ---------------------------------------------------------- |
| `swig.ramp.get_options`      | `GET /wallet/api/ramp/options`                             |
| `swig.ramp.get_quotes`       | `POST /wallet/api/ramp/quotes`                             |
| `swig.ramp.create_order`     | `POST /wallet/api/ramp/orders`                             |
| `swig.ramp.get_order`        | `GET /wallet/api/ramp/orders/{order_id}`                   |
| `swig.ramp.prepare_transfer` | `POST /wallet/api/ramp/orders/{order_id}/transfer/prepare` |
| `swig.ramp.submit_transfer`  | `POST /wallet/api/ramp/orders/{order_id}/transfer/submit`  |

### Options and quotes

Read the options first, so currency, payment-method, and asset values come from
the API rather than a hardcoded list. `RampAssetOption.decimals` and
`RampFiatCurrencyOption.exponent` are the scales you need to build a valid
amount.

```python
options = await swig.ramp.get_options(
    configuration_id=configuration_id,
    environment="sandbox",
    direction="buy",
    country_code="US",
)

quotes = await swig.ramp.get_quotes(
    configuration_id=configuration_id,
    environment="sandbox",
    location=RampLocation(country_code="US"),
    order=RampBuyOrderRequest(
        spend=FiatAmountInput(currency_code="USD", minor_units=10_000),
        receive=SplTokenAsset(mint=usdc_mint),
    ),
)
```

Quotes carry no identifier and must never be cached. Pick one and pass its
`route` to `create_order`; the route is re-priced when the order is created.

### Orders

`request_id` is your idempotency key and is unique within the configuration.
Repeating it returns the stored order; repeating it with different inputs is
refused, so mint it once and reuse it across retries.

```python
order = await swig.ramp.create_order(
    request_id=str(uuid.uuid4()),
    configuration_id=configuration_id,
    environment="sandbox",
    context=RampOrderContext(
        customer_id=customer_id,
        swig_config_address=swig_config_address,
        location=RampLocation(country_code="US"),
    ),
    route=quotes[0].route,
    order=RampBuyOrderRequest(
        spend=FiatAmountInput(currency_code="USD", minor_units=10_000),
        receive=SplTokenAsset(mint=usdc_mint),
    ),
)
```

A buy sends the customer to `order.launch_url`. Poll `swig.ramp.get_order`
until the status is final; a read of a non-final order also reconciles it
against the provider. `refunded` can follow `settled`.

A `launch_url` is a user-specific session URL. Hand it to the customer who owns
the order and keep it out of logs and analytics.

### Selling

A sell waits for `awaiting-transfer` and a `deposit`, then moves the crypto out
of the Swig. Your application owns the signing step.

```python
prepared = await swig.ramp.prepare_transfer(
    order_id=order.id,
    requester_authority={"ed25519": {"publicKey": requester}},
    fee_payer=fee_payer,
)

signed = sign_prepared_transaction(prepared.prepared_transaction, signer)

transfer = await swig.ramp.submit_transfer(
    order_id=order.id,
    transfer_id=prepared.transfer.transfer_id,
    signed_transaction=signed.transaction,
)
```

The prepared transaction is handed over once. If you broadcast it and then lose
it, call `submit_transfer` again without `signed_transaction` to resolve the
attempt that is already live.

## Sign locally

The generic signer helper works with an application-owned Ed25519 signer. The
Swig signer helper patches secp256r1 or secp256k1 signatures into both legacy
and versioned Solana transactions.

ParticipantSet signers operate on one bound member request and never call the
hosted API:

```python
from swig_developer_sdk.signers import (
    create_participant_ed25519_signer,
    create_participant_passkey_signer,
    create_participant_personal_sign_signer,
    sign_participant_set_approval,
)

recovery_signer = create_participant_ed25519_signer(
    public_key=recovery_public_key,
    sign_message=sign_ed25519,
)
recovery_approval = await sign_participant_set_approval(
    prepared.participant_set_approval_plan.members[0],
    recovery_signer,
)

passkey_signer = create_participant_passkey_signer(
    public_key=client_public_key,
    get_assertion=get_webauthn_assertion,
)
client_approval = await sign_participant_set_approval(
    prepared.participant_set_approval_plan.members[1],
    passkey_signer,
)

server_signer = create_participant_personal_sign_signer(
    public_key=server_public_key,
    # personal_sign applies EIP-191 to this exact 64-character lowercase
    # ASCII hex challenge.
    sign_message=personal_sign,
)
server_approval = await sign_participant_set_approval(
    prepared.participant_set_approval_plan.members[2],
    server_signer,
)
```

The Ed25519 callback receives the decoded 32-byte challenge. The shared
ParticipantSet nonce is already committed by every challenge and is not copied
into individual approvals. The passkey adapter returns exact assertion bytes
and converts DER to a raw low-S P-256 signature; compilation also defensively
normalizes externally constructed high-S P-256 approvals. The personal-sign
adapter validates compact k1 signatures, normalizes low-S, and adjusts the
recovery byte. None of these helpers calls the hosted API.

```python
from swig_developer_sdk.signers import (
    sign_prepared_swig_transaction,
    sign_prepared_transaction,
)

signed = await sign_prepared_transaction(
    prepared,
    sign_transaction=application_ed25519_signer,
)

signed = await sign_prepared_swig_transaction(
    prepared,
    secp256r1=application_passkey_signer,
)
```

`sign_prepared_swig_transactions(...)` signs an ordered sequence, which is what
`created.client_authority_transactions` needs.

WebAuthn and EIP-1193 adapters are callback-based, so applications can connect
a browser, hardware, wallet, or remote signer without changing the preparation
API:

```python
from swig_developer_sdk.signers import (
    create_secp256k1_evm_signing_fn,
    create_secp256r1_passkey_signing_fn,
)

passkey_signer = create_secp256r1_passkey_signing_fn(get_webauthn_assertion)
evm_signer = create_secp256k1_evm_signing_fn(
    provider=eip1193_provider,
    address=evm_address,
)
```

## Submit and sponsor

```python
from swig_developer_sdk import SponsorSignedTransactionArgs

submitted = await swig.transactions.sponsor(
    SponsorSignedTransactionArgs(
        transaction=signed.transaction,
        network="mainnet",
        idempotency_key=idempotency_key,
    )
)
# submitted.request_id, submitted.signature, submitted.spent_by_paymaster
```

Pass `idempotency_key` whenever your application may retry; that is the only
case in which the SDK retries a sponsorship POST.

For single-transaction sponsorship, `network` resolves from the call and then
the client default. If neither is set, the paymaster defaults to mainnet.

A returned signature means the Solana RPC accepted the transaction. It may
still be pending and is not confirmation or finality; track it through your
RPC provider when your product needs either.

### Bundle sponsorship

```python
from swig_developer_sdk import SponsorSignedTransactionBundleArgs

bundle = await swig.transactions.sponsor_bundle(
    SponsorSignedTransactionBundleArgs(
        transactions=(signed_create.transaction, signed_add_authority.transaction),
        network="mainnet",
        idempotency_key=idempotency_key,
    )
)
# bundle.request_id, bundle.bundle_id, bundle.signatures,
# bundle.estimated_spent_by_paymaster
```

Constraints enforced before the request is sent:

- mainnet only — any other network raises `ValueError`
- one to five transactions per bundle
- `signatures` come back in bundle order
- `estimated_spent_by_paymaster` is an estimate, not a settled charge

A returned `bundle_id` means Jito accepted the bundle. It may still be pending;
acceptance is not confirmation or finality.

## Paymaster balance

```python
balance = await swig.paymaster.get_balance(network="mainnet")
idp_balance = await swig.paymaster.get_idp_balance(network="mainnet")
# balance.configured, balance.address, balance.balance_lamports, balance.balance_sol
```

## Proxy handler

Python ships a framework-neutral proxy handler instead of one framework
adapter. Adapt your framework's request and response objects at the route
boundary:

```python
from swig_developer_sdk import SwigProxyConfig, create_swig_proxy_handler

swig_handler = create_swig_proxy_handler(
    SwigProxyConfig(api_key=swig_api_key, network="devnet")
)

response = await swig_handler.handle(
    method="POST",
    path="/transfer/sol",
    body=request_body,
)
# return response.body with response.status
```

The handler covers wallet creation, grouped preparation, SOL and SPL transfers,
Jupiter swaps, wallet USD balance, token balances, token transactions, roles,
x402 payment preparation, paymaster balance, and the ramp routes.

`SwigProxyConfig` accepts `api_key`, `transaction_api_url`, `network`,
`fee_payer` (a value or a callable resolved per request),
`resolve_requester_authority`, and an optional `httpx` transport. It falls back
to `SWIG_DEVELOPER_API_KEY` / `SWIG_API_KEY`, `SWIG_TRANSACTION_API_URL`, and
`SWIG_FEE_PAYER` when those fields are unset. A fee payer is required for
preparation routes other than x402; x402 obtains its sponsor fee payer from the
selected payment requirement.

## Errors

```python
from swig_developer_sdk import SwigDeveloperSdkError

try:
    await wallet.transfer.sol(
        fee_payer=fee_payer,
        destination=destination,
        amount=1_000_000,
    )
except SwigDeveloperSdkError as error:
    print(error.status_code, error.code, error)
```

Keep API keys, signed transactions, and ramp launch URLs out of logs.

## Local end-to-end script

`scripts/local_transaction_e2e.py` exercises wallet creation, real P-256
signing, direct and grouped SOL transfers, an SPL-token transfer, the Python
proxy, the live paymaster balance endpoint, and a sponsored transfer where the
user pays no network fee. It also creates a two-member ParticipantSet, adds it
as a general role, compiles detached Ed25519 approvals, executes the transfer,
and proves that another approval plan using the consumed shared nonce is
rejected. Every transaction is submitted to Surfpool and verified on-chain;
the paymaster flow also retries with the same idempotency key and verifies that
no balance change repeats.

It requires a locally running backend stack, and defaults to
`http://localhost:8080` for the Developer API and `http://localhost:8899` for
Surfpool. Override with `SWIG_TRANSACTION_API_URL`, `SOLANA_RPC_URL`, or
`SWIG_DATABASE_URL`. Set `SWIG_E2E_SKIP_PAYMASTER=1` when validating only the
transaction-service and Surfpool path; the receipt explicitly records that the
paymaster phase was skipped.

Jupiter is not covered by this script, since it needs a mainnet-backed
Surfpool.

## Parity

See the repository [parity matrix](../PARITY.md) for the complete mapped
surface against `@swig-wallet/developer-sdk`.
