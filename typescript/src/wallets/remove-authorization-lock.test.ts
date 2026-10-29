import { describe, expect, test } from 'bun:test';

import { SwigClient, type RemoveAuthorizationLockArgs } from '../index.js';

const args: RemoveAuthorizationLockArgs = {
  feePayer: 'sponsor',
  lockId: 3,
};

describe('wallet.removeAuthorizationLock', () => {
  test('asks the backend to prepare an unsigned transaction', async () => {
    const calls: { request: Request; body: unknown }[] = [];
    const client = new SwigClient({
      apiKey: 'test-key',
      baseUrl: 'https://api.example.test',
      network: 'mainnet',
      fetch: (async (
        input: Parameters<typeof fetch>[0],
        init: Parameters<typeof fetch>[1],
      ) => {
        const request = new Request(input, init);
        calls.push({ request, body: await request.json() });
        return Response.json({
          transaction: {
            transaction: 'backend-built-base64',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            network: 'NETWORK_DEVNET',
            recentBlockhash: 'blockhash',
            kind: 'PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK',
            wallet: { swigConfigAddress: 'config', walletAddress: 'wallet' },
          },
        });
      }) as unknown as typeof fetch,
    });
    const wallet = client.wallets.use('config', {
      network: 'devnet',
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
    });
    const prepared = await wallet.removeAuthorizationLock(args);
    expect(calls.length).toBe(1);
    expect(calls[0]?.request.url).toBe(
      'https://api.example.test/transaction/wallet/authorization-lock/remove',
    );
    expect(calls[0]?.request.method).toBe('POST');
    expect(calls[0]?.request.headers.get('authorization')).toBe(
      'Bearer test-key',
    );
    expect(calls[0]?.body).toEqual({
      network: 'NETWORK_DEVNET',
      swigAddress: 'config',
      feePayer: 'sponsor',
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
      lockId: 3,
    });
    expect(prepared).toMatchObject({
      transaction: 'backend-built-base64',
      transactionEncoding: 'base64',
      network: 'devnet',
      kind: 'remove-authorization-lock',
      signatureRequests: [],
    });
    await wallet.removeAuthorizationLock({
      ...args,
      lockId: 0,
      network: 'mainnet',
      requesterAuthority: { ed25519: { publicKey: 'override' } },
    });
    expect(calls[1]?.body).toMatchObject({
      network: 'NETWORK_MAINNET',
      requesterAuthority: { ed25519: { publicKey: 'override' } },
      lockId: 0,
    });
  });

  test('normalizes numeric kind and snake-case prepared transaction fields', async () => {
    const client = new SwigClient({
      apiKey: 'test-key',
      network: 'mainnet',
      fetch: (async () =>
        Response.json({
          transaction: {
            unsigned_transaction: 'prepared',
            transaction_encoding: 1,
            kind: 8,
          },
        })) as unknown as typeof fetch,
    });
    const prepared = await client.wallets
      .use('config')
      .removeAuthorizationLock({
        ...args,
        requesterAuthority: { ed25519: { publicKey: 'engine' } },
      });
    expect(prepared).toMatchObject({
      transaction: 'prepared',
      transactionEncoding: 'base64',
      kind: 'remove-authorization-lock',
    });
  });

  test('validates the request before asking for a transaction', async () => {
    let calls = 0;
    const client = new SwigClient({
      apiKey: 'test-key',
      fetch: (async () => {
        calls++;
        return Response.json({});
      }) as unknown as typeof fetch,
    });
    const requesterAuthority = { ed25519: { publicKey: 'engine' } };
    await expect(
      client.wallets
        .use('config', { network: 'mainnet' })
        .removeAuthorizationLock(args),
    ).rejects.toThrow('requesterAuthority is required');
    await expect(
      client.wallets
        .use('config', { requesterAuthority })
        .removeAuthorizationLock(args),
    ).rejects.toThrow('network is required');
    for (const lockId of [-1, 1.5, 2 ** 32, Number.NaN]) {
      await expect(
        client.wallets
          .use('config', { network: 'mainnet', requesterAuthority })
          .removeAuthorizationLock({ ...args, lockId }),
      ).rejects.toThrow('lockId must be an unsigned 32-bit integer');
    }
    expect(calls).toBe(0);
  });

  test.each([400, 503])(
    'propagates HTTP %i without retrying the POST',
    async (status) => {
      let calls = 0;
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        retryOptions: { maxRetries: 3 },
        fetch: (async () => {
          calls++;
          return Response.json(
            { message: 'lock is held by another role' },
            { status },
          );
        }) as unknown as typeof fetch,
      });
      await expect(
        client.wallets
          .use('config', {
            requesterAuthority: { ed25519: { publicKey: 'engine' } },
          })
          .removeAuthorizationLock(args),
      ).rejects.toMatchObject({ statusCode: status });
      expect(calls).toBe(1);
    },
  );

  test('rejects a backend response without a prepared transaction', async () => {
    const client = new SwigClient({
      apiKey: 'test-key',
      network: 'mainnet',
      fetch: (async () => Response.json({})) as unknown as typeof fetch,
    });
    await expect(
      client.wallets
        .use('config', {
          requesterAuthority: { ed25519: { publicKey: 'engine' } },
        })
        .removeAuthorizationLock(args),
    ).rejects.toThrow('missing transaction');
  });
});
