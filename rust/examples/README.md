# Rust SDK examples

1 standalone flow: 1 runnable Cargo example.
Keygen is a separate utility binary. The package is unpublished;
the SDK remains the default workspace member.

Each flow lives in `src/` with its own imports, configuration, and SDK calls.
Runnable flows have thin CLI runners in `examples/`.

## Choose a flow

| Source | Entry point | Flow |
| --- | --- | --- |
| [wallet_reads.rs](src/wallet_reads.rs) | `--example wallet_reads` | Read roles, balances, history, and optionally a policy |

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

## Runner usage

Each runner supports `--help` and rejects unknown arguments before
loading configuration or making requests. For example:

```sh
cargo +1.87.0 run --locked -p swig-developer-sdk-examples --example wallet_reads -- --help
```

## Development checks

Check compilation and lints without executing live flows:

```sh
cargo +1.87.0 check --locked -p swig-developer-sdk-examples --all-targets
cargo +1.87.0 clippy --locked -p swig-developer-sdk-examples --all-targets -- -D warnings
```

These commands make no hosted API or RPC calls and do not establish
live provider or on-chain success.
