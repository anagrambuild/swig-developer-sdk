import bs58 from 'bs58';
import { describe, expect, test } from 'bun:test';

import { TransactionsClient } from './client.js';

describe('TransactionsClient', () => {
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
          transactionDigest: '11'.repeat(32),
          compilationEnvelope: 'envelope_123',
          threshold: 2,
          members: [
            {
              memberIndex: 0,
              signerType: 'secp256k1',
              publicKey: `02${'22'.repeat(32)}`,
              counter: 7,
              challenge: '33'.repeat(32),
            },
          ],
        },
      },
      approvals: [
        {
          memberIndex: 0,
          counter: 7,
          secp256k1: { signature: Uint8Array.from([1, 2, 3]) },
        },
      ],
    });

    expect(result.preparedTransaction.transaction).toBe('compiled-base64');
    expect(calls).toEqual([
      {
        path: '/transaction/participant-set/compile',
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
              transactionDigest: '11'.repeat(32),
              compilationEnvelope: 'envelope_123',
              threshold: 2,
              members: [
                {
                  memberIndex: 0,
                  signerType: 'PARTICIPANT_SET_SIGNER_TYPE_SECP256K1',
                  publicKey: `02${'22'.repeat(32)}`,
                  counter: 7,
                  challenge: '33'.repeat(32),
                },
              ],
            },
          },
          approvals: [
            {
              memberIndex: 0,
              counter: 7,
              secp256k1: { signature: 'AQID' },
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
