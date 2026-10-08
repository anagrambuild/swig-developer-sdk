# Python SDK review and follow-up checklist

Review date: 2026-10-08.
Swig baseline: `c8885f0a88b67c113ef96cfd3cc74b04ff49813c` (`origin/main`, Python 0.9.0).
This file records the baseline review, research, and Python 0.10.0 implementation.

## Research and reassessment

- [x] Review Swig's public interface, client lifecycle, errors, and packaging.
- [x] Run the existing checks and verify the installed-wheel typing boundary.
- [x] Inspect current OpenAI, Stripe, Boto3, and Google Cloud Storage source.
- [x] Record call-interface patterns with revision-pinned evidence.
- [x] Reassess the initial findings and distinguish defects from design preferences.
- [x] Specify a proposed Swig call interface and actionable follow-ups.

## Revised assessment

Swig already has a conventional Python resource API. Keep its client hierarchy,
wallet handles, keyword arguments, dataclass results, and application-owned signing
boundary. The most important improvements are HTTP lifecycle, distributed typing,
and error handling. Refine the call contract with precise authority inputs,
consistent sponsorship arguments, configurable timeouts, and useful docstrings.

Corrections to the initial review:

- **Keep resource namespaces.** `wallet.transfer.sol(...)`, `wallet.roles.add(...)`,
  and `swig.ramp.get_quotes(...)` fit established SDK designs. Resource handles
  usefully bind identity and defaults. The repository's [parity contract][swig-parity]
  also intentionally preserves the client hierarchy across languages.
- **Keep dictionaries as a supported request shape.** OpenAI and Stripe both use
  `TypedDict` inputs. Replace the loose authority type with precise variants;
  mandatory authority dataclass construction is unnecessary.
- **Treat sponsorship's Args wrapper as an internal consistency issue.** Stripe's
  parameter-object interface shows that such objects are a legitimate Python SDK
  choice. Keywords are preferable here because Swig already uses them for most
  operations and documents that convention in its parity contract.
- **Retain compatibility aliases.** The evidence does not justify removing
  callable transfer/swap shortcuts, `spl_token`, or `SwigServerClient` solely for
  style. Select canonical examples and assess real usage before deprecating any.

Simplicity assessment: the existing resource and dataclass design is sufficient.
Use focused changes at existing boundaries. A new SDK framework, wholesale model
conversion, or flattened resource API would add migration work without a demonstrated
consumer benefit.

## Call-interface patterns from the source

These are source snapshots of the repositories' default branches fetched on the
review date, rather than claims about whichever version a consumer has installed.

| SDK and inspected revision | Representative call shape | Relevant pattern |
| --- | --- | --- |
| OpenAI `8e1fd2587deae364367e791385997ab45aa2a520` | `await client.files.create(file=data, purpose="user_data")` | Resource methods with explicit keyword-only payloads; identifiers may be positional for retrieval; nested inputs use `TypedDict`; responses use models. Separate sync and async clients retain the same method names. [Methods][openai-files], [input type][openai-input]. |
| Stripe `29363094b09a67f8b1b50f577b463edbbb27ec9a` | `await client.v1.customers.create_async(params={"email": email})` | Resource namespaces with typed parameter dictionaries and separate request options. Retrieval takes a positional identifier. Async methods use an `_async` suffix on the same client. [Methods][stripe-customer], [input type][stripe-input], [options][stripe-options]. |
| Boto3 `9b19947c3cfcef75509aa9cf9c95447d8d732282` | `client.send_message(QueueUrl=url, MessageBody=body)` | Generated operation methods require keyword arguments, preserve AWS parameter casing, and return dictionaries. [Client contract][boto-clients], [session implementation][boto-session]. |
| Google Cloud Storage `fc6eae3bbcea47d00f190d4bab51bef80e838a3a` | `client.bucket(name).blob(key).upload_from_string(data, content_type=mime)` | Bound resource handles own identity; methods take ordinary arguments and operational keywords such as timeout/retry. Creating a bucket handle makes no request. [Client][gcs-client], [blob methods][gcs-blob]. |

Google Cloud Storage now lives in `googleapis/google-cloud-python`; the former
`googleapis/python-storage` repository points there. Boto3's resource interface is
maintained but is not receiving new features, so its client API is the stronger
reference for new operations. [Boto3 resource status][boto-resources].

The useful patterns, and how they apply to Swig:

1. **Use named resource operations and bind repeated identity once.** Swig's
   `wallets.use(address, ...)` and resource methods already satisfy this. As with
   GCS's bucket factory, document that creating a handle makes no API request.
2. **Make payload names discoverable.** Explicit keywords and typed parameter
   dictionaries are both established. Keep Swig's ordinary operation arguments
   keyword-only. Keep nested authority dictionaries but give their fields concrete
   types. Existing operation/action dataclasses remain useful domain values.
3. **Separate transport policy from business data.** OpenAI supports client and
   per-call timeout controls; Stripe separates `params` from `options`; GCS methods
   accept timeout and retry settings. Add a client-level timeout to Swig first.
   Preserve endpoint-specific replay rules when exposing any controls.
4. **Keep defaults and call effects visible.** Explain network precedence, units,
   local handle creation, preparation versus submission, and acceptance versus
   confirmation directly on the call. Compare [OpenAI method docs][openai-files]
   and [GCS handle/operation docs][gcs-client].
5. **Give long-lived clients an explicit end of life.** OpenAI's async client has
   a context manager and close method; GCS's client has the synchronous equivalents.
   Swig should reuse one HTTP client for the lifetime of `SwigClient`.
   [OpenAI lifecycle][openai-lifecycle], [GCS lifecycle][gcs-lifecycle].
6. **Expose recognizable errors and preserve their causes.** OpenAI distinguishes
   connection, timeout, status, and response-validation errors; Stripe distinguishes
   connection/API failures and chains transport errors. Swig needs consistent
   classification, without copying every class from either library.
   [OpenAI errors][openai-errors], [OpenAI chaining][openai-chaining],
   [Stripe errors][stripe-errors], [Stripe transport][stripe-http].

Patterns to defer until a Swig requirement earns them:

- Sync support: the examples differ substantially. Keep the current async API;
  revisit a separate sync client only when actual consumers require it.
- Pagination: OpenAI, Stripe, Boto3, and GCS expose iterators for APIs that support
  paging. Swig's reviewed list results expose no continuation token; a `limit`
  argument alone does not justify automatic pagination. [OpenAI lists][openai-lists],
  [Stripe paging][stripe-paging], [Boto3 paging][boto-paging].
- Omission sentinels and `with_options` client copies: add them only for a concrete
  omitted-versus-null or per-request override contract. Keep today's simpler
  configuration until then.

## Implementation checklist

P1 means a confirmed lifecycle or developer-contract defect to fix first.
P2 means a concrete interface improvement. All eight items are implemented below.

- [x] **PYSDK-01 — P1: Reuse the HTTP client and manage its lifetime.**
  In [core.py][swig-http] and [client.py][swig-client], keep one async HTTP client
  per `SwigClient`, shared by resource handles. Add `__aenter__`, `__aexit__`, and
  `aclose`; define transport ownership and behavior after close. The current
  implementation constructs and closes a client for every attempt.
  Acceptance: two requests and a retry reuse the client; the injected transport
  stays open between calls and closes at the documented lifecycle boundary;
  context exit also closes after an exception; repeated close is safe.

- [x] **PYSDK-02 — P1: Distribute and check package typing.**
  Add `src/swig_developer_sdk/py.typed` and include it in both distributions.
  The [current build configuration][swig-package] produces a wheel without it.
  Acceptance: an external consumer checks an installed wheel without the repo's
  `mypy_path`; `SwigClient` has its real type and an invalid method call is rejected.
  See the [typing distribution specification][typing-distribution].

- [x] **PYSDK-03 — P1: Preserve actionable error categories and causes.**
  Narrow the [transport exception boundary][swig-errors] and chain the original
  exception. Keep a common SDK base error; distinguish network/timeout, API status,
  and malformed server responses. Preserve `ValueError`/`TypeError` for invalid
  caller input and let unexpected programming errors propagate.
  Acceptance: a programming `TypeError` is not retried as a network failure;
  transport failures preserve their cause; malformed success responses are
  catchable through the SDK base error; existing safe POST retry rules still hold.

- [x] **PYSDK-04 — P2: Give authority dictionaries precise input types.**
  Replace [WalletAuthority's generic nested Mapping][swig-authority] with concrete
  `TypedDict` variants and an explicit union. Use snake_case in canonical Python
  examples, retaining wire conversion internally and compatibility for currently
  accepted camelCase inputs. Keep runtime validation: static typing does not
  validate external dictionaries or enforce authorization.
  Acceptance: type checks catch missing/misspelled keys and wrong value types;
  valid existing authority forms serialize to unchanged API payloads; unsupported
  and ambiguous authority variants fail explicitly before a request is sent.

- [x] **PYSDK-05 — P2: Make sponsorship follow Swig's keyword convention.**
  Add canonical keyword forms to [sponsor and sponsor_bundle][swig-sponsor], with
  transaction(s), encoding where applicable, network, and idempotency key visible
  in their signatures. Keep existing `SponsorSignedTransactionArgs` and bundle
  arguments working during a documented migration; reject mixed calling forms.
  Acceptance: old and new calls produce the same wire payload and retry decision;
  required parameters are discoverable; bundle count/network restrictions remain.
  This is consistency work, not evidence that request objects are un-Pythonic.

- [x] **PYSDK-06 — P2: Expose an HTTP timeout at client construction.**
  [SwigClient][swig-client] currently exposes neither a timeout nor a configured
  HTTP-client parameter, so calls inherit HTTPX defaults. Add a documented
  `timeout` argument, including its units and disable behavior, and pass it to the
  reused client from PYSDK-01. Start with client-wide configuration; per-call
  overrides can follow when consumers need them.
  Acceptance: mock requests observe the configured timeout; timeout failures
  remain distinguishable through PYSDK-03; the chosen default is documented.

- [x] **PYSDK-07 — P2: Document supported public calls in code.**
  The reviewed resource methods lack docstrings. Cover supported wallet, transfer,
  sponsorship, ramp, and signing entry points with argument units, default
  precedence, return type/meaning, and expected errors. State that transfers prepare
  transactions and sponsorship acceptance is not confirmation. Keep the engineering
  contract's unsupported recovery flow unpromoted.
  Acceptance: `help(wallet.transfer.sol)`, `help(swig.wallets.use)`, and
  `help(swig.transactions.sponsor)` explain these effects without requiring a
  README search; examples match the actual supported signatures.

- [x] **PYSDK-08 — P2: Establish canonical calls while preserving aliases.**
  Prefer `.transfer.sol(...)`, `.transfer.token(...)`, `.swap.jupiter(...)`, and
  `.roles.list(...)` in examples. Keep [callable shortcuts and token aliases][swig-transfer]
  working. Explain their equivalence rather than flattening the resource model.
  Any later deprecation should be based on usage and an explicit compatibility plan.
  Acceptance: canonical examples are consistent; aliases preserve their outputs
  and exceptions; the documented cross-language hierarchy remains intact.

Suggested order: PYSDK-01/02/03 first, PYSDK-06 with lifecycle work, then PYSDK-04/05,
and finally PYSDK-07/08 once the canonical signatures are settled.

## Implemented call interface (0.10.0)

Context management, `timeout`, and keyword sponsorship are available in 0.10.0.
The resource hierarchy,
snake_case authority serialization, preparation, signing callback, and result
attributes follow existing behavior. The example's application supplies a signer
that signs all required Ed25519 transaction signatures; signing remains outside
the hosted API.

```python
from swig_developer_sdk import SubmittedTransaction, SwigClient
from swig_developer_sdk.signers import (
    PreparedTransactionSigningFn,
    sign_prepared_transaction,
)


async def prepare_and_sponsor(
    *,
    api_key: str,
    swig_address: str,
    public_key: str,
    fee_payer: str,
    destination: str,
    amount_lamports: int,
    idempotency_key: str,
    sign_transaction: PreparedTransactionSigningFn,
) -> SubmittedTransaction:
    async with SwigClient(
        api_key=api_key,
        network="devnet",
        timeout=30.0,
    ) as swig:
        wallet = swig.wallets.use(
            swig_address,
            requester_authority={"ed25519": {"public_key": public_key}},
        )
        prepared = await wallet.transfer.sol(
            fee_payer=fee_payer,
            destination=destination,
            amount=amount_lamports,
        )
        signed = await sign_prepared_transaction(
            prepared,
            sign_transaction=sign_transaction,
        )
        return await swig.transactions.sponsor(
            transaction=signed.transaction,
            network=signed.network,
            idempotency_key=idempotency_key,
        )
```

## Validation evidence

The first review at this exact baseline on Python 3.10 produced:

- 67 tests passed; Ruff formatting, Ruff lint, strict mypy, and wheel/source
  distribution builds passed.
- The built wheel contained no `py.typed`. A separate consumer environment's mypy
  check reported `import-untyped`, inferred `SwigClient` as `Any`, and could not
  detect a nonexistent method on that value.
- A lifecycle-aware mock transport closed after the first successful request and
  rejected the second request. This proves ownership/lifecycle behavior; no real
  network performance benchmark was performed.
- A mock transport's `TypeError` was attempted twice and surfaced as
  `NETWORK_ERROR` with both `__cause__` and `__context__` unset.
- A malformed successful transfer response raised plain `ValueError`, outside
  `SwigDeveloperSdkError`.

This follow-up refreshed `origin/main` (unchanged), inspected the four pinned SDK
snapshots and Swig's engineering/parity contracts, and reassessed the call
signatures. Production code did not change, so the earlier check results still
apply to the reviewed baseline. The proposed example was syntax-checked only;
its unimplemented additions cannot yet be runtime-validated.

### Python 0.10.0 implementation receipt

- PYSDK-01/06: one persistent HTTP client, context management, idempotent close,
  timeout validation, and documented transport ownership. Proxy handlers reuse a
  lazy client without retaining a request-specific network and close even unused
  injected transports. Local example clients now have explicit lifetimes.
- PYSDK-02/04: wheel and source distributions include `py.typed`; explicit authority
  variants accept canonical snake_case and existing camelCase forms. Runtime
  validation rejects ambiguous variants and conflicting aliases.
- PYSDK-03: connection, timeout, and response errors share the SDK base. Transport
  causes survive; cancellation/programming errors propagate. Response-validation
  errors retain ValueError compatibility. Raw bodies are no longer fallback details.
- PYSDK-05/08: keyword sponsorship and legacy Args forms produce identical wire
  payloads. Existing resource namespaces, callable shortcuts, and aliases remain.
- PYSDK-07: supported hosted operations and local signing entry points now describe
  effects, units, defaults, ownership, and errors. Recovery support is unchanged.
- Validation on Python 3.10: 109 tests passed, including compiler-backed consumer
  positive/negative cases; Ruff format/lint and strict mypy passed; locked dependency
  verification and source/wheel builds passed. A separate environment installed the
  built wheel, passed strict typing with Any expressions forbidden, and rejected
  an invalid authority key without repository mypy configuration.
- These are hermetic tests, package/consumer checks, and source review. The live
  local-chain E2E script was typechecked but not executed. No network performance
  benchmark was performed. Independent PR review and release verification follow
  through the repository publication workflow.

[swig-parity]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/PARITY.md
[swig-http]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/src/swig_developer_sdk/core.py#L90
[swig-client]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/src/swig_developer_sdk/client.py#L14
[swig-package]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/pyproject.toml#L32
[swig-errors]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/src/swig_developer_sdk/core.py#L119
[swig-authority]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/src/swig_developer_sdk/common.py#L12
[swig-sponsor]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/src/swig_developer_sdk/transactions.py#L161
[swig-transfer]: https://github.com/anagrambuild/swig-developer-sdk/blob/c8885f0a88b67c113ef96cfd3cc74b04ff49813c/python/src/swig_developer_sdk/wallets.py#L1012
[openai-files]: https://github.com/openai/openai-python/blob/8e1fd2587deae364367e791385997ab45aa2a520/src/openai/resources/files.py#L404
[openai-input]: https://github.com/openai/openai-python/blob/8e1fd2587deae364367e791385997ab45aa2a520/src/openai/types/file_create_params.py#L13
[openai-lifecycle]: https://github.com/openai/openai-python/blob/8e1fd2587deae364367e791385997ab45aa2a520/src/openai/_base_client.py#L1647
[openai-errors]: https://github.com/openai/openai-python/blob/8e1fd2587deae364367e791385997ab45aa2a520/src/openai/_exceptions.py#L84
[openai-chaining]: https://github.com/openai/openai-python/blob/8e1fd2587deae364367e791385997ab45aa2a520/src/openai/_base_client.py#L1778
[openai-lists]: https://github.com/openai/openai-python/blob/8e1fd2587deae364367e791385997ab45aa2a520/src/openai/resources/files.py#L535
[stripe-customer]: https://github.com/stripe/stripe-python/blob/29363094b09a67f8b1b50f577b463edbbb27ec9a/stripe/_customer_service.py#L327
[stripe-input]: https://github.com/stripe/stripe-python/blob/29363094b09a67f8b1b50f577b463edbbb27ec9a/stripe/params/_customer_create_params.py#L9
[stripe-options]: https://github.com/stripe/stripe-python/blob/29363094b09a67f8b1b50f577b463edbbb27ec9a/stripe/_request_options.py#L9
[stripe-errors]: https://github.com/stripe/stripe-python/blob/29363094b09a67f8b1b50f577b463edbbb27ec9a/stripe/_error.py#L95
[stripe-http]: https://github.com/stripe/stripe-python/blob/29363094b09a67f8b1b50f577b463edbbb27ec9a/stripe/_http_client.py#L691
[stripe-paging]: https://github.com/stripe/stripe-python/blob/29363094b09a67f8b1b50f577b463edbbb27ec9a/stripe/_list_object.py#L119
[boto-clients]: https://github.com/boto/boto3/blob/9b19947c3cfcef75509aa9cf9c95447d8d732282/docs/source/guide/clients.rst
[boto-session]: https://github.com/boto/boto3/blob/9b19947c3cfcef75509aa9cf9c95447d8d732282/boto3/session.py#L235
[boto-resources]: https://github.com/boto/boto3/blob/9b19947c3cfcef75509aa9cf9c95447d8d732282/docs/source/guide/resources.rst
[boto-paging]: https://github.com/boto/boto3/blob/9b19947c3cfcef75509aa9cf9c95447d8d732282/docs/source/guide/paginators.rst
[gcs-client]: https://github.com/googleapis/google-cloud-python/blob/fc6eae3bbcea47d00f190d4bab51bef80e838a3a/packages/google-cloud-storage/google/cloud/storage/client.py#L436
[gcs-lifecycle]: https://github.com/googleapis/google-cloud-python/blob/fc6eae3bbcea47d00f190d4bab51bef80e838a3a/packages/google-cloud-storage/google/cloud/storage/client.py#L297
[gcs-blob]: https://github.com/googleapis/google-cloud-python/blob/fc6eae3bbcea47d00f190d4bab51bef80e838a3a/packages/google-cloud-storage/google/cloud/storage/blob.py#L3235
[typing-distribution]: https://typing.python.org/en/latest/spec/distributing.html#packaging-type-information
