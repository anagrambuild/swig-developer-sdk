import {
  Keypair,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import assert from 'node:assert/strict';
import test from 'node:test';
import { signReviewedAgentGrant } from '../src/agent-approval';
import type { WalletFlow } from '../src/contracts';

test('only the same reviewed grant reaches Phantom for signature', async (t) => {
  const owner = Keypair.generate();
  const transaction = new VersionedTransaction(
    new TransactionMessage({
      payerKey: owner.publicKey,
      recentBlockhash: owner.publicKey.toBase58(),
      instructions: [],
    }).compileToV0Message(),
  );
  const grant: WalletFlow = {
    state: 'awaiting-approval',
    owner: owner.publicKey.toBase58(),
    walletAddress: 'wallet',
    configAddress: 'config',
    agentPublicKey: 'agent',
    allowance: { sol: '1000', token: '10', window: '432000' },
    transaction: Buffer.from(transaction.serialize()).toString('base64'),
  };
  const sign = t.mock.fn(async (tx: VersionedTransaction) => {
    tx.sign([owner]);
    return tx;
  });
  assert.equal(await signReviewedAgentGrant(undefined, grant, sign), undefined);
  for (const change of [
    { owner: 'another owner' },
    { walletAddress: 'another wallet' },
    { configAddress: 'another config' },
    { agentPublicKey: 'another agent' },
    { transaction: 'changed' },
    { allowance: { ...grant.allowance, sol: '1001' } },
    { allowance: { ...grant.allowance, token: '11' } },
    { allowance: { ...grant.allowance, window: '216000' } },
  ]) {
    assert.equal(
      await signReviewedAgentGrant(grant, { ...grant, ...change }, sign),
      undefined,
    );
  }
  assert.equal(sign.mock.callCount(), 0);
  transaction.sign([owner]);
  assert.equal(
    await signReviewedAgentGrant(grant, grant, sign),
    Buffer.from(transaction.serialize()).toString('base64'),
  );
  assert.equal(sign.mock.callCount(), 1);
});
