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

Hosted wallet operations and native signing land in the dependent branches of
the implementation stack.
