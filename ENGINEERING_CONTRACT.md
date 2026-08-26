# Developer SDK engineering contract

This document records the code-backed contract used for the standalone SDK
migration. It is an engineering review artifact, not a product tutorial.

## Baseline

- Backend source: `anagrambuild/swig-dev-portal` at `eac45da`.
- SDK source history: `anagrambuild/swig-ts/packages/developer-sdk` at
  `6fc9e22`, preserved with `git subtree split`.
- Standalone repository: `anagrambuild/swig-developer-sdk`.
- Public API base URL: `https://api.onswig.com`.
- TypeScript package: `@swig-wallet/developer-sdk` version `0.9.0`.
- Python package: `swig-developer-sdk` version `0.8.0`.

The `0.9.0` TypeScript release removes the browser proxy and framework adapter
entrypoints. Application-owned signing helpers now use the dedicated
`@swig-wallet/developer-sdk/signers` entrypoint.

## Authentication and retry boundary

- Server SDK requests authenticate with `Authorization: Bearer <api-key>`.
- The API gateway also accepts `x-api-key`; SDK examples use Bearer auth.
- API keys stay on the server. The `/signers` entrypoint accepts prepared
  transactions and application-provided signers; it does not call the hosted
  API.
- Python exposes the same boundary through `swig_developer_sdk.signers`; it
  exports no API client, and its helpers construct no client and make no hosted
  requests.
- GET requests use the configured retry policy.
- POST requests do not retry by default because replay may duplicate work.
- Sponsorship POST requests retry only when an idempotency key is present.

## Transaction API

Supported preparation routes:

| Method | Route |
| --- | --- |
| POST | `/transaction/prepare/batch` |
| POST | `/transaction/wallet/create` |
| POST | `/transaction/transfer/sol` |
| POST | `/transaction/transfer/spl-token` |
| POST | `/transaction/swap/jupiter` |
| POST | `/transaction/prepare/custom` |

Wallet preparation idempotency fields were removed because the backend does
not consume them. Prepared transactions are signed locally before submission.
Prepared transaction responses also carry `network` and zero or more signature
requests. Each signature request identifies the scheme, signer, message hash,
slot, and counter.

Recovery routes exist in the backend and remain represented by guarded SDK
types, but the recovery product flow is currently broken. Do not document,
link, promote, or provide setup/runtime examples for recovery in the Developer
SDK or Developer API documentation until it is repaired.

## Wallet API

| Method | Route | Authentication |
| --- | --- | --- |
| POST | `/wallet/swig/lookup` | public client boundary |
| GET | `/wallet/swig/status` | public client boundary |
| GET | `/wallet/policies/{policy_id}` | API key |
| GET | `/wallet/swig/{swig_config_address}/balance/usd` | API key |
| GET | `/wallet/swig/{swig_config_address}/token-balances` | API key |
| GET | `/wallet/swig/{swig_config_address}/token-transactions` | API key |
| GET | `/wallet/swig/{swig_config_address}/roles` | API key |

`/wallet/swig/auth/check` and `/wallet/swig/session` are backend stubs. Do not
advertise them as usable integration flows.

Token balances and token transactions carry `assetKind` in ProtoJSON. Both
SDKs preserve it as `token`, `native-sol`, or `unspecified`.

## Paymaster API

| Method | Route |
| --- | --- |
| GET | `/paymaster/test` |
| GET | `/paymaster/balance` |
| POST | `/paymaster/sign` |
| POST | `/paymaster/sponsor` |
| POST | `/paymaster/sponsor/bundle` |

Single sponsorship returns `request_id`, `signature`, and
`spent_by_paymaster`. It does not return a status field. Bundle sponsorship is
mainnet-only, accepts one to five transactions, and returns `request_id`,
`bundle_id`, `signatures`, and `estimated_spent_by_paymaster`.

A sponsor signature means the Solana RPC accepted the transaction; it may
still be pending and is not confirmation or finality. A bundle ID means Jito
accepted the bundle; it likewise may still be pending.

## Ramp API

Every ramp request requires an environment (`sandbox` or `production`, encoded
on the wire as the MELD enum). Options, quotes, and session creation also
require `organizationMeldConfigurationId`. Quotes additionally require
`externalCustomerId` and `swigConfigAddress`; session reads and off-ramp
authorization actions identify the session in the route path instead.

| Method | Route |
| --- | --- |
| GET | `/wallet/api/ramp/onramp/options` |
| POST | `/wallet/api/ramp/onramp/quote` |
| POST | `/wallet/api/ramp/onramp/session` |
| GET | `/wallet/api/ramp/onramp/session/{session_id}` |
| GET | `/wallet/api/ramp/offramp/options` |
| POST | `/wallet/api/ramp/offramp/quote` |
| POST | `/wallet/api/ramp/offramp/session` |
| POST | `/wallet/api/ramp/offramp/session/{session_id}/prepare` |
| POST | `/wallet/api/ramp/offramp/session/{session_id}/submit` |
| GET | `/wallet/api/ramp/offramp/session/{session_id}` |

The removed generic routes (`/ramp/options`, `/ramp/quote`, `/ramp/sessions`,
and ramp transaction-history routes) are not part of the current backend.
TypeScript and Python expose `swig.ramp.onramp` and `swig.ramp.offramp`.

## Public identity API

The runtime identity endpoints are public IDP routes, not API-key Developer SDK
methods:

| Method | Route |
| --- | --- |
| POST | `/identity/public/auth/start` |
| POST | `/identity/public/auth/email/start` |
| POST | `/identity/public/auth/sms/start` |
| POST | `/identity/public/setup/sponsor` |
| GET/POST | `/identity/public/callback/{provider}` |
| GET | `/identity/public/verify_jwt/jwks.json` |
| GET | `/identity/public/oidc/jwks.json` |
| POST | `/identity/public/auth/email/verify` |
| POST | `/identity/public/auth/email/resend` |
| POST | `/identity/public/auth/sms/verify` |
| POST | `/identity/public/auth/sms/resend` |

There is no `/identity/api/auth/passkey/start` route. The separate IdP SDK owns
identity integration guidance; Developer API docs should describe this boundary
instead of presenting these routes as API-key SDK calls.

## Release trust

- Python publishing workflow identity: repository
  `anagrambuild/swig-developer-sdk`, workflow `publish-python.yml`.
- PyPI trusted publishing for that identity is configured.
- The old `anagrambuild/swig-ts` PyPI publisher must be removed after the
  standalone migration is live.
