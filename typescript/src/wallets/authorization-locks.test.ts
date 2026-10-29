import { describe, expect, test } from 'bun:test';

import { SwigClient, type AuthorizationLocksResult } from '../index.js';

const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

const response = {
  locks: [
    {
      lockId: 0,
      roleId: 1,
      assetAccount: 'swig-wallet',
      amount: '18446744073709551615',
      expirySlot: '500',
      active: true,
    },
    {
      lockId: 1,
      roleId: 2,
      assetAccount: 'usdc-ata',
      mint: USDC,
      amount: '25',
      expirySlot: '90',
      active: false,
    },
  ],
  currentSlot: '100',
  nextLockId: 2,
};

describe('wallet.getAuthorizationLocks', () => {
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
        return Response.json(response);
      }) as unknown as typeof fetch,
    });
    const wallet = client.wallets.use('config/address', { network: 'devnet' });
    await wallet.getAuthorizationLocks();
    await wallet.getAuthorizationLocks({
      network: 'mainnet',
      includeExpired: true,
    });
    await client.wallets
      .use('config')
      .getAuthorizationLocks({ includeExpired: false });
    expect(calls.map((request) => request.url)).toEqual([
      'https://api.example.test/transaction/wallet/config%2Faddress/authorization-locks?network=NETWORK_DEVNET',
      'https://api.example.test/transaction/wallet/config%2Faddress/authorization-locks?network=NETWORK_MAINNET&include_expired=true',
      'https://api.example.test/transaction/wallet/config/authorization-locks?network=NETWORK_MAINNET&include_expired=false',
    ]);
    expect(calls.every((request) => request.method === 'GET')).toBe(true);
    expect(
      calls.every(
        (request) => request.headers.get('authorization') === 'Bearer test-key',
      ),
    ).toBe(true);
  });

  test('normalizes SOL and token locks with uint64 values as strings', async () => {
    const client = new SwigClient({
      apiKey: 'test-key',
      network: 'mainnet',
      fetch: (async () => Response.json(response)) as unknown as typeof fetch,
    });
    expect(await client.wallets.use('config').getAuthorizationLocks()).toEqual({
      locks: [
        {
          lockId: 0,
          roleId: 1,
          assetAccount: 'swig-wallet',
          amount: '18446744073709551615',
          expirySlot: '500',
          active: true,
        },
        {
          lockId: 1,
          roleId: 2,
          assetAccount: 'usdc-ata',
          mint: USDC,
          amount: '25',
          expirySlot: '90',
          active: false,
        },
      ],
      currentSlot: '100',
      nextLockId: 2,
    });
  });

  test('normalizes absent locks and snake-case proto responses', async () => {
    for (const [wire, expected] of <Array<[unknown, AuthorizationLocksResult]>>[
      [
        { currentSlot: '7', nextLockId: 0 },
        { locks: [], currentSlot: '7', nextLockId: 0 },
      ],
      [
        {
          locks: [
            {
              lock_id: 3,
              role_id: 0,
              asset_account: 'swig-wallet',
              amount: 10,
              expiry_slot: 20,
              active: true,
            },
          ],
          current_slot: 15,
          next_lock_id: 4,
        },
        {
          locks: [
            {
              lockId: 3,
              roleId: 0,
              assetAccount: 'swig-wallet',
              amount: '10',
              expirySlot: '20',
              active: true,
            },
          ],
          currentSlot: '15',
          nextLockId: 4,
        },
      ],
    ]) {
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        fetch: (async () => Response.json(wire)) as unknown as typeof fetch,
      });
      expect(
        await client.wallets.use('config').getAuthorizationLocks(),
      ).toEqual(expected);
    }
  });

  test('propagates API failures instead of reporting no locks', async () => {
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
        client.wallets.use('config').getAuthorizationLocks(),
      ).rejects.toMatchObject({ statusCode: status });
    }
  });

  test('requires a network and rejects malformed response values', async () => {
    const missingNetwork = new SwigClient({ apiKey: 'test-key' }).wallets.use(
      'config',
    );
    await expect(missingNetwork.getAuthorizationLocks()).rejects.toThrow(
      'network is required',
    );
    const lock = response.locks[1];
    for (const wire of [
      null,
      'not-an-object',
      [],
      { ...response, locks: 'not-a-list' },
      { ...response, locks: [null] },
      { ...response, currentSlot: undefined },
      { ...response, currentSlot: '-1' },
      { ...response, nextLockId: undefined },
      { ...response, locks: [{ ...lock, amount: '1.5' }] },
      { ...response, locks: [{ ...lock, expirySlot: undefined }] },
      { ...response, locks: [{ ...lock, active: 'yes' }] },
      { ...response, locks: [{ ...lock, mint: '' }] },
      { ...response, locks: [{ ...lock, assetAccount: undefined }] },
    ]) {
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        fetch: (async () => Response.json(wire)) as unknown as typeof fetch,
      });
      await expect(
        client.wallets.use('config').getAuthorizationLocks(),
      ).rejects.toThrow(
        /Invalid authorization-lock response|Response is missing/,
      );
    }
  });
});
