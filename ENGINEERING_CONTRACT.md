# Developer SDK engineering contract

This document records the code-backed contract used for the standalone SDK
migration. It is an engineering review artifact, not a product tutorial.

## Baseline

- Backend source: `anagrambuild/swig-dev-portal` at `0affa0c3`.
- SDK source history: `anagrambuild/swig-ts/packages/developer-sdk` at
  `6fc9e22`, preserved with `git subtree split`.
- Standalone repository: `anagrambuild/swig-developer-sdk`.
- Public API base URL: `https://api.onswig.com`.
- TypeScript package: `@swig-wallet/developer-sdk` version `0.10.0`.
- Python package: `swig-developer-sdk` version `0.9.0`.

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
- Ramp order creation always retries, because `requestId` is required and the
  backend resolves a replay before any eligibility or credential check.
- Ramp quote requests never retry. A retried quote is a different quote, so the
  caller decides whether to ask again.

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
| POST | `/transaction/payment/x402/prepare` |

ParticipantSet setup and stateless approval compilation use:

| Method | Route |
| --- | --- |
| POST | `/transaction/wallet/participant-set/create` |
| POST | `/transaction/wallet/role/add` |
| POST | `/transaction/wallet/participant-set/compile` |

The fresh ParticipantSet contract supports Ed25519, secp256k1, and secp256r1
members and one shared nonce. Transaction preparation routes accept a typed
ParticipantSet requester and may return a `participantSetApprovalPlan` whose
members reuse the standard wallet-authority shape. Compilation sends detached
approvals to the API and returns the RPC-simulated unsigned transaction plus
its authorization expiration slot; it does not sponsor, submit, or broadcast.

`/transaction/wallet/role/add` is the general role endpoint. It accepts the
closed typed action set from `transaction/role.proto`; ParticipantSet is one
supported role authority, not a separate role-creation operation.

Wallet preparation idempotency fields were removed because the backend does
not consume them. Prepared transactions are signed locally before submission.
Prepared transaction responses also carry `network` and zero or more signature
requests. Each signature request identifies the scheme, signer, message hash,
slot, and counter.

The x402 helper validates `PAYMENT-REQUIRED` with the pinned x402 V2 schema,
preserves requirement order and index correspondence while forwarding the
schema-normalized challenge and optional accepted index to the dedicated
preparation route, and assembles `PAYMENT-SIGNATURE` after the prepared
transaction is signed. The SDK does not settle the payment itself.

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

One service covers both directions. Direction is carried by the buy or sell
order rather than a parallel field, so the two cannot disagree. Every request
requires a `configurationId` and a `sandbox` or `production` ramp environment;
order reads and transfer actions identify the order in the route path and take
no environment, because the environment is a property of the stored order.

| Method | Route |
| --- | --- |
| GET | `/wallet/api/ramp/options` |
| POST | `/wallet/api/ramp/quotes` |
| POST | `/wallet/api/ramp/orders` |
| GET | `/wallet/api/ramp/orders/{order_id}` |
| POST | `/wallet/api/ramp/orders/{order_id}/transfer/prepare` |
| POST | `/wallet/api/ramp/orders/{order_id}/transfer/submit` |

Quotes carry no identifier and must never be cached. Order creation takes the
chosen route plus a caller-generated `requestId` idempotency key, unique within
the configuration, and re-prices the route server-side. Repeating a `requestId`
returns the stored order; repeating it with different inputs is refused.

Amounts are integers in the smallest unit — minor units for fiat, base units
for crypto — and are encoded as decimal strings in ProtoJSON, matching the
existing uint64 fields. A value too precise for the currency is refused rather
than rounded. Crypto is a two-case asset: native SOL, encoded as an empty
`{"sol": {}}`, or an SPL mint.

Selling adds a transfer leg. Preparation returns the transfer, the prepared
transaction, and the canonical deposit; the application signs and submits.
The prepared transaction is handed over once, so submitting with an empty
`signedTransaction` resolves an attempt that was already broadcast.

The previous direction-specific surface — `/wallet/api/ramp/{onramp,offramp}/*`
with sessions, quote ids, `organizationMeldConfigurationId`, and the MELD
environment enum — is not part of the current backend.

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
