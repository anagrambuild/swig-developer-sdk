# Rust SDK examples

3 standalone flows: 3 runnable Cargo examples.
Keygen is a separate utility binary. The package is unpublished;
the SDK remains the default workspace member.

Each flow lives in `src/` with its own imports, configuration, and SDK calls.
Runnable flows have thin CLI runners in `examples/`.
Signer loading and transaction construction stay in each signing flow.

## Choose a flow

| Source | Entry point | Flow |
| --- | --- | --- |
| [wallet_reads.rs](src/wallet_reads.rs) | `--example wallet_reads` | Read roles, balances, history, and optionally a policy |
| [ed25519_transfer.rs](src/ed25519_transfer.rs) | `--example ed25519_transfer` | Create or reuse a wallet, fund if needed, transfer SOL, sponsor, confirm |
| [secp256r1_transfer.rs](src/secp256r1_transfer.rs) | `--example secp256r1_transfer` | Create or reuse a secp256r1 wallet, sign a SOL transfer, sponsor, confirm |

## Setup

Use Rust 1.87 or later. From the repository root:

```sh
cd rust/examples
cp .env.example .env
```

Keep real credentials in the untracked `.env`. The commands below run from
`rust/examples` unless they explicitly name another working directory.

Each flow loads `.env` and reads only the inputs it needs. Shell variables
take precedence. This loading belongs to the examples; the SDK receives
explicit configuration and signers from its caller.

The hosted API is the default. Set `SWIG_API_KEY` for hosted calls.
Leave optional values absent when using defaults; an empty value is
not treated as an absent setting.

Transaction flows also require `SWIG_RPC_URL`. API preparation and RPC
must use the same chain. For internal local testing, explicitly configure
matching `SWIG_API_URL` and `SWIG_RPC_URL` endpoints; choosing `devnet`
alone selects neither a local API nor a local RPC.

## Keys

For signing flows, generate throwaway keys with the
[keygen utility](src/bin/keygen.rs):

```sh
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --bin keygen
```

Keygen prints three private-key assignments: `SWIG_ED25519_PRIVATE_KEY`,
`SWIG_SECP256K1_PRIVATE_KEY`, and `SWIG_SECP256R1_PRIVATE_KEY`.
It writes no files. Keep its terminal output private and copy the keys you
need into your own untracked `.env`. Preserve saved keys when reusing their
wallets. Never commit private keys or the real `.env`, and never use these
throwaway keys with production funds.

Wallet reads need no signing key; skip key generation for that flow.

## Configuration

The [.env.example](.env.example) lists the available settings.
Set only the selected flow's inputs.

| Variable | Meaning |
| --- | --- |
| `SWIG_API_KEY` | Required for hosted SDK calls |
| `SWIG_NETWORK` | Defaults to `devnet` |
| `SWIG_API_URL` | Optional override; leave absent for the SDK's hosted API default |
| `SWIG_CONFIG_ADDRESS`, `SWIG_WALLET_ADDRESS` | Existing config and optional matching wallet; reads require a config |
| `SWIG_ED25519_PRIVATE_KEY` | 64-byte Solana keypair encoded as 128 hex characters |
| `SWIG_SECP256K1_PRIVATE_KEY`, `SWIG_SECP256R1_PRIVATE_KEY` | Each is a 32-byte secret encoded as 64 hex characters |
| `SWIG_RPC_URL` | Required for transaction flows: a nonempty HTTP(S) RPC URL on the selected chain |
| `SWIG_DESTINATION`, `SWIG_AMOUNT_LAMPORTS` | Separate transfer recipient and SOL amount; default 1,000,000 lamports |

## Wallet reuse and funding

For flows that create or reuse a wallet, an absent `SWIG_CONFIG_ADDRESS`
creates one from the loaded authority. A supplied config must already
exist and be usable by that authority; errors never fall back to creation.
Save the printed config and wallet addresses to reuse the wallet.
`SWIG_WALLET_ADDRESS` is optional, must match the config's wallet, and
cannot be supplied alone. Reads always require an existing config.

Choose a recipient you control through `SWIG_DESTINATION`. On Devnet,
transfer flows generate a temporary recipient when it is absent; its
private key is not retained, so transferred funds cannot be recovered
through that generated key. Recipients must differ from the requester,
wallet, and config addresses.

The paymaster supplies the fee payer and transaction fees. Fund the
wallet with the SOL being transferred.

## Read an existing wallet

Set `SWIG_API_KEY` and `SWIG_CONFIG_ADDRESS`. Reads need no RPC URL or
signing key and never create a wallet. An optional `SWIG_WALLET_ADDRESS`
must match the configured wallet. Wallet reads rely on the backend's indexer.

```sh
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example wallet_reads
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example wallet_reads -- --policy-id YOUR_POLICY_ID
```

The runner passes the optional policy ID to
`wallet_reads::example(policy_id: Option<String>)`; no extra env variable is needed.

## Run an Ed25519 SOL transfer

Set `SWIG_API_KEY`, `SWIG_RPC_URL`, and `SWIG_ED25519_PRIVATE_KEY`.
This flow is Devnet-only. It creates a wallet when `SWIG_CONFIG_ADDRESS`
is absent, or reuses that existing wallet when supplied.

```sh
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example ed25519_transfer
```

The default transfer is 1,000,000 lamports. When the wallet needs SOL,
the flow requests exactly 2 SOL, confirms the airdrop, and rechecks the
balance including rent. A faucet failure or insufficient balance stops it.

## Run a secp256r1 SOL transfer

Set `SWIG_API_KEY`, `SWIG_RPC_URL`, and `SWIG_SECP256R1_PRIVATE_KEY`.
This Devnet-only flow uses a raw secp256r1 key, not a browser passkey
or WebAuthn ceremony. It follows the same wallet reuse, recipient,
1,000,000-lamport default, and fixed 2 SOL funding rules as Ed25519.

```sh
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example secp256r1_transfer
```

## Runner usage

Each runner supports `--help` and rejects unknown arguments before
loading configuration or making requests. For example:

```sh
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example wallet_reads -- --help
```

Signing-flow `example()` functions take no arguments.

## Submission and verification

The examples trust the configured backend's transaction construction.
They do not independently prove business intent or chain freshness.
API acceptance is not chain confirmation. Ordinary transfer flows wait
for a signature and check its success; a timeout can still mean submission
happened. Keep printed addresses and signatures, and reconcile the original
attempt before rerunning when its outcome is uncertain.

## Development checks

Check compilation and lints without executing live flows:

```sh
cargo +1.87.0 check --locked -p swig-developer-sdk-examples --all-targets
cargo +1.87.0 clippy --locked -p swig-developer-sdk-examples --all-targets -- -D warnings
```

These commands make no hosted API or RPC calls and do not establish
live provider or on-chain success.
