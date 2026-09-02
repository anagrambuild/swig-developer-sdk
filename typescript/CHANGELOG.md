# @swig-wallet/developer-sdk

## 0.10.0

### Major Changes

- Replace the direction-specific `swig.ramp.onramp` and `swig.ramp.offramp`
  clients with a single `swig.ramp` client covering both directions:
  `getOptions`, `getQuotes`, `createOrder`, `getOrder`, `prepareTransfer`, and
  `submitTransfer`. Sessions are now orders, and direction is carried by the
  `buy` or `sell` order rather than by the client you call.
- `organizationMeldConfigurationId` becomes `configurationId`, and the
  environment is a ramp environment rather than a MELD one. `getOrder`,
  `prepareTransfer`, and `submitTransfer` no longer take an environment; it is
  a property of the stored order.
- Quotes no longer carry an id and must not be cached. `createOrder` takes the
  chosen `route` plus a caller-generated `requestId` idempotency key, and
  re-prices the route server-side. Repeating a `requestId` returns the stored
  order.
- Amounts are integers in the smallest unit — `minorUnits` for fiat,
  `baseUnits` for crypto — and cross the wire as decimal strings, so a value
  above 2^53 survives. Crypto is a `CryptoAsset` of `{ type: 'sol' }` or
  `{ type: 'token', mint }` rather than a currency code.
- `prepareAuthorization` and `submitAuthorization` become `prepareTransfer` and
  `submitTransfer`, keyed by `orderId` and `transferId`. Calling
  `submitTransfer` without `signedTransaction` resolves an attempt that was
  already broadcast.

## 0.9.0

### Major Changes

- Replace the `/client` and `/browser` entrypoints with the dedicated
  `/signers` entrypoint for application-owned transaction signing.
- Remove the browser proxy and the Next.js, NestJS, Fetch, and server aggregate
  entrypoints. Import the API-key SDK from the package root and expose only the
  application routes your product needs.

### Minor Changes

- Add x402 V2 payment preparation through the wallet client, including optional
  accepted-offer selection and `PAYMENT-SIGNATURE` assembly.

## 0.8.0

### Major Changes

- Replace the generic ramp client with direction-specific `swig.ramp.onramp` and `swig.ramp.offramp` clients. The removed `/ramp/options`, `/ramp/quote`, `/ramp/sessions`, and ramp transaction-history routes are not part of the deployed API. Every ramp call now requires a `sandbox` or `production` environment; options, quotes, and session creation also require `organizationMeldConfigurationId`, while quotes additionally require `externalCustomerId` and `swigConfigAddress`.

### Minor Changes

- Move the package to its own repository at `anagrambuild/swig-developer-sdk`, published alongside the Python `swig-developer-sdk` package at the same version.
- Default the API base URL to `https://api.onswig.com`.
- Add offramp authorization: `prepareAuthorization` returns a prepared transfer plus display fields, and `submitAuthorization` returns the Solana signature.
- Add `wallet.listRoles()` for reading on-chain roles, authorities, and per-role actions.
- Add `swig.transactions.sponsorBundle(...)` for mainnet-only bundles of one to five signed transactions, returning `requestId`, `bundleId`, `signatures`, and `estimatedSpentByPaymaster`.
- Expose wallet reads, paymaster balance, and the ramp routes through the Next.js, NestJS, and fetch proxy adapters and through `SwigBrowserClient`.
- Remove wallet-preparation idempotency fields that the backend does not consume.
- Preserve the backend `assetKind` discriminator on token balances and token transactions as `token`, `native-sol`, or `unspecified`.

### Patch Changes

- Document that `POST` requests do not retry unless an idempotency key makes the retry safe.

## 0.7.0

### Minor Changes

- 57be9a8: Use the batch and custom transaction preparation endpoints, replacing generic wallet execution with `wallet.buildTransaction`.
- ac039aa: Remove unsupported sponsor metadata and retain optional idempotency keys for backend-enforced retries.

## 0.6.0

### Minor Changes

- 4ee9d27: Replace ProgramExecSession requester authorities with ProgramExecProof requester authorities for keyless IdP flows.

## 0.5.1

### Patch Changes

- 6bc43d3: Rebuild the developer SDK package with ProgramExec session authority declarations.

## 0.5.0

### Minor Changes

- 66679f6: Add ProgramExecSession requester authority support and a structural prepared-transaction signer interface for IdP-backed signing flows.

## 0.4.3

### Patch Changes

- 1da14a6: Add IDP paymaster balance reads for One Business funding views.
- 40d8565: Add One Business grant-access URL and callback helpers.

## 0.4.2

### Patch Changes

- 21191af: Add API-key scoped Swig wallet balance, token activity, and paymaster balance read helpers.
- Updated dependencies [c172c01]
  - @swig-wallet/lib@2.1.0

## 0.4.1

### Patch Changes

- bdd0862: Add grouped wallet operation preparation through `wallet.prepare` and the framework proxy, plus a `destinationAccount` swap override.
- 3e331aa: Add client helpers for signing prepared secp256r1 Swig transactions and route transaction sponsorship through the deployed paymaster endpoint.
- b673767: Fix passkey signing to preserve raw WebAuthn `clientDataJSON` in the secp256r1 authority payload.
- eb6b9c4: Simplify Jupiter swap destination arguments to a single recipient owner account.

## 0.4.0

### Minor Changes

- 2c40fad: Return explicit ordered wallet creation transactions, expose per-transaction signature requests, and use requesterAuthority across create, transfer, token transfer, and swap preparation.
- 3fd1a55: Remove internal intent IDs from prepared transaction and wallet creation SDK responses to match the transaction API.

## 0.3.0

### Minor Changes

- 9defe86: Update developer SDK transaction flows for API-prepared wallet creation responses with optional policy IDs, inline initial users, multiple unsigned transactions, add-authority challenges, opinionated SOL/token transfer and Jupiter swap helpers, Jupiter swap proxy support, and local Surfpool smoke coverage.
- 40d1218: Remove `swigId` from wallet handles, wallet references, and prepared wallet responses so SDK consumers use the Swig config address as the wallet identifier.

## 0.2.0

### Minor Changes

- 94ba0f8: Update developer SDK transaction flows for API-prepared wallet creation responses with multiple unsigned transactions, add-authority challenges, Jupiter swap proxy support, and local Surfpool smoke coverage.

## 0.1.1

### Patch Changes

- d55b973: Publish the developer SDK package publicly.

## 0.1.0

### Minor Changes

- Initial API-key developer SDK scaffold.
