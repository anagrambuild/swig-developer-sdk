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

function client(handler: (request: CapturedRequest) => unknown) {
  return new SwigClient({
    apiKey: 'sk_test',
    baseUrl: 'http://localhost:8080',
    network: 'devnet',
    fetch: jsonFetch(handler),
  });
}

const BUY_QUOTE = {
  route: { provider: 'PROVIDER', paymentMethod: 'CARD' },
  buy: {
    spend: { currencyCode: 'USD', minorUnits: '10000' },
    receive: { asset: { token: { mint: 'MINT' } }, baseUnits: '99000000' },
    totalFee: { currencyCode: 'USD', minorUnits: '250' },
    exchangeRate: '0.0000099',
  },
};

const SELL_QUOTE = {
  route: { provider: 'PROVIDER', paymentMethod: 'BANK' },
  sell: {
    sell: { asset: { sol: {} }, baseUnits: '1000000000' },
    receive: { currencyCode: 'USD', minorUnits: '15000' },
    totalFee: { currencyCode: 'USD', minorUnits: '300' },
    exchangeRate: '150.00',
  },
};

describe('RampClient', () => {
  test('rejects an invalid environment at runtime', async () => {
    const swig = client(() => ({}));

    await expect(
      swig.ramp.getOptions({
        configurationId: '018f-config',
        environment: 'staging' as never,
        direction: 'buy',
      }),
    ).rejects.toThrow('environment must be "sandbox" or "production"');
  });

  test('builds the options query and decodes the four lists', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return {
        countries: [
          {
            code: 'US',
            name: 'United States',
            subdivisions: [{ code: 'US-CA', name: 'California' }],
          },
        ],
        fiatCurrencies: [{ currencyCode: 'USD', exponent: 2 }],
        paymentMethods: ['CARD'],
        assets: [
          {
            asset: { token: { mint: 'MINT' } },
            name: 'USD Coin',
            decimals: 6,
            iconUrl: 'https://example.test/usdc.png',
          },
          { asset: { sol: {} }, name: 'Solana', decimals: 9 },
        ],
      };
    });

    const options = await swig.ramp.getOptions({
      configurationId: '018f-config',
      environment: 'sandbox',
      direction: 'buy',
      countryCode: 'US',
    });

    expect(calls[0]?.url).toContain(
      '/wallet/api/ramp/options?configurationId=018f-config&environment=RAMP_ENVIRONMENT_SANDBOX&direction=RAMP_DIRECTION_BUY&countryCode=US',
    );
    expect(options.countries[0]?.subdivisions[0]?.code).toBe('US-CA');
    expect(options.fiatCurrencies[0]?.exponent).toBe(2);
    expect(options.paymentMethods).toEqual(['CARD']);
    expect(options.assets[0]?.asset).toEqual({ type: 'token', mint: 'MINT' });
    expect(options.assets[0]?.decimals).toBe(6);
    expect(options.assets[1]?.asset).toEqual({ type: 'sol' });
    expect(options.assets[1]?.iconUrl).toBeUndefined();
  });

  test('sends a buy order oneof without a direction flag', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return { quotes: [BUY_QUOTE] };
    });

    const quotes = await swig.ramp.getQuotes({
      configurationId: '018f-config',
      environment: 'sandbox',
      location: { countryCode: 'US' },
      order: {
        type: 'buy',
        spend: { currencyCode: 'USD', minorUnits: 10000 },
        receive: { type: 'token', mint: 'MINT' },
      },
    });

    expect(calls[0]?.url).toContain('/wallet/api/ramp/quotes');
    expect(calls[0]?.body).toEqual({
      configurationId: '018f-config',
      environment: 'RAMP_ENVIRONMENT_SANDBOX',
      location: { countryCode: 'US' },
      buy: {
        spend: { currencyCode: 'USD', minorUnits: '10000' },
        receive: { token: { mint: 'MINT' } },
      },
    });
    expect(quotes[0]?.type).toBe('buy');
  });

  test('encodes native SOL as an empty message on a sell order', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return { quotes: [SELL_QUOTE] };
    });

    const quotes = await swig.ramp.getQuotes({
      configurationId: '018f-config',
      environment: 'sandbox',
      location: { countryCode: 'US', subdivisionCode: 'US-CA' },
      order: {
        type: 'sell',
        sell: { asset: { type: 'sol' }, baseUnits: 1000000000n },
        receiveFiatCurrencyCode: 'USD',
      },
    });

    expect(calls[0]?.body).toEqual({
      configurationId: '018f-config',
      environment: 'RAMP_ENVIRONMENT_SANDBOX',
      location: { countryCode: 'US', subdivisionCode: 'US-CA' },
      sell: {
        sell: { asset: { sol: {} }, baseUnits: '1000000000' },
        receiveFiatCurrencyCode: 'USD',
      },
    });
    const quote = quotes[0];
    expect(quote?.type).toBe('sell');
    if (quote?.type === 'sell') {
      expect(quote.sell.asset).toEqual({ type: 'sol' });
    }
  });

  test('decodes uint64 amounts above 2^53 without loss', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      if (request.url.includes('/quotes')) {
        return {
          quotes: [
            {
              ...BUY_QUOTE,
              buy: {
                ...BUY_QUOTE.buy,
                receive: {
                  asset: { token: { mint: 'MINT' } },
                  baseUnits: '18446744073709551615',
                },
              },
            },
          ],
        };
      }
      return { order: buyOrder('RAMP_ORDER_STATUS_AWAITING_CUSTOMER') };
    });

    const quotes = await swig.ramp.getQuotes({
      configurationId: '018f-config',
      environment: 'sandbox',
      location: { countryCode: 'US' },
      order: {
        type: 'buy',
        spend: { currencyCode: 'USD', minorUnits: '10000' },
        receive: { type: 'token', mint: 'MINT' },
      },
    });

    const quote = quotes[0];
    if (quote?.type !== 'buy') throw new Error('expected a buy quote');
    expect(quote.receive.baseUnits).toBe('18446744073709551615');

    await swig.ramp.createOrder({
      requestId: 'request-1',
      configurationId: '018f-config',
      environment: 'sandbox',
      context: {
        customerId: 'customer-1',
        swigConfigAddress: 'swig-config',
        location: { countryCode: 'US' },
      },
      route: quote.route,
      order: {
        type: 'sell',
        sell: { asset: { type: 'sol' }, baseUnits: quote.receive.baseUnits },
        receiveFiatCurrencyCode: 'USD',
      },
    });

    const body = calls[1]?.body as { sell: { sell: { baseUnits: string } } };
    expect(body.sell.sell.baseUnits).toBe('18446744073709551615');
  });

  test('rejects a uint64 wire value that is not a decimal string', async () => {
    const swig = client(() => ({
      quotes: [
        {
          ...BUY_QUOTE,
          buy: {
            ...BUY_QUOTE.buy,
            spend: { currencyCode: 'USD', minorUnits: 10000 },
          },
        },
      ],
    }));

    await expect(
      swig.ramp.getQuotes({
        configurationId: '018f-config',
        environment: 'sandbox',
        location: { countryCode: 'US' },
        order: {
          type: 'buy',
          spend: { currencyCode: 'USD', minorUnits: '10000' },
          receive: { type: 'token', mint: 'MINT' },
        },
      }),
    ).rejects.toThrow('Ramp response has invalid minorUnits');
  });

  test('unwraps the order envelope and forwards the request id', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return { order: buyOrder('RAMP_ORDER_STATUS_AWAITING_CUSTOMER') };
    });

    const order = await swig.ramp.createOrder({
      requestId: 'request-1',
      configurationId: '018f-config',
      environment: 'sandbox',
      context: {
        customerId: 'customer-1',
        swigConfigAddress: 'swig-config',
        location: { countryCode: 'US' },
      },
      route: { provider: 'PROVIDER', paymentMethod: 'CARD' },
      order: {
        type: 'buy',
        spend: { currencyCode: 'USD', minorUnits: '10000' },
        receive: { type: 'token', mint: 'MINT' },
      },
    });

    expect(order.type).toBe('buy');
    expect(order.status).toBe('awaiting-customer');
    expect(order.launchUrl).toBe('https://provider.test/session');
    const body = calls[0]?.body as { requestId: string; context: unknown };
    expect(body.requestId).toBe('request-1');
    expect(body.context).toEqual({
      customerId: 'customer-1',
      swigConfigAddress: 'swig-config',
      network: 'NETWORK_DEVNET',
      location: { countryCode: 'US' },
    });
  });

  test('throws when the order envelope is missing', async () => {
    const swig = client(() => ({}));

    await expect(swig.ramp.getOrder({ orderId: 'order-1' })).rejects.toThrow(
      'Ramp response is missing order',
    );
  });

  test('encodes the order id in the path', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return { order: buyOrder('RAMP_ORDER_STATUS_SETTLED') };
    });

    await swig.ramp.getOrder({ orderId: 'order/123' });

    expect(calls[0]?.url).toContain('/wallet/api/ramp/orders/order%2F123');
  });

  test('rejects an unrecognised order status', async () => {
    const swig = client(() => ({
      order: buyOrder('RAMP_ORDER_STATUS_CHARGEBACK'),
    }));

    await expect(swig.ramp.getOrder({ orderId: 'order-1' })).rejects.toThrow(
      'Ramp response has invalid status',
    );
  });

  test('decodes an unspecified status', async () => {
    const swig = client(() => ({
      order: buyOrder('RAMP_ORDER_STATUS_UNSPECIFIED'),
    }));

    const order = await swig.ramp.getOrder({ orderId: 'order-1' });

    expect(order.status).toBe('unspecified');
  });

  test('leaves deposit and transfer absent until the provider assigns them', async () => {
    const swig = client(() => ({
      order: {
        id: 'order-1',
        status: 'RAMP_ORDER_STATUS_AWAITING_CUSTOMER',
        createdAt: '2026-09-01T00:00:00Z',
        updatedAt: '2026-09-01T00:00:00Z',
        sell: { quote: SELL_QUOTE.sell, launchUrl: 'https://provider.test/s' },
      },
    }));

    const order = await swig.ramp.getOrder({ orderId: 'order-1' });

    if (order.type !== 'sell') throw new Error('expected a sell order');
    expect(order.deposit).toBeUndefined();
    expect(order.transfer).toBeUndefined();
  });

  test('decodes the prepared transfer envelope', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return {
        preparedTransfer: {
          transfer: {
            transferId: 'transfer-1',
            state: 'TRANSFER_STATE_PREPARED',
            expiresAt: '2026-09-01T00:01:00Z',
          },
          preparedTransaction: {
            transaction: 'prepared-base64',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            network: 'NETWORK_DEVNET',
          },
          deposit: {
            address: 'deposit-address',
            amount: { asset: { sol: {} }, baseUnits: '1000000000' },
          },
        },
      };
    });

    const prepared = await swig.ramp.prepareTransfer({
      orderId: 'order-1',
      requesterAuthority: { ed25519: { publicKey: 'requester' } },
      feePayer: 'payer',
    });

    expect(calls[0]?.url).toContain(
      '/wallet/api/ramp/orders/order-1/transfer/prepare',
    );
    expect(calls[0]?.body).toEqual({
      requesterAuthority: { ed25519: { publicKey: 'requester' } },
      feePayer: 'payer',
    });
    expect(prepared.transfer.state).toBe('prepared');
    expect(prepared.preparedTransaction.transaction).toBe('prepared-base64');
    expect(prepared.deposit.amount.asset).toEqual({ type: 'sol' });
  });

  test('sends an empty signed transaction when none is given', async () => {
    const calls: CapturedRequest[] = [];
    const swig = client((request) => {
      calls.push(request);
      return {
        transfer: {
          transferId: 'transfer-1',
          state: 'TRANSFER_STATE_LANDED',
          expiresAt: '2026-09-01T00:01:00Z',
          solanaSignature: 'signature',
        },
      };
    });

    const transfer = await swig.ramp.submitTransfer({
      orderId: 'order-1',
      transferId: 'transfer-1',
    });

    expect(calls[0]?.body).toEqual({
      transferId: 'transfer-1',
      signedTransaction: '',
    });
    expect(transfer.state).toBe('landed');
    expect(transfer.solanaSignature).toBe('signature');
  });
});

function buyOrder(status: string) {
  return {
    id: 'order-1',
    status,
    createdAt: '2026-09-01T00:00:00Z',
    updatedAt: '2026-09-01T00:00:00Z',
    buy: {
      quote: BUY_QUOTE.buy,
      launchUrl: 'https://provider.test/session',
    },
  };
}
