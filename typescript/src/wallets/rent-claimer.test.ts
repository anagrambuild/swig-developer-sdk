import { describe, expect, test } from 'bun:test';

import { SwigClient } from '../index.js';

describe('wallet.getRentClaimer', () => {
  test('uses the backend API and the request, wallet, then client network', async () => {
    const calls: Request[] = [];
    const client = new SwigClient({
      apiKey: 'test-key',
      baseUrl: 'https://api.example.test',
      network: 'mainnet',
      fetch: (async (
        input: Parameters<typeof fetch>[0],
        init: Parameters<typeof fetch>[1],
      ) => {
        calls.push(new Request(input, init));
        return Response.json({ rentClaimer: 'rent-vault' });
      }) as unknown as typeof fetch,
    });
    const wallet = client.wallets.use('config/address', { network: 'devnet' });
    expect(await wallet.getRentClaimer()).toBe('rent-vault');
    expect(await wallet.getRentClaimer({ network: 'mainnet' })).toBe(
      'rent-vault',
    );
    expect(await client.wallets.use('config').getRentClaimer()).toBe(
      'rent-vault',
    );
    expect(calls.map((request) => request.url)).toEqual([
      'https://api.example.test/transaction/wallet/config%2Faddress/rent-claimer?network=NETWORK_DEVNET',
      'https://api.example.test/transaction/wallet/config%2Faddress/rent-claimer?network=NETWORK_MAINNET',
      'https://api.example.test/transaction/wallet/config/rent-claimer?network=NETWORK_MAINNET',
    ]);
    expect(calls.every((request) => request.method === 'GET')).toBe(true);
    expect(
      calls.every(
        (request) => request.headers.get('authorization') === 'Bearer test-key',
      ),
    ).toBe(true);
  });

  test('normalizes absent, null and snake-case proto responses', async () => {
    for (const [response, expected] of [
      [{}, null],
      [{ rentClaimer: null }, null],
      [{ rent_claimer: 'rent-vault' }, 'rent-vault'],
    ] as const) {
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        fetch: (async () => Response.json(response)) as unknown as typeof fetch,
      });
      expect(await client.wallets.use('config').getRentClaimer()).toBe(
        expected,
      );
    }
  });

  test('propagates API failures instead of reporting an unset claimer', async () => {
    for (const status of [400, 401, 404, 503]) {
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        retryOptions: { maxRetries: 0 },
        fetch: (async () =>
          Response.json(
            { message: 'cannot inspect config' },
            { status },
          )) as unknown as typeof fetch,
      });
      await expect(
        client.wallets.use('config').getRentClaimer(),
      ).rejects.toMatchObject({ statusCode: status });
    }
  });

  test('requires a network and rejects malformed response values', async () => {
    const missingNetwork = new SwigClient({ apiKey: 'test-key' }).wallets.use(
      'config',
    );
    await expect(missingNetwork.getRentClaimer()).rejects.toThrow(
      'network is required',
    );
    for (const response of [
      null,
      'not-an-object',
      { rentClaimer: 42 },
      { rentClaimer: '' },
    ]) {
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        fetch: (async () => Response.json(response)) as unknown as typeof fetch,
      });
      await expect(
        client.wallets.use('config').getRentClaimer(),
      ).rejects.toThrow('Invalid rent-claimer response');
    }
  });
});
