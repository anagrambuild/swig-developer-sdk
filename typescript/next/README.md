# @swig-wallet/developer-sdk/next

Next.js adapter for the Swig Developer SDK proxy.

Use this when your browser app needs a local Next.js route to prepare
transactions without exposing your Swig developer API key to the browser. The
browser calls this route, signs the prepared transaction with
`@swig-wallet/developer-sdk/client`, then sends it directly or submits it to a
sponsor endpoint.

## Route setup

Create one catch-all route:

```typescript
// app/api/swig/[...swig]/route.ts
import { createSwigRouteHandlers } from '@swig-wallet/developer-sdk/next';

export const { GET, POST } = createSwigRouteHandlers();
```

## Routes the helper handles

Preparation and submission:

```text
POST /api/swig/wallet/create
POST /api/swig/prepare
POST /api/swig/transfer/sol
POST /api/swig/transfer/spl-token
POST /api/swig/swap/jupiter
```

Wallet and paymaster reads:

```text
GET  /api/swig/wallet/:swigConfigAddress/balance/usd
GET  /api/swig/wallet/:swigConfigAddress/token-balances
GET  /api/swig/wallet/:swigConfigAddress/token-transactions
GET  /api/swig/wallet/:swigConfigAddress/roles
GET  /api/swig/paymaster/balance
```

Ramp:

```text
GET  /api/swig/ramp/onramp/options
POST /api/swig/ramp/onramp/quote
POST /api/swig/ramp/onramp/session
GET  /api/swig/ramp/onramp/session/:sessionId
GET  /api/swig/ramp/offramp/options
POST /api/swig/ramp/offramp/quote
POST /api/swig/ramp/offramp/session
POST /api/swig/ramp/offramp/session/:sessionId/prepare
POST /api/swig/ramp/offramp/session/:sessionId/submit
GET  /api/swig/ramp/offramp/session/:sessionId
```

Ramp read routes require `environment=sandbox` or `environment=production` as a
query parameter, and the options routes also require
`organizationMeldConfigurationId`.

## Configuration

By default the route helper reads environment variables:

```bash
SWIG_DEVELOPER_API_KEY=...   # or SWIG_API_KEY
SWIG_TRANSACTION_API_URL=... # optional; also SWIG_BACKEND_URL
SWIG_FEE_PAYER=...           # optional; also SWIG_TRANSFER_FEE_PAYER
```

Create an API key from the [Swig dashboard](https://dashboard.onswig.com). The
key stays on the server; never expose it through a `NEXT_PUBLIC_` variable.

If no transaction API URL is configured, the SDK uses `https://api.onswig.com`.

A fee payer is required for every preparation route. The handler resolves it in
this order — the `feePayer` config value or function, then `feePayer` in the
request body, then the environment variables above — and returns a `400` if
none of them produce a value.

You can also pass values explicitly:

```typescript
export const { GET, POST } = createSwigRouteHandlers({
  apiKey: process.env.SWIG_DEVELOPER_API_KEY,
  transactionApiUrl: process.env.SWIG_TRANSACTION_API_URL,
  feePayer: process.env.SWIG_FEE_PAYER,
  network: 'devnet',
});
```

## Client usage

Once the route is installed, browser code uses the browser client. It calls
your local app route only; the Swig developer API key stays on the server.

```typescript
import { SwigBrowserClient } from '@swig-wallet/developer-sdk/browser';
import { signPreparedTransaction } from '@swig-wallet/developer-sdk/client';

const swig = new SwigBrowserClient({
  basePath: '/api/swig',
  network: 'devnet',
});
const wallet = swig.wallets.fromIdpSession(session);

const prepared = await wallet.transfer.sol({
  destination,
  amount: 1_000_000n,
});

const signed = await signPreparedTransaction(prepared, {
  signTransaction: signWithUserWallet,
});
```

The same wallet handle supports token transfers, Jupiter swaps, grouped
operations, and wallet reads:

```typescript
await wallet.transfer.token({ mint, destinationOwner, amount: '2500' });

await wallet.swap.jupiter({
  inputMint,
  outputMint,
  amount: '1000000',
  slippageBps: 75,
});

const { roles } = await wallet.listRoles();
const { balances, totalUsdValue } = await wallet.listTokenBalances();
```

Ramp is reached through `swig.ramp.onramp` and `swig.ramp.offramp` on the same
browser client, with the same `organizationMeldConfigurationId` and
`environment` fields the server SDK requires.

## Server-side requester resolution

If your app does not include `requesterAuthority` in the wallet reference,
resolve it server-side from your own auth context:

```typescript
export const { GET, POST } = createSwigRouteHandlers({
  resolveRequesterAuthority: async ({ wallet }) => {
    return wallet?.requesterAuthority ?? lookupRequesterForRole(wallet?.roleId);
  },
});
```

The resolver receives the route, the parsed body, the wallet reference, and the
original `Request`, so you can key it on session cookies or headers.

## Fee payer per request

`feePayer` also accepts a function when the payer depends on the caller:

```typescript
export const { GET, POST } = createSwigRouteHandlers({
  feePayer: async ({ request }) => {
    const user = await getUserFromSession(request);
    return user.feePayerPublicKey;
  },
});
```
