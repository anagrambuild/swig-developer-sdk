import { describe, expect, test } from 'bun:test';

import { SwigClient, type AddAuthorizationLockArgs } from '../index.js';

const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

const args: AddAuthorizationLockArgs = {
  feePayer: 'sponsor',
  amount: 1_000_000n,
  durationSlots: 150,
};

describe('wallet.addAuthorizationLock', () => {
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
            kind: 'PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK',
            wallet: { swigConfigAddress: 'config', walletAddress: 'wallet' },
          },
        });
      }) as unknown as typeof fetch,
    });
    const wallet = client.wallets.use('config', {
      network: 'devnet',
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
    });
    const prepared = await wallet.addAuthorizationLock(args);
    expect(calls.length).toBe(1);
    expect(calls[0]?.request.url).toBe(
      'https://api.example.test/transaction/wallet/authorization-lock/add',
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
      amount: '1000000',
      durationSlots: '150',
    });
    expect(prepared).toMatchObject({
      transaction: 'backend-built-base64',
      transactionEncoding: 'base64',
      network: 'devnet',
      kind: 'add-authorization-lock',
      signatureRequests: [],
    });
    await wallet.addAuthorizationLock({
      feePayer: 'sponsor',
      mint: USDC,
      amount: '18446744073709551615',
      expirySlot: 900n,
      network: 'mainnet',
      requesterAuthority: { secp256r1: { publicKey: 'passkey' } },
    });
    expect(calls[1]?.body).toEqual({
      network: 'NETWORK_MAINNET',
      swigAddress: 'config',
      feePayer: 'sponsor',
      requesterAuthority: { secp256r1: { publicKey: 'passkey' } },
      mint: USDC,
      amount: '18446744073709551615',
      expirySlot: '900',
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
            kind: 7,
          },
        })) as unknown as typeof fetch,
    });
    const prepared = await client.wallets.use('config').addAuthorizationLock({
      ...args,
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
    });
    expect(prepared).toMatchObject({
      transaction: 'prepared',
      transactionEncoding: 'base64',
      kind: 'add-authorization-lock',
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
    const wallet = client.wallets.use('config', {
      network: 'mainnet',
      requesterAuthority,
    });
    await expect(
      client.wallets
        .use('config', { network: 'mainnet' })
        .addAuthorizationLock(args),
    ).rejects.toThrow('requesterAuthority is required');
    await expect(
      client.wallets
        .use('config', { requesterAuthority })
        .addAuthorizationLock(args),
    ).rejects.toThrow('network is required');
    for (const invalid of [
      { feePayer: 'sponsor', amount: 1 },
      { feePayer: 'sponsor', amount: 1, expirySlot: 900, durationSlots: 150 },
    ] as unknown as AddAuthorizationLockArgs[]) {
      await expect(wallet.addAuthorizationLock(invalid)).rejects.toThrow(
        'exactly one authorization lock expiry',
      );
    }
    for (const amount of [0, '0', 0n]) {
      await expect(
        wallet.addAuthorizationLock({ ...args, amount }),
      ).rejects.toThrow('amount must be greater than zero');
    }
    for (const amount of [-1, 1.5, '1e3', '', '18446744073709551616']) {
      await expect(
        wallet.addAuthorizationLock({ ...args, amount }),
      ).rejects.toThrow(/amount/);
    }
    await expect(
      wallet.addAuthorizationLock({ ...args, amount: 2 ** 53 }),
    ).rejects.toThrow('use a bigint or decimal string');
    await expect(
      wallet.addAuthorizationLock({ ...args, durationSlots: -5 }),
    ).rejects.toThrow('durationSlots');
    for (const mint of ['', 'not-base58!', '11111', `${USDC}1`]) {
      await expect(
        wallet.addAuthorizationLock({ ...args, mint }),
      ).rejects.toThrow('mint must be a base58-encoded 32-byte public key');
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
            { message: 'role lacks ManageAuthorizationLocks' },
            { status },
          );
        }) as unknown as typeof fetch,
      });
      await expect(
        client.wallets
          .use('config', {
            requesterAuthority: { ed25519: { publicKey: 'engine' } },
          })
          .addAuthorizationLock(args),
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
        .addAuthorizationLock(args),
    ).rejects.toThrow('missing transaction');
  });

  test('rejects malformed prepared transaction objects and serialized values', async () => {
    for (const prepared of [
      42,
      'not-an-object',
      [],
      { transaction: 42 },
      { transaction: {} },
      { unsigned_transaction: 42 },
    ]) {
      const client = new SwigClient({
        apiKey: 'test-key',
        network: 'mainnet',
        fetch: (async () =>
          Response.json({ transaction: prepared })) as unknown as typeof fetch,
      });
      await expect(
        client.wallets
          .use('config', {
            requesterAuthority: { ed25519: { publicKey: 'engine' } },
          })
          .addAuthorizationLock(args),
      ).rejects.toThrow(/invalid prepared transaction|invalid transaction/i);
    }
  });
});
