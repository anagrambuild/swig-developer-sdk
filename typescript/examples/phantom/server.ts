import { randomBytes } from 'node:crypto';
import { mkdir, readFile } from 'node:fs/promises';
import { createServer } from 'node:http';

import { Connection, PublicKey } from '@solana/web3.js';
import { SwigClient, SwigDeveloperSdkError } from '@swig-wallet/developer-sdk';
import bs58 from 'bs58';
import express, {
  type NextFunction,
  type Request,
  type Response,
} from 'express';
import { join } from 'node:path';

import { z } from 'zod';

import {
  startReservedWalletSetup,
  type ReservedWalletSetup,
} from '@swig-wallet/developer-sdk/onboarding';
import { DemoState } from './server/state';
import { advanceAppWallet } from './server/wallet';

import type { WalletFlow } from './src/contracts';
import { validateTemplate } from './src/policy';

const origin = 'http://127.0.0.1:4173';
const directory = join(process.cwd(), '.local');
await mkdir(directory, { recursive: true, mode: 0o700 });
process.umask(0o077);
const agentKey =
  process.env.SWIG_AGENT_PUBLIC_KEY ||
  (await readFile(join(directory, 'agent-public-key'), 'utf8')).trim();
const policyId = z.string().min(1).parse(process.env.SWIG_WALLET_TEMPLATE_ID);
const projectId = z.string().cuid().parse(process.env.SWIG_PROJECT_ID);
const state = new DemoState(join(directory, projectId + '.sqlite'));
const apiKey = process.env.SWIG_API_KEY;
const swig = apiKey ? new SwigClient({ apiKey, network: 'devnet' }) : null;
if (
  swig &&
  (await swig.getProjectSummary({ network: 'devnet' })).organizationId !==
    projectId
) {
  throw new Error(
    'The project API key does not match SWIG_PROJECT_ID. Reconnect the local credential helper.',
  );
}
const connection = new Connection(
  process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
  'confirmed',
);
const app = express();
app.disable('x-powered-by');
const server = createServer(app);
// A single local process owns both this listener and the database. Do not run
// replicas without replacing this serialization with a shared job owner.
const busyOwners = new Set<string>();

class AppError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

function cookie(request: Request, name: string): string {
  const part = request.headers.cookie
    ?.split(';')
    .map((item) => item.trim())
    .find((item) => item.startsWith(`${name}=`));
  const value = part?.slice(name.length + 1) ?? '';
  return /^[a-f0-9]{64}$/.test(value) ? value : '';
}

function sessionOwner(request: Request): string {
  const owner = state.sessionOwner(cookie(request, 'swig_session'));
  if (!owner)
    throw new AppError(
      401,
      'LOGIN_REQUIRED',
      'Connect Phantom and approve the sign-in message.',
    );
  return owner;
}

function present(setup: ReservedWalletSetup): WalletFlow {
  const step = setup.step;
  const phase =
    step.kind === 'failed'
      ? 'failed'
      : step.kind === 'ready'
        ? 'ready'
        : step.kind === 'verify'
          ? 'verifying'
          : step.kind === 'sign'
            ? 'awaiting-approval'
            : step.operation === 'agent'
              ? 'adding-agent'
              : step.kind === 'prepare'
                ? 'reserved'
                : 'creating';
  return {
    state: phase,
    ...(step.kind === 'failed' ? { failedSignature: step.signature } : {}),
    owner: setup.owner,
    agentPublicKey: setup.agent,
    walletAddress: setup.walletAddress,
    configAddress: setup.swigConfigAddress,
    ...(step.kind === 'sign' ? { transaction: step.transaction } : {}),
  };
}

app.use((req, res, next) => {
  if (req.headers.host !== '127.0.0.1:4173') {
    res.status(403).end('Open http://127.0.0.1:4173');
    return;
  }
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('Referrer-Policy', 'no-referrer');
  res.setHeader('X-Frame-Options', 'DENY');
  if (req.path.startsWith('/api/')) {
    res.setHeader('Cache-Control', 'no-store');
    if (req.method !== 'GET' && req.headers.origin !== origin) {
      res.status(403).json({
        code: 'INVALID_ORIGIN',
        message: 'Open the app locally to continue.',
      });
      return;
    }
  }
  next();
});
app.use(express.json({ limit: '16kb' }));

app.get('/api/config', (_req, res) => {
  res.json({
    configured: !!swig,
    network: 'devnet',
    policyId,
    agentPublicKey: agentKey,
  });
});

app.post('/api/auth/challenge', (req, res) => {
  const { publicKey } = z
    .object({ publicKey: z.string().min(32).max(44) })
    .strict()
    .parse(req.body);
  const owner = new PublicKey(publicKey);
  if (!PublicKey.isOnCurve(owner.toBytes()))
    throw new AppError(400, 'INVALID_OWNER', 'Use a Phantom signing account.');
  const challenge = state.issueChallenge(owner.toBase58(), origin);
  res.cookie('swig_challenge', challenge.id, {
    httpOnly: true,
    sameSite: 'strict',
    maxAge: 300_000,
    path: '/api/auth',
  });
  res.json({ message: challenge.message });
});

app.post('/api/auth/session', (req, res) => {
  const { signature } = z
    .object({ signature: z.string().min(64).max(90) })
    .strict()
    .parse(req.body);
  const session = state.authenticate(
    cookie(req, 'swig_challenge'),
    bs58.decode(signature),
  );
  res.clearCookie('swig_challenge', { path: '/api/auth' });
  res.cookie('swig_session', session.token, {
    httpOnly: true,
    sameSite: 'strict',
    maxAge: 3_600_000,
  });
  res.json({ owner: session.owner });
});

app.post('/api/auth/logout', (req, res) => {
  state.logout(cookie(req, 'swig_session'));
  res.clearCookie('swig_session');
  res.json({ ok: true });
});

app.post('/api/wallet/advance', async (req, res) => {
  const owner = sessionOwner(req);
  const body = z
    .object({
      signedTransaction: z.string().max(5000).optional(),
      retryFailedSignature: z.string().max(100).optional(),
    })
    .strict()
    .parse(req.body);
  if (!swig)
    throw new AppError(
      503,
      'API_KEY_REQUIRED',
      'Configure SWIG_API_KEY on the local server, then restart it. Start the server through swig-developer-mcp exec -- npm start.',
    );
  if (busyOwners.has(owner))
    throw new AppError(
      409,
      'IN_PROGRESS',
      'This wallet is already being processed. Continue in a moment.',
    );
  busyOwners.add(owner);
  try {
    let attempt = state.getAttempt(owner);
    if (!attempt) {
      if (body.signedTransaction || body.retryFailedSignature)
        throw new AppError(
          409,
          'UNEXPECTED_SIGNATURE',
          'Reconnect Phantom to begin.',
        );
      const template = await swig.walletTemplates.get(policyId);
      validateTemplate(template, policyId);
      attempt = await startReservedWalletSetup(swig, {
        network: 'devnet',
        owner,
        agent: agentKey,
        template,
        salt: randomBytes(32),
      });
      state.saveAttempt(attempt);
    } else {
      if (
        attempt.owner !== owner ||
        attempt.agent !== agentKey ||
        attempt.policyId !== policyId
      )
        throw new AppError(
          409,
          'CONFIG_CHANGED',
          'Restore the original app configuration before resuming.',
        );
      attempt = await advanceAppWallet(
        state,
        swig,
        connection,
        attempt,
        body.signedTransaction
          ? { signedTransaction: body.signedTransaction }
          : body.retryFailedSignature
            ? { retryFailedSignature: body.retryFailedSignature }
            : undefined,
      );
    }
    res.json(present(attempt));
  } catch (error) {
    const saved = state.getAttempt(owner);
    if (saved?.step.kind === 'submit')
      throw new AppError(
        503,
        'SPONSORSHIP_UNCONFIRMED',
        `The sponsorship outcome is unconfirmed. Resume to retry the same transaction. If it remains unavailable, reconcile request ${saved.step.idempotencyKey} before preparing another transaction.`,
      );
    throw error;
  } finally {
    busyOwners.delete(owner);
  }
});

app.use('/api', (_req, res) =>
  res.status(404).json({ code: 'NOT_FOUND', message: 'Unknown app endpoint.' }),
);
app.use((error: unknown, _req: Request, res: Response, _next: NextFunction) => {
  if (error instanceof AppError) {
    res.status(error.status).json({ code: error.code, message: error.message });
    return;
  }
  if (error instanceof z.ZodError) {
    res.status(400).json({
      code: 'INVALID_INPUT',
      message: 'The request contains invalid or missing fields.',
    });
    return;
  }
  if (error instanceof SwigDeveloperSdkError) {
    console.error('Swig request failed', {
      code: error.code,
      statusCode: error.statusCode,
    });
    res.status(502).json({
      code: error.code,
      message: `Swig request failed (${error.code}, HTTP ${error.statusCode}). Check your project key, policy access and paymaster. The saved attempt has been retained.`,
    });
    return;
  }
  console.error('App request failed', {
    type: error instanceof Error ? error.name : 'unknown',
  });
  res.status(400).json({
    code: 'REQUEST_FAILED',
    message:
      'The request could not be verified. Reconnect or resume the saved wallet setup.',
  });
});

app.use(express.static(join(process.cwd(), 'dist')));
server.listen(4173, '127.0.0.1', () =>
  console.log(`Swig demo: ${origin}\nSDK configured: ${Boolean(swig)}`),
);
server.on('error', (error) => {
  console.error('Server could not start', error.message);
  process.exitCode = 1;

  state.db.close();
});
