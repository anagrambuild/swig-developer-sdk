import { Keypair } from '@solana/web3.js';
import type { ReservedWalletSetup } from '@swig-wallet/developer-sdk/onboarding';
import assert from 'node:assert/strict';
import { createPrivateKey, sign } from 'node:crypto';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { DemoState } from '../server/state';

function signMessage(keypair: Keypair, message: string): Uint8Array {
  return sign(
    null,
    Buffer.from(message),
    createPrivateKey({
      key: Buffer.concat([
        Buffer.from('302e020100300506032b657004220420', 'hex'),
        Buffer.from(keypair.secretKey.slice(0, 32)),
      ]),
      format: 'der',
      type: 'pkcs8',
    }),
  );
}

test('login verifies the exact server challenge and consumes it once', () => {
  const state = new DemoState(':memory:');
  try {
    const owner = Keypair.generate();
    const challenge = state.issueChallenge(
      owner.publicKey.toBase58(),
      'http://127.0.0.1:4173',
    );
    const signature = signMessage(owner, challenge.message);
    assert.equal(
      challenge.message.split('\n\n')[1],
      'Sign in to your Devnet Swig wallet.',
    );
    const session = state.authenticate(challenge.id, signature);
    assert.equal(state.sessionOwner(session.token), owner.publicKey.toBase58());
    assert.equal(state.sessionOwner('wrong-token'), null);
    assert.throws(
      () => state.authenticate(challenge.id, signature),
      /already used/,
    );
    state.logout(session.token);
    assert.equal(state.sessionOwner(session.token), null);
  } finally {
    state.db.close();
  }
});

test('wrong key, altered message and expired login cannot authenticate', () => {
  const state = new DemoState(':memory:');
  try {
    const owner = Keypair.generate();
    const wrongKey = state.issueChallenge(
      owner.publicKey.toBase58(),
      'http://127.0.0.1:4173',
    );
    assert.throws(
      () =>
        state.authenticate(
          wrongKey.id,
          signMessage(Keypair.generate(), wrongKey.message),
        ),
      /invalid/,
    );
    const altered = state.issueChallenge(
      owner.publicKey.toBase58(),
      'http://127.0.0.1:4173',
    );
    assert.throws(
      () =>
        state.authenticate(
          altered.id,
          signMessage(owner, `${altered.message}changed`),
        ),
      /invalid/,
    );
    const expired = state.issueChallenge(
      owner.publicKey.toBase58(),
      'http://127.0.0.1:4173',
    );
    state.db
      .prepare('UPDATE challenges SET expires = 0 WHERE id = ?')
      .run(expired.id);
    assert.throws(
      () => state.authenticate(expired.id, signMessage(owner, expired.message)),
      /expired/,
    );
  } finally {
    state.db.close();
  }
});

test('reservation, transaction and idempotency key survive a process restart', () => {
  const directory = mkdtempSync(join(tmpdir(), 'swig-demo-test-'));
  const path = join(directory, 'state.sqlite');
  const attempt: ReservedWalletSetup = {
    owner: 'test-owner',
    agent: 'agent',
    policyId: 'policy',
    network: 'devnet',
    walletAddress: 'wallet',
    swigConfigAddress: 'config',
    salt: 'aa'.repeat(32),
    permissions: [{ type: 'programAll' }],
    step: {
      kind: 'submit',
      operation: 'agent',
      transaction: 'signed',
      idempotencyKey: 'same-key',
    },
  };
  try {
    const state = new DemoState(path);
    state.saveAttempt(attempt);
    state.db.close();
    const restarted = new DemoState(path);
    try {
      assert.deepEqual(restarted.getAttempt('test-owner'), attempt);
      assert.equal(restarted.getAttempt('another-owner'), null);
    } finally {
      restarted.db.close();
    }
  } finally {
    rmSync(directory, { recursive: true });
  }
});
