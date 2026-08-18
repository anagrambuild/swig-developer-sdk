# Swig Developer SDK

Server-side SDKs for the hosted Swig wallet API. The Swig API prepares wallet
transactions; your application signs and submits them.

Canonical source: <https://github.com/anagrambuild/swig-developer-sdk>

| Package | Language | Version | Directory |
| --- | --- | --- | --- |
| `@swig-wallet/developer-sdk` | TypeScript | `0.8.0` | [`typescript`](./typescript) |
| `swig-developer-sdk` | Python | `0.8.0` | [`python`](./python) |

Both packages target the public API base URL `https://api.onswig.com`.

## The prepare-first boundary

1. Your server creates a client with an API key.
2. Your server prepares a wallet operation and receives one or more unsigned
   transactions.
3. Your client signs the transactions that carry a signature request.
4. Your app submits them directly, or through Swig's paymaster sponsorship.

API keys stay on the server. Browser code talks to a proxy route in your own
app, never to `api.onswig.com` directly.

TypeScript signs with `@solana/web3.js`; Python signs with `solders`. Neither
package sends signing material to the API.

## Install

```bash
# TypeScript
bun add @swig-wallet/developer-sdk

# Python
pip install swig-developer-sdk
```

Create an API key in the [Swig dashboard](https://dashboard.onswig.com).

## Documentation

- [TypeScript package](./typescript/README.md)
  - [Next.js route helpers](./typescript/next/README.md)
  - [NestJS route helper](./typescript/nest/README.md)
- [Python package](./python/README.md)
- [Cross-language parity matrix](./PARITY.md)
- [Engineering contract](./ENGINEERING_CONTRACT.md)

## Repository layout

```text
typescript/   @swig-wallet/developer-sdk sources, tests, and framework adapters
python/       swig-developer-sdk sources and tests
```

## License

Apache-2.0. See [LICENSE.txt](./LICENSE.txt).
