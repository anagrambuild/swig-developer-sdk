import { describe, expect, test } from 'bun:test';

import { SwigClient } from '../server/typescript/index.js';

describe('ParticipantSetsClient', () => {
  test('creates a mixed ParticipantSet through the typed resource', async () => {
    const calls: Array<{ path: string; body: unknown }> = [];
    const client = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'https://example.test',
      network: 'devnet',
      fetch: (async (input, init) => {
        const request = new Request(input, init);
        calls.push({
          path: new URL(request.url).pathname,
          body: JSON.parse(await request.text()),
        });
        return Response.json({
          participantSetAddress: 'participant_set_123',
          setId: 'set_id_123',
          transaction: {
            transaction: 'prepared-base64',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            network: 'NETWORK_DEVNET',
            kind: 'PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET',
          },
        });
      }) as typeof fetch,
    });

    await expect(
      client.participantSets.create({
        swigConfigAddress: 'swig_123',
        feePayer: 'payer_123',
        threshold: 2,
        members: [
          { ed25519: { publicKey: 'ed25519_public_key' } },
          { secp256k1: { publicKey: `02${'11'.repeat(32)}` } },
          { secp256r1: { publicKey: `03${'22'.repeat(32)}` } },
        ],
        setId: 'set_id_123',
      }),
    ).resolves.toMatchObject({
      participantSetAddress: 'participant_set_123',
      setId: 'set_id_123',
      transaction: {
        transaction: 'prepared-base64',
        kind: 'create-participant-set',
      },
    });
    expect(calls).toEqual([
      {
        path: '/transaction/wallet/participant-set/create',
        body: {
          network: 'NETWORK_DEVNET',
          feePayer: 'payer_123',
          swigAddress: 'swig_123',
          setId: 'set_id_123',
          threshold: 2,
          members: [
            { ed25519: { publicKey: 'ed25519_public_key' } },
            { secp256k1: { publicKey: `02${'11'.repeat(32)}` } },
            { secp256r1: { publicKey: `03${'22'.repeat(32)}` } },
          ],
        },
      },
    ]);
  });
});
