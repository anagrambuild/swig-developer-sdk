import { describe, expect, test } from 'bun:test';

import { SwigClient, type SetRentClaimerArgs } from '../index.js';

const args: SetRentClaimerArgs = {
  feePayer: 'sponsor',
  rentClaimer: 'rent-vault',
};

describe('wallet.setRentClaimer', () => {
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
            kind: 'PREPARED_TRANSACTION_KIND_SET_RENT_CLAIMER',
            wallet: { swigConfigAddress: 'config', walletAddress: 'wallet' },
          },
        });
      }) as unknown as typeof fetch,
    });
    const wallet = client.wallets.use('config', {
      network: 'devnet',
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
    });
    const prepared = await wallet.setRentClaimer(args);
    expect(calls.length).toBe(1);
    expect(calls[0]?.request.url).toBe(
      'https://api.example.test/transaction/wallet/rent-claimer/set',
    );
    expect(calls[0]?.request.method).toBe('POST');
    expect(calls[0]?.request.headers.get('authorization')).toBe(
      'Bearer test-key',
    );
    expect(calls[0]?.body).toEqual({
      network: 'NETWORK_DEVNET',
      swigAddress: 'config',
      feePayer: 'sponsor',
      rentClaimer: 'rent-vault',
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
    });
    expect(prepared).toMatchObject({
      transaction: 'backend-built-base64',
      transactionEncoding: 'base64',
      network: 'devnet',
      kind: 'set-rent-claimer',
      signatureRequests: [],
    });
    await wallet.setRentClaimer({
      ...args,
      network: 'mainnet',
      requesterAuthority: { ed25519: { publicKey: 'override' } },
    });
    expect(calls[1]?.body).toMatchObject({
      network: 'NETWORK_MAINNET',
      requesterAuthority: { ed25519: { publicKey: 'override' } },
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
            kind: 6,
          },
        })) as unknown as typeof fetch,
    });
    const prepared = await client.wallets.use('config').setRentClaimer({
      ...args,
      requesterAuthority: { ed25519: { publicKey: 'engine' } },
    });
    expect(prepared).toMatchObject({
      transaction: 'prepared',
      transactionEncoding: 'base64',
      kind: 'set-rent-claimer',
    });
  });

  test('requires supported authority and network before requesting a transaction', async () => {
    let calls = 0;
    const client = new SwigClient({
      apiKey: 'test-key',
      fetch: (async () => {
        calls++;
        return Response.json({});
      }) as unknown as typeof fetch,
    });
    await expect(
      client.wallets.use('config', { network: 'mainnet' }).setRentClaimer(args),
    ).rejects.toThrow('requesterAuthority is required');
    await expect(
      client.wallets
        .use('config', {
          requesterAuthority: { ed25519: { publicKey: 'engine' } },
        })
        .setRentClaimer(args),
    ).rejects.toThrow('network is required');
    for (const requesterAuthority of [
      { secp256r1: { publicKey: 'passkey' } },
      { programExecProof: { roleId: 1, zkProof: 'proof' } },
      { participantSet: { address: 'set' } },
    ] as const) {
      await expect(
        client.wallets
          .use('config', { network: 'mainnet', requesterAuthority })
          .setRentClaimer(args),
      ).rejects.toThrow('direct Ed25519');
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
            { message: 'rent claimer is already set' },
            { status },
          );
        }) as unknown as typeof fetch,
      });
      await expect(
        client.wallets
          .use('config', {
            requesterAuthority: { ed25519: { publicKey: 'engine' } },
          })
          .setRentClaimer(args),
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
        .setRentClaimer(args),
    ).rejects.toThrow('missing transaction');
  });

  test('rejects malformed prepared transaction objects and serialized values', async () => {
    for (const prepared of [
      42,
      'not-an-object',
      [],
      { transaction: 42 },
      { transaction: {} },
      { transaction: [] },
      { transaction: true },
      { unsigned_transaction: 42 },
      { unsignedTransaction: {} },
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
          .setRentClaimer(args),
      ).rejects.toThrow(/invalid prepared transaction|invalid transaction/i);
    }
  });
});
