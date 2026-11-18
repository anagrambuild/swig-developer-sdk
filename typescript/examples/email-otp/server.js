import { createRemoteJWKSet } from 'jose';
import { randomBytes } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { SwigClient } from '../../dist/index.js';
import { LoginSessions } from './sessions.js';

const port = Number(process.env.PORT || 4181);
const origin = 'http://127.0.0.1:' + port;
const callbackUrl = origin + '/auth/callback';
const apiKey = process.env.SWIG_API_KEY;
const walletTemplateId = process.env.SWIG_WALLET_TEMPLATE_ID;
if (!apiKey || !walletTemplateId)
  throw new Error(
    'Set SWIG_API_KEY and SWIG_WALLET_TEMPLATE_ID in the server .env.local',
  );
const sdk = new SwigClient({ apiKey, network: 'devnet' });
const agentPublicKey =
  process.env.SWIG_AGENT_PUBLIC_KEY ||
  (await readFile('.local/agent-public-key', 'utf8')).trim();
const projectPaymasterAddress = process.env.SWIG_PROJECT_PAYMASTER_ADDRESS;
const app = await sdk.idp.ensureEmailWalletApp({
  network: 'devnet',
  callbackUrl,
  walletTemplateId,
  agentPublicKey,
  projectPaymasterAddress,
});
const issuer = 'https://backend.prod.infra.onswig.com/identity/public';
const jwks = createRemoteJWKSet(new URL(issuer + '/oidc/jwks.json'));
const index = await readFile(new URL('./web/index.html', import.meta.url));
const bundle = await readFile(new URL('./dist/main.js', import.meta.url));
const attempts = new Map();
const sessions = new LoginSessions();
const cleanup = setInterval(() => {
  sessions.expire();
  for (const [id, attempt] of attempts)
    if (attempt.expires <= Date.now()) attempts.delete(id);
}, 60000);
cleanup.unref();

createServer(async (req, res) => {
  res.setHeader('Cache-Control', 'no-store');
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('Referrer-Policy', 'no-referrer');
  if (req.headers.host !== new URL(origin).host) {
    res.writeHead(403).end();
    return;
  }
  if (req.method === 'GET' && (req.url === '/' || req.url === '/main.js')) {
    const script = req.url === '/main.js';
    res
      .writeHead(200, {
        'Content-Type': script ? 'text/javascript' : 'text/html; charset=utf-8',
      })
      .end(script ? bundle : index);
    return;
  }
  const cookies = new URLSearchParams(
    (req.headers.cookie || '').split('; ').join('&'),
  );
  const sessionId = cookies.get('session');
  if (req.method === 'GET' && req.url === '/session') {
    res
      .writeHead(200, { 'Content-Type': 'application/json' })
      .end(
        JSON.stringify({ authenticated: Boolean(sessions.subject(sessionId)) }),
      );
    return;
  }
  if (req.method !== 'POST' || req.headers.origin !== origin) {
    res.writeHead(403).end();
    return;
  }
  if (req.url === '/logout') {
    sessions.logout(sessionId);
    res
      .writeHead(204, {
        'Set-Cookie': 'session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0',
      })
      .end();
    return;
  }
  if (req.url === '/attempt') {
    const cookie = randomBytes(24).toString('hex');
    const state = randomBytes(24).toString('hex');
    const oidcNonce = randomBytes(32).toString('hex');
    attempts.set(cookie, { state, oidcNonce, expires: Date.now() + 600000 });
    res
      .writeHead(200, {
        'Content-Type': 'application/json',
        'Set-Cookie':
          'otp=' + cookie + '; HttpOnly; SameSite=Strict; Path=/; Max-Age=600',
      })
      .end(
        JSON.stringify({
          clientId: app.clientId,
          callbackUrl,
          oidcNonce,
          state,
          agentPublicKey,
        }),
      );
    return;
  }
  if (req.url !== '/complete') {
    res.writeHead(404).end();
    return;
  }
  const cookie = req.headers.cookie
    ?.split('; ')
    .find((v) => v.startsWith('otp='))
    ?.slice(4);
  const attempt = attempts.get(cookie);
  attempts.delete(cookie);
  try {
    if (!attempt || attempt.expires <= Date.now()) throw new Error();
    let body = '';
    for await (const bytes of req) {
      body += bytes.toString();
      if (body.length > 16384) throw new Error();
    }
    const { idToken, state } = JSON.parse(body);
    const session = await sessions.issue(
      idToken,
      attempt,
      state,
      issuer,
      app.clientId,
      jwks,
    );
    sessions.logout(sessionId);
    res
      .writeHead(204, {
        'Set-Cookie': [
          'otp=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0',
          'session=' +
            session.id +
            '; HttpOnly; SameSite=Strict; Path=/; Max-Age=' +
            session.maxAge,
        ],
      })
      .end();
  } catch {
    res.writeHead(401).end('Email sign-in could not be verified. Start again.');
  }
}).listen(port, '127.0.0.1', () =>
  console.log('Email wallet app listening at', origin),
);
