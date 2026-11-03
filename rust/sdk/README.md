# Swig Developer SDK for Rust

Native Rust SDK for the Swig Developer API. Requires Rust 1.87 or later.

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

Foundation tests run in-process without servers, sockets, or real-time waits.
They capture requests and consume queued responses. Retry tests use zero delay.
Retry-schedule tests compare durations without sleeping.

The private HTTP helpers return successful response bytes unchanged. Endpoint
methods deserialize those bytes and report their own response errors. HTTP
failures use canonical gRPC statuses or HTTP status fallbacks, without retaining
upstream messages.

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
- Grant `WalletPermission::ManageAuthorizationLocks { max_duration_slots }`
  in addition to spending permissions, even for a role with `All`.
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

CI is unchanged in this draft and does not currently run the Rust suite.
Run `cargo test --workspace --locked` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` from `rust/`.
