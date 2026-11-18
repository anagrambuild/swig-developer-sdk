import {
  Keypair,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import bs58 from 'bs58';
import { describe, expect, test } from 'bun:test';
import { HttpClient } from '../core/http.js';

import { TransactionsClient } from './client.js';

describe('TransactionsClient', () => {
  for (const version of ['legacy', 'v0'] as const) {
    test(`sign returns the paymaster-signed ${version} transaction without submitting it`, async () => {
      const payer = Keypair.generate();
      const sender = Keypair.generate();
      const message = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: sender.publicKey.toBase58(),
        instructions: [
          SystemProgram.transfer({
            fromPubkey: sender.publicKey,
            toPubkey: payer.publicKey,
            lamports: 1,
          }),
        ],
      });
      const tx = new VersionedTransaction(
        version === 'v0'
          ? message.compileToV0Message()
          : message.compileToLegacyMessage(),
      );
      tx.sign([sender]);
      const original = tx.serialize();
      tx.sign([payer]);
      const signed = tx.serialize();
      const externalId = version === 'v0' ? 'order-42' : undefined;
      const calls: string[] = [];
      const http = new HttpClient({
        apiKey: 'fixture',
        baseUrl: 'https://sdk.test',
        retry: { maxRetries: 0, retryDelay: 0, backoffMultiplier: 1 },
        fetch: Object.assign(
          async (input: RequestInfo | URL, init?: RequestInit) => {
            const request = new Request(input, init);
            calls.push(request.method + ' ' + request.url);
            expect(await request.json()).toEqual({
              base58_encoded_transaction: bs58.encode(original),
              network: 'devnet',
              ...(externalId === undefined ? {} : { external_id: externalId }),
            });
            return Response.json({
              request_id: 'req-1',
              signed_transaction: bs58.encode(tx.serialize()),
            });
          },
          { preconnect: fetch.preconnect },
        ),
      });
      const client = new TransactionsClient(http, 'devnet');
      const args = { transaction: bytesToBase64(original), externalId };
      expect(await client.sign(args)).toEqual({
        requestId: 'req-1',
        signedTransaction: bytesToBase64(signed),
      });
      expect(calls).toEqual(['POST https://sdk.test/paymaster/sign']);

      const validPayerSignature = tx.signatures[0]?.slice();
      if (!validPayerSignature)
        throw new Error('Fixture is missing its payer signature');
      const invalidPayerSignature = validPayerSignature.slice();
      invalidPayerSignature[0] = (invalidPayerSignature[0] ?? 0) ^ 1;
      tx.addSignature(payer.publicKey, invalidPayerSignature);
      await expect(client.sign(args)).rejects.toThrow(
        'Invalid paymaster signature',
      );
      tx.addSignature(payer.publicKey, validPayerSignature);

      // A valid payer signature alone does not bind the other signature slots.
      tx.addSignature(sender.publicKey, new Uint8Array(64));
      await expect(client.sign(args)).rejects.toThrow(
        'Paymaster changed the transaction',
      );
      tx.addSignature(payer.publicKey, new Uint8Array(64));
      await expect(client.sign(args)).rejects.toThrow(
        'Missing paymaster signature',
      );
    });
  }

  test('sign preserves a v1 message with its signatures at the end', async () => {
    // Signed getTransaction fixture from the paymaster backend.
    const signed = Buffer.from(
      'gQEAAQwAAAAGhoWdOMgKlyFIGEFpF+C/j8oVfkZPmMz9D/qd6S/UggEFyoTiTXdhUBQKFFAB/JZaRp9/+qq44d72uHsnwhwnZ459Jbtx8Ujed9Z7sMeyVrIT3x6JCftdlPGzI9E24xRHK4IeTael0NiJg7w/YTr89LR+cV4kDlwt4UIZ7iM4gtBtAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAANDOlC4efFBuIY3w198cUvr9w1KeSNZ00dskx1tUzMvkANAwAAAAIABARQAAEAAgMAAAEAIAD7/6XyxGmB28d+vHdY/8iLZCfN+WeKs8alx3iWOAPnYunxyc+BG99zsbtOLOMEosAUgsfF3ODUnvLEINKijQG4URcIAAAACAAAAE2nfKtvTNbFcmLsydZHawTD8EIeCYSLYWzYzpMVZsWivSHo3XEyLXWe7nuHA4jCg9+LvCeArE4NYwP0++hIyAM=',
      'base64',
    );
    const unsigned = Buffer.from(signed);
    unsigned.fill(0, unsigned.length - 64);
    const http = new HttpClient({
      apiKey: 'fixture',
      baseUrl: 'https://sdk.test',
      retry: { maxRetries: 0, retryDelay: 0, backoffMultiplier: 1 },
      fetch: Object.assign(
        async () =>
          Response.json({
            request_id: 'req-v1',
            signed_transaction: bs58.encode(signed),
          }),
        { preconnect: fetch.preconnect },
      ),
    });
    expect(
      await new TransactionsClient(http).sign({
        transaction: unsigned.toString('base64'),
      }),
    ).toEqual({
      requestId: 'req-v1',
      signedTransaction: signed.toString('base64'),
    });
  });

  test('compiles detached ParticipantSet approvals without submitting', async () => {
    const calls: Array<{ path: string; body: unknown }> = [];
    const transactions = new TransactionsClient({
      post: async (path: string, body: unknown) => {
        calls.push({ path, body });
        return {
          transaction: {
            transaction: 'compiled-base64',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            network: 'NETWORK_DEVNET',
          },
          authorizationExpirationSlot: '12345',
        };
      },
    } as never);

    const result = await transactions.compileParticipantSetApprovals({
      preparedTransaction: {
        transaction: 'original-base64',
        transactionEncoding: 'base64',
        network: 'devnet',
        signatureRequests: [],
        participantSetApprovalPlan: {
          type: 'participantSet',
          participantSetAddress: 'participant_set_123',
          roleId: 4,
          expirationSlot: '12345',
          nonce: 7,
          transactionDigest: '11'.repeat(32),
          compilationEnvelope: 'envelope_123',
          threshold: 2,
          members: [
            {
              memberIndex: 0,
              authority: {
                secp256k1: { publicKey: `02${'22'.repeat(32)}` },
              },
              challenge: '33'.repeat(32),
            },
            {
              memberIndex: 1,
              authority: {
                ed25519: { publicKey: 'ed25519_public_key' },
              },
              challenge: '44'.repeat(32),
            },
          ],
        },
      },
      approvals: [
        {
          memberIndex: 0,
          secp256k1: { signature: Uint8Array.from([1, 2, 3]) },
        },
        {
          memberIndex: 1,
          ed25519: { signature: Uint8Array.from([4, 5, 6]) },
        },
      ],
    });

    expect(result).toMatchObject({
      transaction: { transaction: 'compiled-base64' },
      authorizationExpirationSlot: '12345',
    });
    expect(calls).toEqual([
      {
        path: '/transaction/wallet/participant-set/compile',
        body: {
          preparedTransaction: {
            transaction: 'original-base64',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            wallet: undefined,
            expiresAt: undefined,
            network: 'NETWORK_DEVNET',
            recentBlockhash: undefined,
            kind: undefined,
            signatureRequests: [],
            participantSetApprovalPlan: {
              participantSetAddress: 'participant_set_123',
              roleId: 4,
              expirationSlot: '12345',
              nonce: 7,
              transactionDigest: '11'.repeat(32),
              compilationEnvelope: 'envelope_123',
              threshold: 2,
              members: [
                {
                  memberIndex: 0,
                  authority: {
                    secp256k1: { publicKey: `02${'22'.repeat(32)}` },
                  },
                  challenge: '33'.repeat(32),
                },
                {
                  memberIndex: 1,
                  authority: {
                    ed25519: { publicKey: 'ed25519_public_key' },
                  },
                  challenge: '44'.repeat(32),
                },
              ],
            },
          },
          approvals: [
            {
              memberIndex: 0,
              secp256k1: { signature: 'AQID' },
            },
            {
              memberIndex: 1,
              ed25519: { signature: 'BAUG' },
            },
          ],
        },
      },
    ]);
  });

  test('sponsors base64 transactions through the deployed paymaster endpoint', async () => {
    const transactionBytes = Uint8Array.from([1, 2, 3, 4, 5]);
    const calls: Array<{
      path: string;
      body: unknown;
      options: unknown;
    }> = [];
    const transactions = new TransactionsClient(
      {
        post: async (path: string, body: unknown, options: unknown) => {
          calls.push({ path, body, options });
          return {
            request_id: 'request_123',
            signature: 'sponsored_signature_123',
            spent_by_paymaster: '5000',
          };
        },
      } as never,
      'devnet',
    );

    await expect(
      transactions.sponsor({
        transaction: bytesToBase64(transactionBytes),
        idempotencyKey: 'sponsor-request-123',
        externalId: 'order-42',
      }),
    ).resolves.toEqual({
      requestId: 'request_123',
      signature: 'sponsored_signature_123',
      spentByPaymaster: '5000',
    });

    expect(calls).toEqual([
      {
        path: '/paymaster/sponsor',
        body: {
          base58_encoded_transaction: bs58.encode(transactionBytes),
          network: 'devnet',
          idempotencyKey: 'sponsor-request-123',
          external_id: 'order-42',
        },
        options: { retry: true },
      },
    ]);
  });

  test('sponsors a mainnet bundle and reports the bundle response', async () => {
    const calls: Array<{ path: string; body: unknown; options: unknown }> = [];
    const transactions = new TransactionsClient(
      {
        post: async (path: string, body: unknown, options: unknown) => {
          calls.push({ path, body, options });
          return {
            request_id: 'request_bundle',
            bundle_id: 'bundle_123',
            signatures: ['signature_1'],
            estimated_spent_by_paymaster: '9000',
          };
        },
      } as never,
      'mainnet',
    );
    const transaction = bytesToBase64(Uint8Array.from([1, 2, 3]));

    await expect(
      transactions.sponsorBundle({
        transactions: [transaction],
        idempotencyKey: 'bundle-request-123',
      }),
    ).resolves.toEqual({
      requestId: 'request_bundle',
      bundleId: 'bundle_123',
      signatures: ['signature_1'],
      estimatedSpentByPaymaster: '9000',
    });
    expect(calls).toEqual([
      {
        path: '/paymaster/sponsor/bundle',
        body: {
          base58_encoded_transactions: [
            bs58.encode(Uint8Array.from([1, 2, 3])),
          ],
          network: 'mainnet',
          idempotencyKey: 'bundle-request-123',
        },
        options: { retry: true },
      },
    ]);
  });
});

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';

  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }

  return btoa(binary);
}
