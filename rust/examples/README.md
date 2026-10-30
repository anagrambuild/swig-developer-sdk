# Rust SDK examples

Workspace setup and a key-generation utility for Rust SDK examples.
The package is unpublished; the SDK remains the default workspace member.

## Setup

Use Rust 1.87 or later. From the repository root:

```sh
cd rust/examples
cp .env.example .env
```

Keep real credentials in the untracked `.env`. The commands below run from
`rust/examples` unless they explicitly name another working directory.

## Keys

Generate throwaway keys with the
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

## Configuration

The [.env.example](.env.example) lists the available settings.
It reserves entries for the private keys printed by keygen.

| Variable | Meaning |
| --- | --- |
| `SWIG_ED25519_PRIVATE_KEY` | 64-byte Solana keypair encoded as 128 hex characters |
| `SWIG_SECP256K1_PRIVATE_KEY`, `SWIG_SECP256R1_PRIVATE_KEY` | Each is a 32-byte secret encoded as 64 hex characters |

## Development checks

Check compilation and lints without executing live flows:

```sh
cargo +1.87.0 check --locked -p swig-developer-sdk-examples --all-targets
cargo +1.87.0 clippy --locked -p swig-developer-sdk-examples --all-targets -- -D warnings
```

These commands make no hosted API or RPC calls and do not establish
live provider or on-chain success.
