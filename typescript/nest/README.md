# @swig-wallet/developer-sdk/nest

NestJS adapter for the Swig Developer SDK proxy.

Use this when your client app needs a NestJS route to prepare transactions. The
adapter keeps your Swig developer API key on the server; the client calls the
route, signs the prepared transaction with
`@swig-wallet/developer-sdk/client`, then sends it directly or submits it to a
sponsor endpoint.

## Controller setup

Create one controller mounted at your Swig proxy prefix. The handler routes
`GET` and `POST` itself, so bind it with `@All`:

```typescript
import { All, Controller, Req, Res } from '@nestjs/common';
import { createSwigNestHandler } from '@swig-wallet/developer-sdk/nest';
import type { Request, Response } from 'express';

const swigHandler = createSwigNestHandler();

@Controller('swig')
export class SwigController {
  @All('*')
  handle(@Req() request: Request, @Res() response: Response) {
    return swigHandler(request, response);
  }
}
```

## Routes the handler expects

Preparation and submission:

```text
POST /swig/wallet/create
POST /swig/prepare
POST /swig/transfer/sol
POST /swig/transfer/spl-token
POST /swig/swap/jupiter
```

Wallet and paymaster reads:

```text
GET  /swig/wallet/:swigConfigAddress/balance/usd
GET  /swig/wallet/:swigConfigAddress/token-balances
GET  /swig/wallet/:swigConfigAddress/token-transactions
GET  /swig/wallet/:swigConfigAddress/roles
GET  /swig/paymaster/balance
```

Ramp:

```text
GET  /swig/ramp/onramp/options
POST /swig/ramp/onramp/quote
POST /swig/ramp/onramp/session
GET  /swig/ramp/onramp/session/:sessionId
GET  /swig/ramp/offramp/options
POST /swig/ramp/offramp/quote
POST /swig/ramp/offramp/session
POST /swig/ramp/offramp/session/:sessionId/prepare
POST /swig/ramp/offramp/session/:sessionId/submit
GET  /swig/ramp/offramp/session/:sessionId
```

Ramp read routes require `environment=sandbox` or `environment=production` as a
query parameter, and the options routes also require
`organizationMeldConfigurationId`.

## Configuration

By default the handler reads environment variables:

```bash
SWIG_DEVELOPER_API_KEY=...   # or SWIG_API_KEY
SWIG_TRANSACTION_API_URL=... # optional; also SWIG_BACKEND_URL
SWIG_FEE_PAYER=...           # optional; also SWIG_TRANSFER_FEE_PAYER
```

Create an API key from the [Swig dashboard](https://dashboard.onswig.com) and
keep it in server-side configuration only.

If no transaction API URL is configured, the SDK uses `https://api.onswig.com`.

A fee payer is required for every preparation route. The handler resolves it in
this order — the `feePayer` config value or function, then `feePayer` in the
request body, then the environment variables above — and returns a `400` if
none of them produce a value.

You can also pass values explicitly:

```typescript
const swigHandler = createSwigNestHandler({
  apiKey: process.env.SWIG_DEVELOPER_API_KEY,
  transactionApiUrl: process.env.SWIG_TRANSACTION_API_URL,
  feePayer: process.env.SWIG_FEE_PAYER,
  network: 'devnet',
});
```

## Client usage

Point your client at the Nest route and sign the prepared payload:

```typescript
import { signPreparedTransaction } from '@swig-wallet/developer-sdk/client';

const { prepared } = await fetch('https://api.example.com/swig/transfer/sol', {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    network: 'devnet',
    wallet: {
      swigConfigAddress,
      walletAddress,
      requesterAuthority: { ed25519: { publicKey: userPublicKey } },
    },
    destination,
    amount: '1000000',
  }),
}).then((response) => response.json());

const signed = await signPreparedTransaction(prepared, {
  signTransaction: signWithUserWallet,
});
```

Preparation routes wrap their result in `{ prepared }`. Read and ramp routes
return the resource body directly. Errors come back as `{ error: string }` with
a `4xx` or `5xx` status.

Browser apps can use `SwigBrowserClient` against the same prefix:

```typescript
import { SwigBrowserClient } from '@swig-wallet/developer-sdk/browser';

const swig = new SwigBrowserClient({ basePath: '/swig', network: 'devnet' });
```

## Server-side requester resolution

If your app resolves the requester from auth context, do it server-side:

```typescript
const swigHandler = createSwigNestHandler({
  resolveRequesterAuthority: async ({ request, wallet }) => {
    return wallet?.requesterAuthority ?? lookupRequesterFromRequest(request);
  },
});
```

`feePayer` accepts the same function form when the payer depends on the caller.
