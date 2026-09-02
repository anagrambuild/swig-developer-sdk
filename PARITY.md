# Developer SDK parity

Python mirrors the TypeScript server SDK by behavior and client hierarchy
except for features explicitly marked as pending. Python uses snake_case names
and keyword arguments; TypeScript uses camelCase and options objects.
TypeScript is version `0.10.0`, Python is version `0.9.0`, and both default to
`https://api.onswig.com`.

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
| ParticipantSet creation | `swig.participantSets.create` | `swig.participant_sets.create` | same standard authority members and prepared transaction |
| General role creation | `wallet.roles.add` | `wallet.roles.add` | same typed authority and closed action set |
| ParticipantSet compilation | `swig.transactions.compileParticipantSetApprovals` | `swig.transactions.compile_participant_set_approvals` | same shared nonce, detached approvals, expiration, and no submission side effect |
| x402 payment preparation | `wallet.x402.prepareFromResponse` | `wallet.x402.prepare_from_response` | same selection, preparation, and payment-header assembly |

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

Both SDKs expose one ramp client covering both directions. Direction is carried
by the buy or sell order, not by the client you call.

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| Options | `swig.ramp.getOptions` | `swig.ramp.get_options` | same query, direction encoding, and four normalized lists |
| Quotes | `swig.ramp.getQuotes` | `swig.ramp.get_quotes` | same order oneof on the wire and same route plus details result |
| Order creation | `swig.ramp.createOrder` | `swig.ramp.create_order` | same `requestId` idempotency key, same retry on a 5xx, same normalized order |
| Order read | `swig.ramp.getOrder` | `swig.ramp.get_order` | same path encoding and same status enum mapping |
| Transfer preparation | `swig.ramp.prepareTransfer` | `swig.ramp.prepare_transfer` | same prepared transaction, transfer, and deposit |
| Transfer submission | `swig.ramp.submitTransfer` | `swig.ramp.submit_transfer` | same empty `signedTransaction` resolution of an already-broadcast attempt |

Amounts are integers in the smallest unit and cross the wire as decimal
strings in both languages, so a value above 2^53 survives. Crypto assets are a
two-case type — native SOL or an SPL mint — encoded as `{"sol": {}}` or
`{"token": {"mint": ...}}` and decoded by key presence in both languages.
Status and transfer-state enums are decoded from their wire names through an
explicit table; an unrecognized value is refused rather than guessed.

## Signing

TypeScript uses `@swig-wallet/developer-sdk/signers`. Python uses the
`swig_developer_sdk.signers` module. Both signer surfaces accept
application-owned callbacks and make no hosted API requests.

| Surface | TypeScript | Python | Parity target |
| --- | --- | --- | --- |
| Generic signing | `signPreparedTransaction`, signer object | `sign_prepared_transaction`, signer protocol | same metadata preservation |
| Swig r1 signing | passkey callback and local transaction patching | WebAuthn callback and `solders` patching | byte-for-byte transaction semantics |
| Swig k1 signing | EIP-1193 callback and local transaction patching | EIP-1193-compatible callback and `solders` patching | byte-for-byte transaction semantics |
| Participant Ed25519 approval | `createParticipantEd25519Signer`, `signParticipantSetApproval` | `create_participant_ed25519_signer`, `sign_participant_set_approval` | decoded 32-byte challenge and exact 64-byte signature |
| Participant passkey approval | `createParticipantPasskeySigner`, `signParticipantSetApproval` | `create_participant_passkey_signer`, `sign_participant_set_approval` | exact challenge and assertion bytes, DER-to-raw low-S P-256 signature |
| Participant personal-sign approval | `createParticipantPersonalSignSigner`, `signParticipantSetApproval` | `create_participant_personal_sign_signer`, `sign_participant_set_approval` | lowercase ASCII challenge, low-S k1 signature, and adjusted recovery byte |

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
