# Swig Developer SDK for Rust

Native Rust SDK for the Swig Developer API. Requires Rust 1.87 or later.

For Rust 1.87, set your application's `rust-version = "1.87"` and use
[Cargo resolver 3](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)
in the consuming workspace so dependency resolution selects compatible versions.
Rust 2024 applications use resolver 3 by default; older editions can set
`resolver = "3"` under `[workspace]`. After changing the resolver, run
`cargo update` to refresh an existing lockfile. Newer dependency releases selected
by older resolvers can require a newer compiler.

```sh
cargo add swig-developer-sdk
```

Construct a reusable wallet client with an API key and an explicit network:

```rust
use swig_developer_sdk::{ClientConfig, Network, WalletClient};

let api_key = std::env::var("SWIG_API_KEY")?;
let client = WalletClient::new(ClientConfig::new(api_key, Network::Devnet))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

- Keep the API key on the server. Client construction validates configuration
  and makes no hosted request.
- The default URL is `https://api.onswig.com`. Local development can supply
  an explicit loopback HTTP URL through `config.http.base_url`.
- `config.http.request_timeout_ms` defaults to `None`. Set a positive timeout
  to bound each request, including its response body.
- Eligible operations use three retries by default, with delays of one, two,
  and four seconds. GETs retry transport failures and HTTP 5xx responses.
  Ordinary POSTs run once. Endpoints opt safe POSTs into retries.
  Requests follow no redirects.
- The SDK returns typed errors without emitting its own logs.

Use `WalletClient` to read roles, balances, history, and policies, or prepare
wallet creation, transfers, swaps, custom transactions, and role additions.
Prepared transactions are unsigned. Sign with your application's Ed25519 keypair
or the SDK's secp256r1/secp256k1 signing operations as supported by the prepared
transaction. Participant-set operations prepare sets and collect approvals.

Use `PaymasterClient` for funding balances and transaction or bundle sponsorship,
`RampClient` for quotes and order/transfer operations, and `x402` for payment
requirements and hosted payment preparation. Preserve submission IDs and
signatures to reconcile an uncertain result; API acceptance is separate from
chain confirmation or fiat settlement.

See the [Rust examples](https://github.com/anagrambuild/swig-developer-sdk/tree/main/rust/examples)
for complete configuration, signing, sponsorship, and confirmation flows.

## Releases

Merging a new version in `rust/sdk/Cargo.toml` to `main` runs formatting, tests,
Clippy, and package verification, then publishes to crates.io and creates a
`rust-v<version>` GitHub release. Update `rust/Cargo.lock` with the version bump.
An already published version is skipped. The Rust workflow can also be rerun
manually on `main` to recover a failed release.

The first publication uses `cargo login`, then
`cargo publish --locked -p swig-developer-sdk` from `rust/`.
For subsequent releases, configure the crate's
[trusted publisher](https://crates.io/docs/trusted-publishing) with repository
`anagrambuild/swig-developer-sdk`, workflow `rust.yml`, and environment `crates-io`.
Restrict that GitHub environment to the `main` branch. CI uses a short-lived
publishing token through GitHub OIDC.
