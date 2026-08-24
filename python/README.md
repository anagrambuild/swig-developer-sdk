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
from swig_developer_sdk import (
    Secp256k1ParticipantSetMember,
    WebAuthnP256ParticipantSetMember,
)

created_set = await swig.participant_sets.create(
    swig_config_address=swig_config_address,
    fee_payer=fee_payer,
    threshold=2,
    members=(
        WebAuthnP256ParticipantSetMember(public_key=client_public_key),
        Secp256k1ParticipantSetMember(public_key=server_public_key),
    ),
)

wallet = swig.wallets.use(
    swig_config_address,
    requester_authority=requester_authority,
)
add_role = await wallet.roles.add(
    fee_payer=fee_payer,
    participant_set_address=created_set.participant_set_address,
    permissions=permissions,
)
```

After submitting the setup transactions, prepare an existing legacy action
with a ParticipantSet requester and compile detached approvals:

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

Compilation does not sponsor, submit, or broadcast the result.

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

Ramp is split into `swig.ramp.onramp` and `swig.ramp.offramp`. Every ramp call
requires an `environment` of `"sandbox"` or `"production"`; the SDK encodes it
as the MELD enum on the wire. Options, quotes, and session creation also require
`organization_meld_configuration_id`. Quotes additionally require
`external_customer_id`, `swig_config_address`, and a network (from
`QuoteRampArgs.network` or the client default).

| Client method | Route |
| --- | --- |
| `swig.ramp.onramp.get_options` | `GET /wallet/api/ramp/onramp/options` |
| `swig.ramp.onramp.quote` | `POST /wallet/api/ramp/onramp/quote` |
| `swig.ramp.onramp.create_session` | `POST /wallet/api/ramp/onramp/session` |
| `swig.ramp.onramp.get_session` | `GET /wallet/api/ramp/onramp/session/{session_id}` |
| `swig.ramp.offramp.get_options` | `GET /wallet/api/ramp/offramp/options` |
| `swig.ramp.offramp.quote` | `POST /wallet/api/ramp/offramp/quote` |
| `swig.ramp.offramp.create_session` | `POST /wallet/api/ramp/offramp/session` |
| `swig.ramp.offramp.prepare_authorization` | `POST /wallet/api/ramp/offramp/session/{session_id}/prepare` |
| `swig.ramp.offramp.submit_authorization` | `POST /wallet/api/ramp/offramp/session/{session_id}/submit` |
| `swig.ramp.offramp.get_session` | `GET /wallet/api/ramp/offramp/session/{session_id}` |

### Onramp

```python
import os

from swig_developer_sdk import QuoteRampArgs

environment = "sandbox"
configuration_id = os.environ["SWIG_MELD_CONFIG_ID"]

options = await swig.ramp.onramp.get_options(
    organization_meld_configuration_id=configuration_id,
    environment=environment,
    country_code="US",
)

result = await swig.ramp.onramp.quote(
    QuoteRampArgs(
        organization_meld_configuration_id=configuration_id,
        environment=environment,
        external_customer_id=external_customer_id,
        swig_config_address=swig_config_address,
        network="devnet",
        source_amount="100.00",
        source_currency_code=options.fiat_currency_codes[0],
        destination_currency_code=options.crypto_currency_codes[0],
        country_code="US",
        payment_method_type=options.payment_method_types[0],
    )
)

session = await swig.ramp.onramp.create_session(
    organization_meld_configuration_id=configuration_id,
    quote_id=result.quotes[0].quote_id,
    environment=environment,
)

# Send session.launch_url to the user. Treat it as a user-specific secret.
state = await swig.ramp.onramp.get_session(
    session_id=session.session_id,
    environment=environment,
)
```

`state.status` is one of `unspecified`, `created`, `pending`, `settling`,
`settled`, `failed`, `declined`, `cancelled`, or `refunded`.

### Offramp

Offramp adds an on-chain authorization step: the user's wallet must sign the
transfer to the provider before the session can settle.

```python
from swig_developer_sdk import QuoteRampArgs
from swig_developer_sdk.signers import sign_prepared_swig_transaction

options = await swig.ramp.offramp.get_options(
    organization_meld_configuration_id=configuration_id,
    environment=environment,
    country_code="US",
)

result = await swig.ramp.offramp.quote(
    QuoteRampArgs(
        organization_meld_configuration_id=configuration_id,
        environment=environment,
        external_customer_id=external_customer_id,
        swig_config_address=swig_config_address,
        network="mainnet",
        source_amount="25.00",
        source_currency_code=options.crypto_currencies[0].currency_code,
        destination_currency_code=options.fiat_currency_codes[0],
        country_code="US",
        payment_method_type=options.payment_method_types[0],
    )
)

session = await swig.ramp.offramp.create_session(
    organization_meld_configuration_id=configuration_id,
    quote_id=result.quotes[0].quote_id,
    environment=environment,
)

authorization = await swig.ramp.offramp.prepare_authorization(
    session_id=session.session_id,
    environment=environment,
    fee_payer=fee_payer,
    requester_authority={"secp256r1": {"publicKey": passkey_public_key}},
)

# Show authorization.display to the user, then sign.
signed = await sign_prepared_swig_transaction(
    authorization.prepared_transaction,
    secp256r1=application_passkey_signer,
)

submitted = await swig.ramp.offramp.submit_authorization(
    session_id=session.session_id,
    environment=environment,
    authorization_id=authorization.authorization_id,
    signed_transaction=signed.transaction,
)
# submitted.solana_signature
```

`authorization.display` carries the human-readable transfer summary
(`source_amount`, `destination_amount`, `service_provider`, wallet addresses,
and optional `payment_method_type` / `provider_destination_amount`). Offramp
sessions add `transfer-required` and `transfer-submitted` to the status set.

## Sign locally

The generic signer helper works with an application-owned Ed25519 signer. The
Swig signer helper patches secp256r1 or secp256k1 signatures into both legacy
and versioned Solana transactions.

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

The handler covers the same routes as the TypeScript adapters: wallet create,
grouped prepare, SOL and SPL transfers, Jupiter swap, wallet USD balance, token
balances, token transactions, roles, paymaster balance, and the onramp and
offramp routes.

`SwigProxyConfig` accepts `api_key`, `transaction_api_url`, `network`,
`fee_payer` (a value or a callable resolved per request),
`resolve_requester_authority`, and an optional `httpx` transport. It falls back
to `SWIG_DEVELOPER_API_KEY` / `SWIG_API_KEY`, `SWIG_TRANSACTION_API_URL`, and
`SWIG_FEE_PAYER` when those fields are unset. A fee payer is required for every
preparation route; the handler returns a `400` body when it cannot resolve one.

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
user pays no network fee. Every transaction is submitted to Surfpool and
verified on-chain; the paymaster flow also retries with the same idempotency
key and verifies that no balance change repeats.

It requires a locally running backend stack, and defaults to
`http://localhost:8080` for the Developer API and `http://localhost:8899` for
Surfpool. Override with `SWIG_TRANSACTION_API_URL`, `SOLANA_RPC_URL`, or
`SWIG_DATABASE_URL`.

Jupiter is not covered by this script, since it needs a mainnet-backed
Surfpool.

## Parity

See the repository [parity matrix](../PARITY.md) for the complete mapped
surface against `@swig-wallet/developer-sdk`.
