import {
  Connection,
  Keypair,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import { SwigClient } from '@swig-wallet/developer-sdk';
import type { ReservedWalletSetup } from '@swig-wallet/developer-sdk/onboarding';
import assert from 'node:assert/strict';
import test from 'node:test';
import { DemoState } from '../server/state';
import { advanceAppWallet } from '../server/wallet';

test('a single server call persists the exact payload before sending and resumes an uncertain receipt', async (t) => {
  const state = new DemoState(':memory:');
  t.after(() => state.db.close());
  const key = Keypair.generate().publicKey;
  const tx = new VersionedTransaction(
    new TransactionMessage({
      payerKey: key,
      recentBlockhash: key.toBase58(),
      instructions: [
        SystemProgram.transfer({ fromPubkey: key, toPubkey: key, lamports: 0 }),
      ],
    }).compileToLegacyMessage(),
  );
  const queued: ReservedWalletSetup = {
    network: 'devnet',
    owner: key.toBase58(),
    agent: 'agent',
    walletAddress: 'wallet',
    swigConfigAddress: key.toBase58(),
    policyId: 'policy',
    salt: '00'.repeat(32),
    permissions: [],
    step: {
      kind: 'queued',
      operation: 'create',
      transaction: Buffer.from(tx.serialize()).toString('base64'),
      idempotencyKey: 'stable',
    },
  };
  const client = new SwigClient({ apiKey: 'test' }),
    rpc = new Connection('http://localhost:8899');
  t.mock.method(rpc, 'isBlockhashValid', async () => ({
    value: true,
    context: { slot: 1 },
  }));
  let sent = 0;
  t.mock.method(
    client.transactions,
    'sponsor',
    async (payload: { idempotencyKey: string; transaction: string }) => {
      assert.deepEqual(state.getAttempt(queued.owner)?.step, {
        ...queued.step,
        kind: 'submit',
      });
      assert.equal(payload.idempotencyKey, 'stable');
      if (++sent === 1) throw new Error('receipt unavailable');
      return {
        signature: 'receipt',
        requestId: 'request',
        spentByPaymaster: '0',
      };
    },
  );
  await assert.rejects(
    advanceAppWallet(state, client, rpc, queued),
    /receipt unavailable/,
  );
  const saved = state.getAttempt(queued.owner);
  assert.ok(saved);
  assert.deepEqual((await advanceAppWallet(state, client, rpc, saved)).step, {
    kind: 'confirm',
    operation: 'create',
    signature: 'receipt',
  });
  assert.equal(sent, 2);
  assert.equal(state.getAttempt(queued.owner)?.step.kind, 'confirm');
});
