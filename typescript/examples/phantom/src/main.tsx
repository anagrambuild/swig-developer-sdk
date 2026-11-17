import { PhantomWalletAdapter } from '@solana/wallet-adapter-phantom';
import { VersionedTransaction } from '@solana/web3.js';
import bs58 from 'bs58';
import { Buffer } from 'buffer';
import { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { z } from 'zod';
import {
  appConfigSchema,
  challengeSchema,
  flowSchema,
  loginSchema,
  type WalletFlow,
} from './contracts';
import './style.css';

type UiState =
  | { type: 'idle' }
  | { type: 'working'; message: string }
  | { type: 'error'; message: string }
  | { type: 'ready' };

async function responseBody(request: Promise<Response>): Promise<unknown> {
  const response = await request;
  const body: unknown = await response.json();
  if (!response.ok) {
    const failure = z.object({ message: z.string() }).safeParse(body);
    throw new Error(
      failure.success
        ? failure.data.message
        : `Request failed (${response.status}). Try again.`,
    );
  }
  return body;
}

function App() {
  const phantom = useMemo(() => new PhantomWalletAdapter(), []);
  const [config, setConfig] = useState<z.infer<typeof appConfigSchema> | null>(
    null,
  );
  const [ui, setUi] = useState<UiState>({ type: 'idle' });
  const [flow, setFlow] = useState<WalletFlow | null>(null);
  const [owner, setOwner] = useState<string | null>(null);
  const generation = useRef(0);
  const running = useRef(false);
  const currentOwner = useRef<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    void responseBody(fetch('/api/config', { signal: controller.signal }))
      .then((body) => setConfig(appConfigSchema.parse(body)))
      .catch(() => {
        if (!controller.signal.aborted)
          setUi({
            type: 'error',
            message:
              'The local server is unavailable. Start it and reload this page.',
          });
      });
    return () => controller.abort();
  }, []);

  useEffect(() => {
    const invalidate = () => {
      generation.current += 1;
      currentOwner.current = null;
      setOwner(null);
      setFlow(null);
      setUi({ type: 'idle' });
      void fetch('/api/auth/logout', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{}',
      }).catch(() =>
        setUi({
          type: 'error',
          message:
            'Could not end the local session. Reload before reconnecting.',
        }),
      );
    };
    const accountChanged = () => {
      if (
        currentOwner.current &&
        phantom.publicKey?.toBase58() !== currentOwner.current
      )
        invalidate();
    };
    phantom.on('disconnect', invalidate);
    phantom.on('connect', accountChanged);
    return () => {
      phantom.off('disconnect', invalidate);
      phantom.off('connect', accountChanged);
      generation.current += 1;
    };
  }, [phantom]);

  async function connectAndCreate(): Promise<void> {
    if (running.current) return;
    running.current = true;
    const run = ++generation.current;
    try {
      if (phantom.readyState !== 'Installed')
        throw new Error(
          'Install the Phantom browser extension, then reload this page.',
        );
      setUi({ type: 'working', message: 'Connect your Phantom account…' });
      await phantom.connect();
      const publicKey = phantom.publicKey?.toBase58();
      if (!publicKey)
        throw new Error('Phantom did not return a signing account.');
      currentOwner.current = publicKey;
      setOwner(publicKey);
      setUi({
        type: 'working',
        message: 'Approve the sign-in message in Phantom…',
      });
      const challenge = challengeSchema.parse(
        await responseBody(
          fetch('/api/auth/challenge', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ publicKey }),
          }),
        ),
      );
      if (
        generation.current !== run ||
        phantom.publicKey?.toBase58() !== publicKey
      )
        return;
      const signature = await phantom.signMessage(
        new TextEncoder().encode(challenge.message),
      );
      if (
        generation.current !== run ||
        phantom.publicKey?.toBase58() !== publicKey
      )
        return;
      const login = loginSchema.parse(
        await responseBody(
          fetch('/api/auth/session', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ signature: bs58.encode(signature) }),
          }),
        ),
      );
      if (login.owner !== publicKey)
        throw new Error(
          'The signed-in account differs from Phantom. Reconnect.',
        );
      let signedTransaction: string | undefined;
      for (let iteration = 0; iteration < 90; iteration += 1) {
        if (
          generation.current !== run ||
          phantom.publicKey?.toBase58() !== publicKey
        )
          return;
        const next = flowSchema.parse(
          await responseBody(
            fetch('/api/wallet/advance', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify(
                signedTransaction ? { signedTransaction } : {},
              ),
            }),
          ),
        );
        signedTransaction = undefined;
        if (generation.current !== run) return;
        if (
          next.owner !== publicKey ||
          next.agentPublicKey !== config?.agentPublicKey
        )
          throw new Error(
            'The wallet setup returned different authority keys.',
          );
        setFlow(next);
        if (next.state === 'ready') {
          setUi({ type: 'ready' });
          return;
        }
        const messages = {
          reserved: 'Address reserved. Creating your wallet…',
          creating: 'Confirming wallet creation on Devnet…',
          'awaiting-approval':
            'Approve your agent’s spending limits in Phantom…',
          'adding-agent': 'Adding your agent and confirming its access…',
          verifying: 'Checking the wallet’s indexed roles and permissions…',
        };
        setUi({ type: 'working', message: messages[next.state] });
        if (next.state === 'awaiting-approval') {
          if (!next.transaction)
            throw new Error('The agent approval transaction is missing.');
          const transaction = VersionedTransaction.deserialize(
            Buffer.from(next.transaction, 'base64'),
          );
          const originalMessage = Buffer.from(transaction.message.serialize());
          const signed = await phantom.signTransaction(transaction);
          if (
            generation.current !== run ||
            phantom.publicKey?.toBase58() !== publicKey
          )
            return;
          if (
            !Buffer.from(signed.message.serialize()).equals(originalMessage)
          ) {
            throw new Error(
              'Phantom changed the transaction message. Nothing was submitted.',
            );
          }
          signedTransaction = Buffer.from(signed.serialize()).toString(
            'base64',
          );
        } else {
          await new Promise((resolve) => window.setTimeout(resolve, 1500));
        }
      }
      throw new Error(
        'Confirmation or indexing is taking longer than expected. Resume to check the same saved wallet.',
      );
    } catch (error) {
      if (generation.current === run)
        setUi({
          type: 'error',
          message:
            error instanceof Error
              ? error.message
              : 'The wallet request was cancelled. You can resume safely.',
        });
    } finally {
      running.current = false;
    }
  }

  async function disconnect(): Promise<void> {
    generation.current += 1;
    try {
      await responseBody(
        fetch('/api/auth/logout', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: '{}',
        }),
      );
      await phantom.disconnect();
      currentOwner.current = null;
      setOwner(null);
      setFlow(null);
      setUi({ type: 'idle' });
    } catch {
      setUi({
        type: 'error',
        message: 'Could not disconnect. Reload before switching accounts.',
      });
    }
  }

  const busy = ui.type === 'working';
  return (
    <main>
      <header>
        <a className="wordmark" href="/">
          swig<span> / playground</span>
        </a>
        <span className="network">
          <span />
          Devnet
        </span>
      </header>
      <section className="wallet-card" aria-labelledby="title">
        <div className="wallet-mark" aria-hidden="true">
          s<span>↗</span>
        </div>
        <p className="eyebrow">PHANTOM + YOUR AGENT</p>
        <h1 id="title">
          Your wallet.
          <br />
          An agent with limits.
        </h1>
        <p className="intro">
          Sign in with Phantom to create your Swig wallet.
          <br className="desktop" /> You keep full control. Your agent gets a
          daily allowance.
        </p>

        <div className="policy">
          <div>
            <span className="role-icon" aria-hidden="true">
              P
            </span>
            <div>
              <strong>You</strong>
              <p>Full control, including managing roles</p>
            </div>
            <span className="role-tag">Phantom</span>
          </div>
          <div>
            <span className="role-icon agent-icon" aria-hidden="true">
              A
            </span>
            <div>
              <strong>Your agent</strong>
              <p>100 test USDC + 1 SOL per approximately daily window</p>
              <small>
                Any program or venue, including transfers within those limits.
              </small>
            </div>
            <span className="role-tag">Local key</span>
          </div>
        </div>
        <p className="timing">
          Allowances reset every 216,000 slots. Actual timing varies.
        </p>

        {ui.type !== 'ready' && (
          <button
            className="primary"
            type="button"
            disabled={busy || !config}
            onClick={() => void connectAndCreate()}
          >
            {busy
              ? 'Setting up your wallet…'
              : owner
                ? 'Resume wallet setup'
                : 'Connect Phantom'}
            <span aria-hidden="true">↗</span>
          </button>
        )}
        <div
          className={`feedback ${ui.type}`}
          aria-live="polite"
          aria-atomic="true"
        >
          {ui.type === 'working' && (
            <p>
              <span className="spinner" aria-hidden="true" />
              {ui.message}
            </p>
          )}
          {ui.type === 'error' && <p role="alert">{ui.message}</p>}
          {ui.type === 'ready' && (
            <p>✓ Wallet created. Your agent’s access is verified.</p>
          )}
          {ui.type === 'idle' && (
            <p>The app sponsors Devnet fees. Approvals stay in Phantom.</p>
          )}
        </div>

        {flow && (
          <div className="wallet-result">
            <div className="result-label">
              <strong>
                {ui.type === 'ready'
                  ? 'Your Swig wallet'
                  : 'Reserved wallet address'}
              </strong>
              <span>
                {ui.type === 'ready' ? 'Active' : 'Setup in progress'}
              </span>
            </div>
            <p className="address">{flow.walletAddress}</p>
            <a
              href={`https://faucet.solana.com/?cluster=devnet&walletAddress=${encodeURIComponent(flow.walletAddress)}`}
              target="_blank"
              rel="noreferrer"
            >
              Fund devnet wallet ↗
            </a>
          </div>
        )}
        {owner && (
          <div className="account">
            <span>
              Phantom · {owner.slice(0, 6)}…{owner.slice(-6)}
            </span>
            <button
              type="button"
              onClick={() => void disconnect()}
              disabled={busy}
            >
              Disconnect
            </button>
          </div>
        )}
      </section>
      {config && !config.configured && (
        <aside className="setup" aria-label="Developer setup">
          <strong>One developer setup step remains</strong>
          <p>Connect the project using the Swig credential helper.</p>
          <p>
            Start this app with{' '}
            <code>swig-developer-mcp exec -- npm start</code>.
          </p>
          <a
            href="https://dashboard.onswig.com/"
            target="_blank"
            rel="noreferrer"
          >
            Open Developer Portal ↗
          </a>
        </aside>
      )}
      <footer>
        <details>
          <summary>App details</summary>
          <dl>
            <dt>Agent public key</dt>
            <dd>{config?.agentPublicKey || 'Loading…'}</dd>
            <dt>Policy</dt>
            <dd>Phantom + Agent (Devnet)</dd>
            <dt>Creation</dt>
            <dd>Reserved address → wallet activation → agent approval</dd>
          </dl>
          <p>The agent’s private key stays on the developer’s machine.</p>
        </details>
        <a
          className="docs-link"
          href="https://build.onswig.com/developer-sdk/create-wallets"
          target="_blank"
          rel="noreferrer"
        >
          Built with Swig Developer SDK ↗
        </a>
      </footer>
    </main>
  );
}

const root = document.getElementById('root');
if (!root) throw new Error('App root is missing');
createRoot(root).render(<App />);
