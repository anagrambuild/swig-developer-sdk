# @swig-wallet/developer-sdk

API-key SDK for preparing Swig wallet operations on a server, with a separate
entrypoint for application-owned signing.

- Version: `0.9.0`
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
3. Your application obtains every required transaction-level and embedded
   signature.
4. Your app submits the transactions in order, directly or through Swig's
   paymaster sponsorship.

The API key never leaves your server. The `/signers` entrypoint accepts no API
key and does not call Swig's hosted API.

## Entrypoints

| Import path                          | Use it for                                        |
| ------------------------------------ | ------------------------------------------------- |
| `@swig-wallet/developer-sdk`         | `SwigClient` and shared types                     |
| `@swig-wallet/developer-sdk/signers` | application-owned transaction signing helpers     |
| `@swig-wallet/developer-sdk/core`    | `DEFAULT_BACKEND_URL` and `SwigDeveloperSdkError` |

## Create a server client

Create an API key from the [Swig dashboard](https://dashboard.onswig.com), and
keep it in server-side configuration.

```typescript
import { SwigClient } from '@swig-wallet/developer-sdk';

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

| Field                         | What to do                                                   |
| ----------------------------- | ------------------------------------------------------------ |
| `wallet`                      | `swigConfigAddress`, `walletAddress`, and resolved `network` |
| `transactions`                | submit in this exact order                                   |
| `clientAuthorityTransactions` | get a client authority signature first                       |
| `feePayerOnlyTransactions`    | send or sponsor without a client authority signature         |
| `creationTransaction`         | the create transaction itself                                |

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

### Configure and use a ParticipantSet

Create the set, submit its prepared creation transaction, then attach it to one
Swig role with the permissions your application chooses:

```typescript
const createdSet = await swig.participantSets.create({
  swigConfigAddress,
  feePayer,
  threshold: 2,
  members: [
    { ed25519: { publicKey: recoveryPublicKey } },
    { secp256r1: { publicKey: clientPublicKey } },
    { secp256k1: { publicKey: serverPublicKey } },
  ],
});

const wallet = swig.wallets.use(swigConfigAddress, {
  requesterAuthority,
});
const addRole = await wallet.roles.add({
  feePayer,
  authority: {
    participantSet: { address: createdSet.participantSetAddress },
  },
  actions: [
    { type: 'solLimit', amount: 1_000_000n },
    { type: 'program', programId },
  ],
});
```

After those transactions land, use the set as the requester for a transaction
preparation route:

```typescript
const participantWallet = swig.wallets.use(swigConfigAddress, {
  requesterAuthority: {
    participantSet: { address: createdSet.participantSetAddress },
  },
});
const prepared = await participantWallet.transfer.sol({
  feePayer,
  destination,
  amount: 1_000_000n,
});

const compiled = await swig.transactions.compileParticipantSetApprovals({
  preparedTransaction: prepared,
  approvals,
});
```

Each member approval signs the challenge returned for that member and the plan
uses one shared nonce. Compilation validates the plan through the API and
returns `compiled.transaction`, an RPC-simulated unsigned transaction.
Sponsorship or submission remains a separate explicit call.

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

### Prepare an x402 payment

Pass the resource provider's `402 Payment Required` response directly to the
wallet. When `acceptedIndex` is omitted, Swig selects the first eligible exact
Solana requirement and returns its original array index.

```typescript
import { SwigClient } from '@swig-wallet/developer-sdk';
import {
  createX402Payment,
  signPreparedTransaction,
} from '@swig-wallet/developer-sdk/signers';

const swig = new SwigClient({
  apiKey: process.env.SWIG_API_KEY!,
  network: 'devnet',
});
const wallet = swig.wallets.use({
  swigConfigAddress,
  walletAddress,
  requesterAuthority: {
    ed25519: {
      publicKey: userPublicKey,
    },
  },
});

const challenge = await fetch(resourceUrl);
const prepared = await wallet.x402.prepareFromResponse(challenge);
const signed = await signPreparedTransaction(prepared.preparedTransaction, {
  signTransaction,
});
const payment = createX402Payment(prepared, signed);

const paidResponse = await fetch(resourceUrl, {
  headers: payment.paymentSignatureHeaders,
});
```

To select a specific offer, pass its index from the original `accepts` array:

```typescript
const prepared = await wallet.x402.prepareFromResponse(challenge, {
  acceptedIndex,
});
```

Use `signPreparedTransaction` for Ed25519 authorities. For
Secp256r1/passkey authorities, use `signPreparedSwigTransaction` to fulfill
the prepared transaction's signature requests.

See the [complete runnable x402 example](https://github.com/anagrambuild/swig-developer-sdk/tree/main/typescript/examples/x402).

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

`swig.ramp` covers both directions. Direction is carried by the `buy` or `sell`
order you pass, so there is no separate on-ramp or off-ramp client. Every call
requires a `configurationId` and an `environment` of `'sandbox'` or
`'production'`; `getOrder`, `prepareTransfer`, and `submitTransfer` identify the
order in the route and take no environment.

Amounts are integers in the smallest unit the thing has — `minorUnits` for fiat
(cents for USD, whole yen for JPY) and `baseUnits` for crypto. They cross the
wire as decimal strings, so a value above 2^53 survives; pass a `bigint`,
`number`, or `string` and read a `string` back. Crypto is a `CryptoAsset` of
`{ type: 'sol' }` or `{ type: 'token', mint }`.

| Client method               | Route                                                      |
| --------------------------- | ---------------------------------------------------------- |
| `swig.ramp.getOptions`      | `GET /wallet/api/ramp/options`                             |
| `swig.ramp.getQuotes`       | `POST /wallet/api/ramp/quotes`                             |
| `swig.ramp.createOrder`     | `POST /wallet/api/ramp/orders`                             |
| `swig.ramp.getOrder`        | `GET /wallet/api/ramp/orders/{order_id}`                   |
| `swig.ramp.prepareTransfer` | `POST /wallet/api/ramp/orders/{order_id}/transfer/prepare` |
| `swig.ramp.submitTransfer`  | `POST /wallet/api/ramp/orders/{order_id}/transfer/submit`  |

### Options and quotes

Read the options first, so currency, payment-method, and asset values come from
the API rather than a hardcoded list. `AssetOption.decimals` and
`FiatCurrencyOption.exponent` are the scales you need to build a valid amount.

```typescript
const options = await swig.ramp.getOptions({
  configurationId,
  environment: 'sandbox',
  direction: 'buy',
  countryCode: 'US',
});

const quotes = await swig.ramp.getQuotes({
  configurationId,
  environment: 'sandbox',
  location: { countryCode: 'US' },
  order: {
    type: 'buy',
    spend: { currencyCode: 'USD', minorUnits: 10_000n },
    receive: { type: 'token', mint: usdcMint },
  },
});
```

Quotes carry no identifier and must never be cached. Pick one and pass its
`route` to `createOrder`; the route is re-priced when the order is created.

### Orders

`requestId` is your idempotency key and is unique within the configuration.
Repeating it returns the stored order; repeating it with different inputs is
refused, so mint it once and reuse it across retries.

```typescript
const order = await swig.ramp.createOrder({
  requestId: crypto.randomUUID(),
  configurationId,
  environment: 'sandbox',
  context: {
    customerId,
    swigConfigAddress,
    location: { countryCode: 'US' },
  },
  route: quotes[0].route,
  order: {
    type: 'buy',
    spend: { currencyCode: 'USD', minorUnits: 10_000n },
    receive: { type: 'token', mint: usdcMint },
  },
});
```

A buy sends the customer to `order.launchUrl`. Poll `swig.ramp.getOrder` until
the status is final; a read of a non-final order also reconciles it against the
provider. `refunded` can follow `settled`.

> A `launchUrl` is a user-specific session URL. Hand it to the customer who owns
> the order and keep it out of logs and analytics.

### Selling

A sell waits for `awaiting-transfer` and a `deposit`, then moves the crypto from
the Swig. Your application owns the signing step.

```typescript
import { signPreparedTransaction } from '@swig-wallet/developer-sdk/signers';

const prepared = await swig.ramp.prepareTransfer({
  orderId: order.id,
  requesterAuthority: { ed25519: { publicKey: requester } },
  feePayer,
});

const signed = await signPreparedTransaction(
  prepared.preparedTransaction,
  signer,
);

const transfer = await swig.ramp.submitTransfer({
  orderId: order.id,
  transferId: prepared.transfer.transferId,
  signedTransaction: signed.transaction,
});
```

The prepared transaction is handed over once. If you broadcast it and then lose
it, call `submitTransfer` again without `signedTransaction` to resolve the
attempt that is already live.

## Signing

Use the signing entrypoint with an application-owned wallet, passkey, hardware
device, or custody service. It accepts no API key and does not call the Swig
API.

### ParticipantSet approvals

Participant signers are detached from `SwigClient`. They receive one bound
member request and return its member index; the shared ParticipantSet nonce is
already committed by the challenge and never copied into an individual proof.

```typescript
import {
  createParticipantEd25519Signer,
  createParticipantPasskeySigner,
  createParticipantPersonalSignSigner,
  signParticipantSetApproval,
} from '@swig-wallet/developer-sdk/signers';

const recoverySigner = createParticipantEd25519Signer({
  publicKey: recoveryPublicKey,
  signMessage: signEd25519,
});
const recoveryApproval = await signParticipantSetApproval(
  prepared.participantSetApprovalPlan!.members[0]!,
  recoverySigner,
);

const passkeySigner = createParticipantPasskeySigner({
  publicKey: clientPublicKey,
  credentialId,
  userVerification: 'preferred',
});
const clientApproval = await signParticipantSetApproval(
  prepared.participantSetApprovalPlan!.members[1]!,
  passkeySigner,
);

const serverSigner = createParticipantPersonalSignSigner({
  publicKey: serverPublicKey,
  // personalSign must apply the EIP-191 personal-sign prefix to this exact
  // 64-character lowercase ASCII hex challenge.
  signMessage: personalSign,
});
const serverApproval = await signParticipantSetApproval(
  prepared.participantSetApprovalPlan!.members[2]!,
  serverSigner,
);
```

The Ed25519 callback receives the decoded 32-byte challenge. The passkey helper
returns raw authenticator data, exact `clientDataJSON`, and converts DER to a
raw low-S P-256 signature; compilation also defensively normalizes externally
constructed high-S P-256 approvals. The personal-sign helper validates the
compact k1 signature, normalizes it to low-S, and adjusts the recovery byte.
None of these helpers calls the hosted API or compiles the transaction.

### Ed25519

```typescript
import {
  signPreparedTransaction,
  type PreparedTransaction,
} from '@swig-wallet/developer-sdk/signers';
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
transfer, swap, and ramp transfer.

```typescript
import {
  createSecp256r1PasskeySigningFn,
  signPreparedSwigTransaction,
  signPreparedSwigTransactions,
  type PreparedTransaction,
} from '@swig-wallet/developer-sdk/signers';

const passkeySigningFn = createSecp256r1PasskeySigningFn({
  allowCredentials: [{ id: credentialId, type: 'public-key' }],
  userVerification: 'preferred',
});

// The signers entrypoint only signs. Your app owns fetching the prepared payload from
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
} from '@swig-wallet/developer-sdk/signers';

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

For wallet creation, authority-sign only `created.clientAuthorityTransactions`.

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

For single-transaction sponsorship, `network` resolves from the call and then
the client default. If neither is set, the paymaster defaults to mainnet.

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

## One Business grant access

Use this when a local app needs an admin to grant one of the app's keys access
to an existing One Business Swig. The local app sends the admin to One Business,
then reads the result on its callback page.

```typescript
import {
  buildOneBusinessGrantAccessUrl,
  completeOneBusinessGrantAccess,
} from '@swig-wallet/developer-sdk';

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
