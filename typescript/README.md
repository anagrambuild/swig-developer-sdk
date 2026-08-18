# @swig-wallet/developer-sdk

API-key SDK for preparing Swig wallet operations on a server and signing the
prepared transactions on a client.

- Version: `0.8.0`
- Source: <https://github.com/anagrambuild/swig-developer-sdk>
- Default API base URL: `https://api.onswig.com`

```bash
bun add @swig-wallet/developer-sdk
```

## How it works

The SDK is prepare-first:

1. Your server creates a `SwigClient` with an API key.
2. Your server prepares a wallet operation and receives one or more unsigned
   transactions.
3. Your client signs any transaction whose `signatureRequests` is non-empty.
4. Your app submits the transactions in order, directly or through Swig's
   paymaster sponsorship.

The API key never leaves your server. Browser code calls a proxy route in your
own app — see [Framework proxy routes](#framework-proxy-routes).

## Entrypoints

| Import path                                    | Use it for                                        |
| ---------------------------------------------- | ------------------------------------------------- |
| `@swig-wallet/developer-sdk`                   | `SwigClient` and shared types                     |
| `@swig-wallet/developer-sdk/server/typescript` | API-key server SDK                                |
| `@swig-wallet/developer-sdk/server`            | server aggregate exports                          |
| `@swig-wallet/developer-sdk/client`            | client-only signing helpers                       |
| `@swig-wallet/developer-sdk/browser`           | `SwigBrowserClient` and One Business helpers      |
| `@swig-wallet/developer-sdk/next`              | Next.js route handlers                            |
| `@swig-wallet/developer-sdk/nest`              | NestJS request handler                            |
| `@swig-wallet/developer-sdk/server/fetch`      | framework-neutral `Request`/`Response` handlers   |
| `@swig-wallet/developer-sdk/core`              | `DEFAULT_BACKEND_URL` and `SwigDeveloperSdkError` |

## Create a server client

Create an API key from the [Swig dashboard](https://dashboard.onswig.com), and
keep it in server-side configuration.

```typescript
import { SwigClient } from '@swig-wallet/developer-sdk/server/typescript';

const swig = new SwigClient({
  apiKey: process.env.SWIG_API_KEY!,
  network: 'mainnet',
});
```

Requests authenticate with `Authorization: Bearer <api-key>`. Override the base
URL when you target a non-production deployment:

```typescript
const swig = new SwigClient({
  apiKey: process.env.SWIG_API_KEY!,
  baseUrl: 'http://localhost:8080',
  network: 'devnet',
  retryOptions: { maxRetries: 3, retryDelay: 1000, backoffMultiplier: 2 },
});
```

### Retry behavior

- `GET` requests use the configured retry policy.
- `POST` requests do **not** retry by default, because replaying a preparation
  or submission request can duplicate work.
- Sponsorship `POST` requests retry only when you pass an `idempotencyKey`. A
  matching retry returns the original paymaster response.
- `4xx` responses throw immediately as `SwigDeveloperSdkError`; `5xx` and
  network failures are what the retry policy covers.

## Create a wallet

```typescript
const created = await swig.wallets.create({
  feePayer,
  initialUser: {
    ed25519: { publicKey: userPublicKey },
  },
});

return {
  wallet: created.wallet,
  transactions: created.transactions,
  transactionsToSign: created.clientAuthorityTransactions,
};
```

Pass `secp256r1` for a passkey initial user or `secp256k1` for an EVM key:

```typescript
const created = await swig.wallets.create({
  feePayer,
  initialUser: {
    secp256r1: { publicKey: passkeyPublicKey },
  },
});
```

If `policyId` is omitted, the backend creates the wallet from the inline
`initialUser`. If `policyId` is provided, the SDK also reads the policy so it
can return policy-derived metadata alongside the transactions.

The response splits the prepared transactions so your app knows what to do
next:

| Field                         | What to do                                                       |
| ----------------------------- | ---------------------------------------------------------------- |
| `wallet`                      | `swigConfigAddress`, `walletAddress`, and resolved `network`     |
| `transactions`                | submit in this exact order                                       |
| `clientAuthorityTransactions` | get a client authority signature first                           |
| `feePayerOnlyTransactions`    | send or sponsor without a client authority signature             |
| `operatorSignedTransactions`  | already backend-signed; only needs fee payer or sponsor handling |
| `creationTransaction`         | the create transaction itself                                    |

A prepared transaction needs a client authority signature when
`signatureRequests.length > 0`.

## Attach to an existing wallet

```typescript
const wallet = swig.wallets.use({
  swigConfigAddress,
  walletAddress,
  requesterAuthority: {
    ed25519: { publicKey: userPublicKey },
  },
});
```

`swig.wallets.use` also accepts a bare `swigConfigAddress` string plus options,
and `swig.wallets.fromIdpSession(session)` builds the same handle from an IdP
session.

### Prepare a SOL transfer

```typescript
const preparedTransfer = await wallet.transfer.sol({
  feePayer,
  destination,
  amount: 1_000_000n,
});
```

### Prepare a token transfer

Token transfers are owner-based. The backend derives the token program, source
ATA, destination ATA, and any destination ATA creation.

```typescript
const preparedTokenTransfer = await wallet.transfer.token({
  feePayer,
  mint,
  destinationOwner,
  amount: 10_000n,
});
```

`wallet.transfer.splToken` is an alias, and calling `wallet.transfer(args)`
directly routes to the SOL or token endpoint based on whether `mint` is set.

### Prepare a Jupiter swap

```typescript
const preparedSwap = await wallet.swap.jupiter({
  feePayer,
  inputMint: 'So11111111111111111111111111111111111111112',
  outputMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  amount: 10_000n,
  slippageBps: 100,
  destinationAccount,
  wrapAndUnwrapSol: true,
});
```

`destinationAccount` is the recipient owner. The backend derives the output
token ATA for SPL outputs, or the native destination for unwrapped SOL.

### Prepare grouped operations

Use `wallet.prepare` when several operations should be built into one
backend-shaped plan. The backend decides the final instruction layout and
derives token accounts for token transfers.

```typescript
const prepared = await wallet.prepare({
  feePayer,
  operations: [
    { type: 'transferSol', destination, amount: 1_000_000n },
    { type: 'transferToken', mint, destinationOwner, amount: 10_000n },
  ],
});

return {
  wallet: prepared.wallet,
  transactions: prepared.transactions,
  clientAuthorityTransactions: prepared.clientAuthorityTransactions,
  feePayerOnlyTransactions: prepared.feePayerOnlyTransactions,
};
```

### Build a custom transaction

Use `wallet.buildTransaction` when your application supplies raw Solana
instructions. The backend wraps them in the Swig signing flow and returns an
unsigned transaction; signing and submission stay in your app.

```typescript
const prepared = await wallet.buildTransaction({
  feePayer,
  instructions: [
    {
      programId,
      accounts: [{ pubkey: destination, isWritable: true }],
      data: instructionData,
    },
  ],
  addressLookupTableAccounts,
});
```

## Read wallet state

All wallet reads are API-key `GET` requests and follow the retry policy.

```typescript
const usd = await wallet.getUsdBalance();
// usd.swigConfigAddress, usd.walletAddress, usd.usdValue

const tokens = await wallet.listTokenBalances();
// tokens.balances[].mintAddress, assetKind, uiAmount, usdValue

const activity = await wallet.listTokenTransactions({ limit: 25 });
// activity.transactions[].transactionSignature, direction, assetKind, uiAmount
```

### Roles

`wallet.listRoles()` returns the on-chain roles attached to the Swig, which is
how you inspect who currently holds authority and what each authority may do.

```typescript
const { swigConfigAddress, walletAddress, roles } = await wallet.listRoles();

for (const role of roles) {
  console.log(role.roleId, role.authorityType, role.authorityValue);
  for (const action of role.actions) {
    console.log(action.actionIndex, action.actionCode, action.actionData);
  }
}
```

`authorityValue` is the authority's public key material and `authorityType` is
the protocol authority type discriminant. `actionData` is the raw per-action
payload, so read it against the protocol's action definitions rather than
assuming a fixed shape.

Policy metadata is a separate read:

```typescript
const policy = await swig.wallets.getPolicy(policyId);
```

## Fiat ramps

Ramp is split into `swig.ramp.onramp` and `swig.ramp.offramp`. Every ramp call
requires an `environment` of `'sandbox'` or `'production'`; the SDK encodes it
as the MELD enum on the wire. Options, quotes, and session creation also require
`organizationMeldConfigurationId`. Quotes additionally require
`externalCustomerId`, `swigConfigAddress`, and a network (from the argument or
the client default).

| Client method                            | Route                                                        |
| ---------------------------------------- | ------------------------------------------------------------ |
| `swig.ramp.onramp.getOptions`            | `GET /wallet/api/ramp/onramp/options`                        |
| `swig.ramp.onramp.quote`                 | `POST /wallet/api/ramp/onramp/quote`                         |
| `swig.ramp.onramp.createSession`         | `POST /wallet/api/ramp/onramp/session`                       |
| `swig.ramp.onramp.getSession`            | `GET /wallet/api/ramp/onramp/session/{session_id}`           |
| `swig.ramp.offramp.getOptions`           | `GET /wallet/api/ramp/offramp/options`                       |
| `swig.ramp.offramp.quote`                | `POST /wallet/api/ramp/offramp/quote`                        |
| `swig.ramp.offramp.createSession`        | `POST /wallet/api/ramp/offramp/session`                      |
| `swig.ramp.offramp.prepareAuthorization` | `POST /wallet/api/ramp/offramp/session/{session_id}/prepare` |
| `swig.ramp.offramp.submitAuthorization`  | `POST /wallet/api/ramp/offramp/session/{session_id}/submit`  |
| `swig.ramp.offramp.getSession`           | `GET /wallet/api/ramp/offramp/session/{session_id}`          |

### Onramp

```typescript
const environment = 'sandbox';
const organizationMeldConfigurationId = process.env.SWIG_MELD_CONFIG_ID!;

const options = await swig.ramp.onramp.getOptions({
  organizationMeldConfigurationId,
  environment,
  countryCode: 'US',
});

const { quotes } = await swig.ramp.onramp.quote({
  organizationMeldConfigurationId,
  environment,
  externalCustomerId,
  swigConfigAddress,
  network: 'devnet',
  sourceAmount: '100.00',
  sourceCurrencyCode: options.fiatCurrencyCodes[0]!,
  destinationCurrencyCode: options.cryptoCurrencyCodes[0]!,
  countryCode: 'US',
  paymentMethodType: options.paymentMethodTypes[0],
});

const session = await swig.ramp.onramp.createSession({
  organizationMeldConfigurationId,
  environment,
  quoteId: quotes[0]!.quoteId,
});

// Send session.launchUrl to the user. Treat it as a user-specific secret.
const state = await swig.ramp.onramp.getSession({
  sessionId: session.sessionId,
  environment,
});
// state.status is one of 'unspecified' | 'created' | 'pending' | 'settling'
// | 'settled' | 'failed' | 'declined' | 'cancelled' | 'refunded'
```

### Offramp

Offramp adds an on-chain authorization step: the user's wallet must sign the
transfer to the provider before the session can settle.

```typescript
import { signPreparedSwigTransaction } from '@swig-wallet/developer-sdk/client';

const options = await swig.ramp.offramp.getOptions({
  organizationMeldConfigurationId,
  environment,
  countryCode: 'US',
});

const { quotes } = await swig.ramp.offramp.quote({
  organizationMeldConfigurationId,
  environment,
  externalCustomerId,
  swigConfigAddress,
  network: 'mainnet',
  sourceAmount: '25.00',
  sourceCurrencyCode: options.cryptoCurrencies[0]!.currencyCode,
  destinationCurrencyCode: options.fiatCurrencyCodes[0]!,
  countryCode: 'US',
  paymentMethodType: options.paymentMethodTypes[0],
});

const session = await swig.ramp.offramp.createSession({
  organizationMeldConfigurationId,
  environment,
  quoteId: quotes[0]!.quoteId,
});

const authorization = await swig.ramp.offramp.prepareAuthorization({
  sessionId: session.sessionId,
  environment,
  feePayer,
  requesterAuthority: { secp256r1: { publicKey: passkeyPublicKey } },
});

// Show authorization.display to the user, then sign on the client.
const signed = await signPreparedSwigTransaction(
  authorization.preparedTransaction,
  { secp256r1: passkeySigningFn },
);

const { solanaSignature } = await swig.ramp.offramp.submitAuthorization({
  sessionId: session.sessionId,
  environment,
  authorizationId: authorization.authorizationId,
  signedTransaction: signed.transaction,
});
```

`authorization.display` carries the human-readable transfer summary
(`sourceAmount`, `destinationAmount`, `serviceProvider`, wallet addresses, and
optional `paymentMethodType` / `providerDestinationAmount`).

## Client signing

Client code should only sign prepared transactions. It should not hold the API
key or call the Swig API directly.

### Ed25519

```typescript
import {
  signPreparedTransaction,
  type PreparedTransaction,
} from '@swig-wallet/developer-sdk/client';
import { VersionedTransaction } from '@solana/web3.js';

declare const prepared: PreparedTransaction;

const signed = await signPreparedTransaction(prepared, {
  signTransaction: async (transaction) => {
    const versioned = VersionedTransaction.deserialize(
      Buffer.from(transaction, 'base64'),
    );
    versioned.sign([userKeypair]);
    return Buffer.from(versioned.serialize()).toString('base64');
  },
});
```

### Passkeys (secp256r1)

Use this for any prepared transaction whose `signatureRequests` contains a
`secp256r1` request. The same pattern applies to create, transfer, token
transfer, swap, and offramp authorization.

```typescript
import {
  createSecp256r1PasskeySigningFn,
  signPreparedSwigTransaction,
  signPreparedSwigTransactions,
  type PreparedTransaction,
} from '@swig-wallet/developer-sdk/client';

const passkeySigningFn = createSecp256r1PasskeySigningFn({
  allowCredentials: [{ id: credentialId, type: 'public-key' }],
  userVerification: 'preferred',
});

// The client SDK only signs. Your app owns fetching the prepared payload from
// your own route:
//
// const prepared = await fetch('/your-app-prepare-route').then((r) => r.json());
declare const prepared: PreparedTransaction;

const signed = await signPreparedSwigTransaction(prepared, {
  secp256r1: passkeySigningFn,
});

declare const created: { clientAuthorityTransactions: PreparedTransaction[] };
const signedCreateTransactions = await signPreparedSwigTransactions(
  created.clientAuthorityTransactions,
  { secp256r1: passkeySigningFn },
);
```

### EVM keys (secp256k1)

The helper wraps an EIP-1193 provider with `personal_sign` and adds the
Ethereum message prefix expected by Swig's secp256k1 authority payload.

```typescript
import {
  createSecp256k1EvmSigningFn,
  signPreparedSwigTransaction,
  type PreparedTransaction,
} from '@swig-wallet/developer-sdk/client';

declare const prepared: PreparedTransaction;
declare const evmAddress: string;

const evmSigningFn = createSecp256k1EvmSigningFn({
  provider: window.ethereum,
  address: evmAddress,
});

const signed = await signPreparedSwigTransaction(prepared, {
  secp256k1: evmSigningFn,
});
```

For wallet creation, sign only `created.clientAuthorityTransactions`. Do not
authority-sign `created.operatorSignedTransactions`; those already carry a
backend signature and only need the final fee-payer or sponsor step.

## Submit and sponsor

After signing, submit through your own RPC path, or hand the signed transaction
to Swig's paymaster on the server.

```typescript
const submitted = await swig.transactions.sponsor({
  transaction: signed.transaction,
  network: 'mainnet',
  idempotencyKey,
});

// submitted.requestId, submitted.signature, submitted.spentByPaymaster
```

`sponsor` handles the deployed paymaster route and the base58 payload encoding
the backend expects. Pass `idempotencyKey` whenever your application may retry;
that is the only case in which the SDK retries a sponsorship POST.

A returned signature means the Solana RPC accepted the transaction. It may
still be pending and is not confirmation or finality; track it through your
RPC provider when your product needs either.

### Bundle sponsorship

`sponsorBundle` submits one to five related transactions as a single bundle —
for example a create plus an add-authority transaction, or an ordered plan
returned by `wallet.prepare`.

```typescript
const bundle = await swig.transactions.sponsorBundle({
  transactions: [signedCreate.transaction, signedAddAuthority.transaction],
  network: 'mainnet',
  idempotencyKey,
});

// bundle.requestId, bundle.bundleId, bundle.signatures,
// bundle.estimatedSpentByPaymaster
```

Constraints enforced by the SDK before the request is sent:

- mainnet only — any other network throws
- one to five transactions per bundle
- `signatures` come back in bundle order
- `estimatedSpentByPaymaster` is an estimate, not a settled charge

A returned `bundleId` means Jito accepted the bundle. It may still be pending;
acceptance is not confirmation or finality.

## Framework proxy routes

If your app only needs an API-key proxy and does not need to inspect or modify
prepared transactions, use the framework route helpers:

- [Next.js](./next/README.md)
- [NestJS](./nest/README.md)

Both wrap `createSwigFetchHandler` / `createSwigGetHandler` from
`@swig-wallet/developer-sdk/server/fetch`, which you can also mount directly in
any `Request`/`Response` runtime.

Once a proxy route exists, browser code uses `SwigBrowserClient`, which calls
only your local routes:

```typescript
import { SwigBrowserClient } from '@swig-wallet/developer-sdk/browser';

const swig = new SwigBrowserClient({
  basePath: '/api/swig',
  network: 'devnet',
});
const wallet = swig.wallets.fromIdpSession(session);

const prepared = await wallet.transfer.sol({ destination, amount: 1_000_000n });
const roles = await wallet.listRoles();
```

## One Business grant access

Use this when a local app needs an admin to grant one of the app's keys access
to an existing One Business Swig. The local app sends the admin to One Business,
then reads the result on its callback page.

```typescript
import {
  buildOneBusinessGrantAccessUrl,
  completeOneBusinessGrantAccess,
} from '@swig-wallet/developer-sdk/browser';

const grantUrl = buildOneBusinessGrantAccessUrl({
  swigPubkey,
  authorityPublicKey: appAuthorityPublicKey,
  appName: 'Local Trading App',
  redirectUri: 'http://localhost:5173/swig/grant/callback',
  state: crypto.randomUUID(),
  actions: [
    {
      type: 'transferToken',
      mint: usdcMint,
      amount: '10000000',
      cadence: 'daily',
    },
  ],
});

window.location.assign(grantUrl);

// In /swig/grant/callback:
const grant = completeOneBusinessGrantAccess(window.location.href);
// grant.roleId and grant.walletAddress identify the newly granted authority.
```

## Errors

Failed requests throw `SwigDeveloperSdkError` with a `code` and `statusCode`.

```typescript
import { SwigDeveloperSdkError } from '@swig-wallet/developer-sdk/core';

try {
  await wallet.transfer.sol({ feePayer, destination, amount: 1_000_000n });
} catch (error) {
  if (error instanceof SwigDeveloperSdkError) {
    console.error(error.statusCode, error.code, error.message);
  }
  throw error;
}
```

Keep API keys, signed transactions, and ramp launch URLs out of logs.
