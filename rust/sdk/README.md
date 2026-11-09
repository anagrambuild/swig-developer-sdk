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
An already published version is skipped. To recover after publication, rerun
the original failed workflow run so it retains the published commit.

The first publication uses `cargo login`, then
`cargo publish --locked -p swig-developer-sdk` from `rust/`.
For subsequent releases, configure the crate's
[trusted publisher](https://crates.io/docs/trusted-publishing) with repository
`anagrambuild/swig-developer-sdk`, workflow `rust.yml`, and environment `crates-io`.
Restrict that GitHub environment to the `main` branch. CI uses a short-lived
publishing token through GitHub OIDC.

## Authorization locks (draft)

This branch adds hosted lock preparation and reads. It requires the auth-lock
backend routes from portal PR #1170 and the corresponding on-chain program;
it does not depend on the Rust Protocol SDK or its crate publication.

```rust
use solana_pubkey::Pubkey;
use swig_developer_sdk::{
    AuthorizationLockError, AuthorizationLockExpiry, PrepareAddAuthorizationLockArgs,
    PreparedTransaction, WalletAuthority, WalletClient,
};

async fn prepare_hold(
    client: &WalletClient,
    swig_config_address: Pubkey,
    fee_payer: Pubkey,
    requester_authority: WalletAuthority,
) -> Result<PreparedTransaction, AuthorizationLockError> {
    client.prepare_add_authorization_lock(PrepareAddAuthorizationLockArgs {
        swig_config_address,
        fee_payer,
        requester_authority,
        mint: None,
        amount: 1_000_000,
        expiry: AuthorizationLockExpiry::DurationSlots(150),
    }).await
}
```

- Use `Some(mint)` for tokens and `None` for SOL. Amounts are `u64` base units.
- Choose `DurationSlots` or `ExpirySlot`; the enum prevents ambiguous expiry.
- Grant `Permission::ManageAuthorizationLocks { max_duration_slots }` through
  the Protocol SDK in addition to spending permissions, even for `All`. The
  hosted add-role API does not yet accept this permission. Role reads decode it
  as `SwigRolePermission::ManageAuthorizationLocks`.
- `get_authorization_locks(&swig_config_address, include_expired)` returns
  typed lock IDs, amounts, expiry slots, and the service's current slot.
  Set `include_expired` to include both expired and fully captured records.
- `prepare_remove_authorization_lock(PrepareRemoveAuthorizationLockArgs { ... })`
  prepares release by the holder, or pruning by an eligible role once inactive.
- Preparation returns an unsigned transaction. Complete its authority signing
  or ParticipantSet approvals, then fee-payer signing and submission. The Rust
  native secp256k1 and raw secp256r1 signers support both lock instructions.
- Capture happens through the holder's normal spending; there is no separate
  capture endpoint. A captured amount of zero is valid read state.

The existing Rust CI runs formatting, tests, Clippy, and package verification;
its configuration is unchanged in this draft. Run `cargo test --workspace --locked` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` from `rust/`.
