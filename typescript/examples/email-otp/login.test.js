import { generateKeyPair, SignJWT } from 'jose';
import assert from 'node:assert/strict';
import test from 'node:test';
import { verifyRegistration } from './check-registration.js';
import { appOrigin, isLoopback } from './origin.js';
import { LoginSessions } from './sessions.js';
import { describeGrant } from './web/preview.js';

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
test('the preview displays the observed mint and both human-readable limits', () => {
  const agentPolicy = {
    mint: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
    tokenAmount: '100000000',
    solAmount: '1000000000',
    window: '216000',
  };
  const wallet = {
    walletAddress: 'wallet',
    agentPublicKey: 'agent',
    agentPolicy,
  };
  assert.match(
    describeGrant(wallet),
    /100 Devnet USDC .* and 1 SOL per 216000 slots/,
  );
  assert.throws(() =>
    describeGrant({
      ...wallet,
      agentPolicy: { ...agentPolicy, mint: 'other' },
    }),
  );
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
