import { generateKeyPair, SignJWT } from 'jose';
import assert from 'node:assert/strict';
import test from 'node:test';
import { verifyRegistration } from './check-registration.js';
import { appOrigin, isLoopback } from './origin.js';
import { LoginSessions } from './sessions.js';
import { verifyWalletEmail } from './web/verify.js';

test('only a verified login issues an expiring application session', async () => {
  const issuer = 'https://issuer.test',
    client = 'client';
  const { publicKey, privateKey } = await generateKeyPair('EdDSA');
  const idToken = await new SignJWT({ nonce: 'nonce' })
    .setSubject('verified-user')
    .setIssuer(issuer)
    .setAudience(client)
    .setIssuedAt()
    .setExpirationTime('5m')
    .setProtectedHeader({ alg: 'EdDSA' })
    .sign(privateKey);
  const attempt = {
    oidcNonce: 'nonce',
    state: 'state',
    expires: Date.now() + 60000,
  };
  const sessions = new LoginSessions();
  for (const [token, nonce, state] of [
    [idToken, 'other', 'state'],
    ['invalid', 'nonce', 'state'],
    [idToken, 'nonce', 'wrong'],
  ]) {
    await assert.rejects(
      sessions.issue(
        token,
        { ...attempt, oidcNonce: nonce },
        state,
        issuer,
        client,
        publicKey,
      ),
    );
  }
  const { id } = await sessions.issue(
    idToken,
    attempt,
    'state',
    issuer,
    client,
    publicKey,
  );
  assert.equal(sessions.subject(id), 'verified-user');
  assert.equal(sessions.subject(id, Date.now() + 300001), undefined);
});
test('Verify email proceeds directly to trusted approval and validates the registered agent and mint', async (t) => {
  const events = [];
  const popup = { document: { body: {} }, close: () => events.push('close') };
  const previous = globalThis.window;
  t.after(() => { if (previous === undefined) delete globalThis.window; else globalThis.window = previous; });
  globalThis.window = { open: () => { events.push('open'); return popup; } };
  const wallet = { agentPublicKey: 'agent', agentPolicy: { mint: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU' } };
  const input = { challengeId: 'challenge', code: '123456' };
  const complete = { status: 'complete', callbackUrl: 'callback' };
  const pending = { status: 'approval_required', wallet };
  const email = {
    verify: async () => { events.push('verify'); return pending; },
    approve: async (request, options) => {
      assert.deepEqual(request, { challengeId: input.challengeId });
      assert.equal(options.approvalWindow, popup);
      events.push('approve');
      return complete;
    },
    cancel: () => events.push('cancel'),
  };
  assert.equal(await verifyWalletEmail(email, input, 'agent'), complete);
  assert.deepEqual(events, ['open', 'verify', 'approve', 'close']);
  events.length = 0;
  await assert.rejects(verifyWalletEmail(email, input, 'other'), /registered agent/);
  assert.deepEqual(events, ['open', 'verify', 'cancel', 'close']);
  globalThis.window.open = () => null;
  assert.equal(await verifyWalletEmail(email, input, 'agent'), pending);
  wallet.agentPolicy.mint = 'other';
  await assert.rejects(verifyWalletEmail(email, input, 'agent'), /Devnet USDC/);
  email.verify = async () => complete;
  assert.equal(await verifyWalletEmail(email, input, 'agent'), complete);
});

test('accepts HTTP on literal loopback hosts and HTTPS for hosted callbacks', () => {
  assert.equal(appOrigin(undefined, '4181').origin, 'http://127.0.0.1:4181');
  assert.equal(appOrigin(undefined, '4312').origin, 'http://127.0.0.1:4312');
  for (const host of ['localhost', '127.0.0.1', '[::1]']) {
    assert.ok(isLoopback(appOrigin('https://' + host + ':4182').hostname));
  }
  for (const raw of [
    'http://localhost:3000',
    'http://127.0.0.1:51837',
    'http://localhost',
  ]) {
    assert.equal(appOrigin(raw).origin, raw);
  }
  const proxy = appOrigin('https://wallet.example/');
  assert.equal(isLoopback(proxy.hostname), false);
  assert.equal(
    new URL('/auth/callback', proxy).href,
    'https://wallet.example/auth/callback',
  );
  for (const invalid of [
    'http://localhost.example:4181',
    'http://wallet.example',
    'http://127.1:4181',
    'http://10.0.0.1:3000',
    'http://[::1]:4181',
    'http://localhost:0',
    'http://user@localhost:3000',
    'https://user@wallet.example',
    'https://wallet.example/?',
    'https://wallet.example/#',
    'https://wallet.example/other',
    'https://wallet.example/../',
  ]) {
    assert.throws(() => appOrigin(invalid));
  }
});

test('readiness checks the registered origin, client and agent', async () => {
  const originalFetch = globalThis.fetch;
  try {
    for (const origin of [
      appOrigin('http://localhost:4312'),
      appOrigin('https://wallet.example'),
    ]) {
      const attempt = {
        clientId: 'this-client',
        agentPublicKey: 'this-agent',
        callbackUrl: new URL('/auth/callback', origin).href,
      };
      globalThis.fetch = async () => Response.json(attempt);
      await verifyRegistration(
        origin,
        attempt.clientId,
        attempt.agentPublicKey,
      );
      for (const wrong of [
        { clientId: 'other' },
        { agentPublicKey: 'stale' },
        { callbackUrl: 'http://localhost:4313/auth/callback' },
      ]) {
        globalThis.fetch = async () => Response.json({ ...attempt, ...wrong });
        await assert.rejects(
          verifyRegistration(origin, attempt.clientId, attempt.agentPublicKey),
          /does not match/,
        );
      }
    }
  } finally {
    globalThis.fetch = originalFetch;
  }
});
