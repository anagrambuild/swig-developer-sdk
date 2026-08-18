import { describe, expect, test } from 'bun:test';

import { SwigClient } from '../server/typescript/index.js';

type CapturedRequest = { url: string; method: string; body?: unknown };

function jsonFetch(
  handler: (request: CapturedRequest) => unknown,
): typeof fetch {
  return (async (input, init) => {
    const request = new Request(input, init);
    const text = await request.text();
    return Response.json(
      handler({
        url: request.url,
        method: request.method,
        ...(text ? { body: JSON.parse(text) } : {}),
      }),
    );
  }) as typeof fetch;
}

describe('RampClient', () => {
  test('rejects an invalid environment at runtime', async () => {
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      fetch: jsonFetch(() => ({})),
    });

    await expect(
      swig.ramp.onramp.getOptions({
        organizationMeldConfigurationId: '018f-config',
        environment: 'staging' as never,
      }),
    ).rejects.toThrow('environment must be "sandbox" or "production"');
  });

  test('uses the onramp options, quote, session, and status contracts', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        if (request.url.includes('/options')) {
          return {
            countries: [
              {
                country_code: 'US',
                country_name: 'United States',
                subdivisions: [
                  {
                    subdivision_code: 'US-CA',
                    subdivision_name: 'California',
                  },
                ],
              },
            ],
            fiat_currency_codes: ['USD'],
            payment_method_types: ['CARD'],
            crypto_currency_codes: ['USDC_SOLANA'],
          };
        }
        if (request.url.endsWith('/quote')) {
          return {
            quotes: [
              {
                quote_id: 'quote_123',
                service_provider: 'TRANSAK',
                payment_method_type: 'CARD',
                source_amount: '100',
                source_currency_code: 'USD',
                destination_amount: '99',
                destination_currency_code: 'USDC_SOLANA',
                exchange_rate: '0.99',
                total_fee: '1',
              },
            ],
          };
        }
        if (request.method === 'POST') {
          return { session_id: 'session_123', launch_url: 'https://launch' };
        }
        return {
          session_id: 'session_123',
          status: 'ONRAMP_SESSION_STATUS_PENDING',
          created_at: '2026-08-18T00:00:00Z',
          updated_at: '2026-08-18T00:01:00Z',
        };
      }),
    });
    const configuration = {
      organizationMeldConfigurationId: '018f-config',
      environment: 'sandbox' as const,
    };

    await expect(
      swig.ramp.onramp.getOptions({
        ...configuration,
        countryCode: 'US',
      }),
    ).resolves.toMatchObject({
      countries: [{ countryCode: 'US' }],
      cryptoCurrencyCodes: ['USDC_SOLANA'],
    });
    await expect(
      swig.ramp.onramp.quote({
        ...configuration,
        externalCustomerId: 'customer_123',
        swigConfigAddress: 'swig_123',
        sourceAmount: '100',
        sourceCurrencyCode: 'USD',
        destinationCurrencyCode: 'USDC_SOLANA',
        countryCode: 'US',
      }),
    ).resolves.toMatchObject({ quotes: [{ serviceProvider: 'TRANSAK' }] });
    await expect(
      swig.ramp.onramp.createSession({
        ...configuration,
        quoteId: 'quote_123',
      }),
    ).resolves.toEqual({
      sessionId: 'session_123',
      launchUrl: 'https://launch',
    });
    await expect(
      swig.ramp.onramp.getSession({
        sessionId: 'session_123',
        environment: 'sandbox',
      }),
    ).resolves.toMatchObject({ status: 'pending' });

    expect(calls[0]?.url).toContain(
      '/wallet/api/ramp/onramp/options?organizationMeldConfigurationId=018f-config&environment=MELD_ENVIRONMENT_SANDBOX&countryCode=US',
    );
    expect(calls[1]).toMatchObject({
      url: 'http://localhost:8080/wallet/api/ramp/onramp/quote',
      body: {
        organizationMeldConfigurationId: '018f-config',
        externalCustomerId: 'customer_123',
        swigConfigAddress: 'swig_123',
        network: 'NETWORK_DEVNET',
        sourceAmount: '100',
        sourceCurrencyCode: 'USD',
        destinationCurrencyCode: 'USDC_SOLANA',
        countryCode: 'US',
        environment: 'MELD_ENVIRONMENT_SANDBOX',
      },
    });
  });

  test('uses the offramp options and authorization contracts', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'mainnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        if (request.url.includes('/options')) {
          return {
            countries: [],
            fiat_currency_codes: ['USD'],
            payment_method_types: ['ACH'],
            crypto_currencies: [
              {
                currency_code: 'USDC_SOLANA',
                currency_name: 'USD Coin',
                icon_url: 'https://icon',
                contract_address: 'mint_123',
              },
            ],
          };
        }
        if (request.url.endsWith('/prepare')) {
          return {
            authorization_id: 'authorization_123',
            prepared_transaction: {
              transaction: 'base64_transaction',
              transaction_encoding: 'TRANSACTION_ENCODING_BASE64',
              signature_requests: [],
            },
            display: {
              source_wallet_address: 'wallet_123',
              destination_wallet_address: 'provider_123',
              source_amount: '10',
              source_currency_code: 'USDC_SOLANA',
              destination_amount: '9.90',
              destination_currency_code: 'USD',
              service_provider: 'TRANSAK',
            },
          };
        }
        if (request.url.endsWith('/submit')) {
          return { solana_signature: 'signature_123' };
        }
        return {
          session_id: 'session_123',
          status: 'OFFRAMP_SESSION_STATUS_TRANSFER_REQUIRED',
          source_amount: '10',
          source_currency_code: 'USDC_SOLANA',
          destination_amount: '9.90',
          destination_currency_code: 'USD',
          service_provider: 'TRANSAK',
          created_at: '2026-08-18T00:00:00Z',
          updated_at: '2026-08-18T00:01:00Z',
        };
      }),
    });
    const configuration = {
      organizationMeldConfigurationId: '018f-config',
      environment: 'production' as const,
    };

    await expect(
      swig.ramp.offramp.getOptions(configuration),
    ).resolves.toMatchObject({
      cryptoCurrencies: [{ contractAddress: 'mint_123' }],
    });
    const prepared = await swig.ramp.offramp.prepareAuthorization({
      sessionId: 'session_123',
      environment: 'production',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
      feePayer: 'payer_123',
    });
    expect(prepared.authorizationId).toBe('authorization_123');
    expect(prepared.preparedTransaction.transaction).toBe('base64_transaction');
    await expect(
      swig.ramp.offramp.submitAuthorization({
        sessionId: 'session_123',
        authorizationId: 'authorization_123',
        signedTransaction: 'base64_signed_transaction',
        environment: 'production',
      }),
    ).resolves.toEqual({ solanaSignature: 'signature_123' });
    await expect(
      swig.ramp.offramp.getSession({
        sessionId: 'session_123',
        environment: 'production',
      }),
    ).resolves.toMatchObject({ status: 'transfer-required' });

    expect(calls[1]).toMatchObject({
      url: 'http://localhost:8080/wallet/api/ramp/offramp/session/session_123/prepare',
      body: {
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
        environment: 'MELD_ENVIRONMENT_PRODUCTION',
        feePayer: 'payer_123',
      },
    });
  });
});
