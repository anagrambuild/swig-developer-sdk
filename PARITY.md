# Developer SDK parity

Python mirrors the TypeScript server SDK by behavior and client hierarchy.
Python uses snake_case names and keyword arguments; TypeScript uses camelCase
and options objects. TypeScript is version `0.9.0`, Python is version `0.8.0`,
and both default to `https://api.onswig.com`.

## Client surface

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| API client | `SwigClient` | `SwigClient` | same auth, retry, network, and error behavior |
| Wallet creation | `swig.wallets.create` | `swig.wallets.create` | same request and normalized transaction groups |
| Wallet handle | `wallets.use`, `wallets.fromIdpSession` | `wallets.use`, `wallets.from_idp_session` | same inherited wallet, network, and requester authority |
| Grouped preparation | `wallet.prepare` | `wallet.prepare` | same operation wire shape and normalized response |
| Transfers | `wallet.transfer.sol/token/splToken` | `wallet.transfer.sol/token/spl_token` | same endpoints and prepared transaction output |
| Jupiter swap | `wallet.swap.jupiter` | `wallet.swap.jupiter` | same optional swap controls |
| Custom transaction | `wallet.buildTransaction` | `wallet.build_transaction` | same custom preparation request and instruction shape |
| Policy read | `swig.wallets.getPolicy` | `swig.wallets.get_policy` | same raw policy metadata |
| ParticipantSet creation | `swig.participantSets.create` | `swig.participant_sets.create` | same mixed-member setup and prepared transaction |
| ParticipantSet role | `wallet.roles.add` | `wallet.roles.add` | same typed authority and caller-selected permissions |
| ParticipantSet compilation | `swig.transactions.compileParticipantSetApprovals` | `swig.transactions.compile_participant_set_approvals` | same bound plan, detached approvals, and no submission side effect |

## Wallet reads

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| USD balance | `wallet.getUsdBalance` | `wallet.get_usd_balance` | same required-field validation |
| Token balances | `wallet.listTokenBalances` | `wallet.list_token_balances` | same normalization, totals, and `assetKind` discriminator |
| Token transactions | `wallet.listTokenTransactions` | `wallet.list_token_transactions` | same `limit`, direction, and `assetKind` normalization |
| Roles | `wallet.listRoles` | `wallet.list_roles` | same role, authority, and action normalization |

## Paymaster and submission

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| Paymaster balance | `swig.paymaster.getBalance` | `swig.paymaster.get_balance` | same query and normalization |
| IDP paymaster balance | `swig.paymaster.getIdpBalance` | `swig.paymaster.get_idp_balance` | same `kind` mapping |
| Sponsorship | `swig.transactions.sponsor` | `swig.transactions.sponsor` | same base58 conversion and idempotency-key forwarding |
| Bundle sponsorship | `swig.transactions.sponsorBundle` | `swig.transactions.sponsor_bundle` | same mainnet-only and 1–5 transaction validation |

Sponsor responses report submission acceptance, not confirmation or finality.

## Ramp

Both SDKs expose direction-specific clients. There is no generic ramp client.

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| Onramp options | `swig.ramp.onramp.getOptions` | `swig.ramp.onramp.get_options` | same query and normalization |
| Onramp quote | `swig.ramp.onramp.quote` | `swig.ramp.onramp.quote` | same required fields and MELD environment encoding |
| Onramp session | `swig.ramp.onramp.createSession`, `getSession` | `create_session`, `get_session` | same session id, launch URL, and status enum mapping |
| Offramp options | `swig.ramp.offramp.getOptions` | `swig.ramp.offramp.get_options` | same crypto-currency normalization |
| Offramp quote | `swig.ramp.offramp.quote` | `swig.ramp.offramp.quote` | same required fields and MELD environment encoding |
| Offramp session | `swig.ramp.offramp.createSession`, `getSession` | `create_session`, `get_session` | same session fields and status enum mapping |
| Offramp authorization | `swig.ramp.offramp.prepareAuthorization`, `submitAuthorization` | `prepare_authorization`, `submit_authorization` | same prepared transfer, display fields, and Solana signature |

## Signing

TypeScript uses `@swig-wallet/developer-sdk/signers`. Python uses the
`swig_developer_sdk.signers` module. Both signer surfaces accept
application-owned callbacks and make no hosted API requests.

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| Generic signing | `signPreparedTransaction`, signer object | `sign_prepared_transaction`, signer protocol | same metadata preservation |
| Swig r1 signing | passkey callback and local transaction patching | WebAuthn callback and `solders` patching | byte-for-byte transaction semantics |
| Swig k1 signing | EIP-1193 callback and local transaction patching | EIP-1193-compatible callback and `solders` patching | byte-for-byte transaction semantics |

## Additional helpers

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| One Business | grant URL, redirect, and callback parsing | same | same query contract and errors |

The TypeScript package does not ship browser proxy, Next.js, NestJS, or Fetch
adapter entrypoints. Applications own their HTTP boundary between the server
SDK and any signer environment.

## Not documented

Recovery types exist in both packages but the hosted recovery flow is not
currently supported. It is intentionally absent from both package READMEs and
from the published documentation.
