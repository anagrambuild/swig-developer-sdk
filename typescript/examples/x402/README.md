# Swig x402 example

This Devnet demo can run the complete TypeScript payment flow or keep its local
servers running for the [Rust x402 client](../../../rust/examples/README.md#run-an-x402-payment):

1. Setup validates or provisions the required on-chain state.
2. A facilitator verifies and settles Swig payment transactions.
3. A resource server protects a weather endpoint with x402.
4. By default, the TypeScript developer uses `wallet.x402.prepareFromResponse()`
   to prepare the payment, signs it, and retries the protected request. With
   `--server-only`, the servers wait for a separate client instead.

The initial demo is Devnet-only. Setup and the local facilitator use the same
RPC endpoint as the payment client. Settlement belongs solely to the facilitator;
the client must not also submit its payment through RPC or the paymaster.

## Configuration

From the repository root:

```sh
cd typescript
bun install
cp examples/x402/.env.example examples/x402/.env.local
```

For the complete TypeScript flow, set these values in `.env.local`:

- `SWIG_API_KEY`
- `SWIG_RPC_URL` (or the existing `SOLANA_RPC_URL` alias)

`SWIG_API_URL` optionally overrides the hosted API; `SWIG_BACKEND_URL` remains
supported as an alias. `SWIG_ED25519_PRIVATE_KEY` accepts the same 64-byte
hex-encoded Solana keypair as the Rust examples. It is required in server-only
mode so setup funds the wallet used by the Rust signer. When both an alias and
its shared setting are supplied, their values must agree; developer key
settings must identify the same keypair.

The facilitator and resource-provider keypairs are optional. Setup uses
`X402_FACILITATOR_KEYPAIR` and `X402_RESOURCE_PROVIDER_KEYPAIR` when provided;
otherwise it generates and remembers them. The existing
`X402_DEVELOPER_KEYPAIR` setting remains supported as a JSON array of 64
secret-key bytes. In the complete TypeScript flow, setup can also generate and
remember an omitted developer keypair. The other `X402_*_KEYPAIR` settings use
the same JSON format.

`X402_PAYMENT_AMOUNT` is the canonical positive integer placed in the x402
requirement. It is denominated in atomic token units. When setup creates a
mint, it derives the mint decimals from the amount; for example, an amount of
`1000` creates a three-decimal mint.

## Setup state

`run.ts` always invokes setup before starting the services. Setup:

- Ensures the facilitator has enough SOL.
- Validates or creates the Swig wallet.
- Validates a configured `X402_MINT`, or creates a mint when it is omitted.
- Derives and creates the Swig and resource-provider token accounts.
- Ensures the Swig token account can cover one payment.

Generated identity keypairs, mint, and Swig values are remembered in the
gitignored `.local/state.json` file. Token-account addresses are always derived
and are not persisted.

Environment values take priority over remembered state. A Swig supplied through
`SWIG_CONFIG_ADDRESS` must already exist. Setup derives its wallet address when
`SWIG_WALLET_ADDRESS` is absent and checks a supplied wallet address against it.
A wallet address alone is insufficient. If a supplied mint needs a token top-up,
the facilitator must be its mint authority. Setup funds the exact wallet and
mint it reports to the payment client.

Setup can also be run independently when only provisioning or validation is
needed:

```sh
bun run example:x402:setup
```

## Run the complete flow

From `typescript`, build the SDK package, then run the example:

```sh
bun run build
bun run example:x402
```

The runner starts the facilitator and resource server on ports `4022` and
`4021` by default and binds both to the local loopback interface. Set
`X402_FACILITATOR_PORT` or
`X402_RESOURCE_SERVER_PORT` to override them.

When no accepted index is supplied, Swig selects the first compatible exact
Solana payment option and preserves its original position in `accepts`. The
example signs the prepared transaction, retries the protected request, and
prints both the resource and `PAYMENT-RESPONSE` header before shutting down its
local services.

## Run the servers for Rust

Configure `SWIG_API_KEY`, `SWIG_RPC_URL`, and `SWIG_ED25519_PRIVATE_KEY` in your
untracked `rust/examples/.env`. The same optional `SWIG_API_URL` override applies
to both processes. Keep private keys in that file; this workflow does not copy
them. The TypeScript `.env.local` is optional for server ports, mint, amount,
and other local settings. Copy `.env.example` if needed and keep aliases
consistent with the shared values.

In the first terminal, from the repository root:

```sh
cd typescript
bun install
bun run build
bun --env-file=../rust/examples/.env --env-file=examples/x402/.env.local run examples/x402/run.ts --server-only
```

Omit the second `--env-file` argument when you have no TypeScript `.env.local`.
Setup creates or validates the wallet and mint, funds the wallet for one
payment, and leaves the facilitator and resource server running. It prints
public `export` lines for `SWIG_CONFIG_ADDRESS`, `SWIG_WALLET_ADDRESS`,
`SWIG_X402_RESOURCE_URL`, `SWIG_X402_ASSET`, `SWIG_X402_PAY_TO`, and
`SWIG_X402_MAX_AMOUNT`.

Paste those public exports into a second terminal at the repository root, then:

```sh
cd rust/examples
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example x402_payment
```

The Rust flow requires that existing config and reuses its funded wallet; it
does not create another wallet. It checks the approved resource, asset,
recipient, and payment cap before signing. The facilitator settles the payment.
Leave the first terminal open until the request completes, then press Ctrl-C
there to stop the servers. Running without `--server-only` still executes the
complete TypeScript payment and shuts down automatically.
