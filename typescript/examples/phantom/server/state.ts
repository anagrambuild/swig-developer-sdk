import type { ReservedWalletSetup } from '@swig-wallet/developer-sdk/onboarding';
import { createHash, randomBytes } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { z } from 'zod';
import { verifyEd25519 } from './security';

const operation = z.enum(['create', 'agent']);
const step = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('prepare'), operation }),
  z.object({
    kind: z.literal('sign'),
    transaction: z.string(),
    payer: z.string(),
  }),
  z.object({
    kind: z.enum(['queued', 'submit']),
    operation,
    transaction: z.string(),
    idempotencyKey: z.string(),
  }),
  z.object({ kind: z.literal('confirm'), operation, signature: z.string() }),
  z.object({ kind: z.literal('failed'), operation, signature: z.string() }),
  z.object({ kind: z.literal('verify') }),
  z.object({ kind: z.literal('ready') }),
]);
const recurring = { recurringAmount: z.string(), window: z.string() };
const attemptSchema = z.object({
  owner: z.string(),
  agent: z.string(),
  network: z.literal('devnet'),
  walletAddress: z.string(),
  swigConfigAddress: z.string(),
  policyId: z.string(),
  salt: z.string().regex(/^[0-9a-f]{64}$/),
  step,
  permissions: z.array(
    z.discriminatedUnion('type', [
      z.object({ type: z.literal('programAll') }),
      z.object({ type: z.literal('solRecurringLimit'), ...recurring }),
      z.object({
        type: z.literal('tokenRecurringLimit'),
        mint: z.string(),
        ...recurring,
      }),
    ]),
  ),
});
export class DemoState {
  readonly db: DatabaseSync;

  constructor(path: string) {
    this.db = new DatabaseSync(path);
    this.db.exec(`
      PRAGMA journal_mode=WAL;
      CREATE TABLE IF NOT EXISTS challenges (id TEXT PRIMARY KEY, owner TEXT NOT NULL, message TEXT NOT NULL, expires INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, owner TEXT NOT NULL, expires INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS attempts (owner TEXT PRIMARY KEY, data TEXT NOT NULL);
    `);
  }

  issueChallenge(
    owner: string,
    origin: string,
  ): { id: string; message: string } {
    this.db.prepare('DELETE FROM challenges WHERE expires < ?').run(Date.now());
    const id = randomBytes(32).toString('hex');
    const message = `${new URL(origin).host} wants you to sign in with your Solana account:\n${owner}\n\nSign in to your Devnet Swig wallet.\n\nURI: ${origin}\nVersion: 1\nChain ID: solana:devnet\nNonce: ${id}\nIssued At: ${new Date().toISOString()}`;
    this.db
      .prepare('INSERT INTO challenges VALUES (?, ?, ?, ?)')
      .run(id, owner, message, Date.now() + 300_000);
    return { id, message };
  }

  authenticate(
    id: string,
    signature: Uint8Array,
  ): { token: string; owner: string } {
    // DELETE RETURNING consumes even a rejected proof; the next attempt needs a fresh challenge.
    const row = z
      .object({ owner: z.string(), message: z.string(), expires: z.number() })
      .safeParse(
        this.db
          .prepare(
            'DELETE FROM challenges WHERE id = ? RETURNING owner, message, expires',
          )
          .get(id),
      );
    if (
      !row.success ||
      row.data.expires < Date.now() ||
      !verifyEd25519(row.data.owner, Buffer.from(row.data.message), signature)
    ) {
      throw new Error(
        'Login proof is invalid, expired or already used. Connect Phantom again.',
      );
    }
    const token = randomBytes(32).toString('hex');
    this.db.prepare('DELETE FROM sessions WHERE expires < ?').run(Date.now());
    this.db
      .prepare('INSERT INTO sessions VALUES (?, ?, ?)')
      .run(
        createHash('sha256').update(token).digest('hex'),
        row.data.owner,
        Date.now() + 3_600_000,
      );
    return { token, owner: row.data.owner };
  }

  sessionOwner(token: string): string | null {
    const result = z
      .object({ owner: z.string() })
      .safeParse(
        this.db
          .prepare('SELECT owner FROM sessions WHERE id = ? AND expires > ?')
          .get(createHash('sha256').update(token).digest('hex'), Date.now()),
      );
    return result.success ? result.data.owner : null;
  }

  logout(token: string): void {
    this.db
      .prepare('DELETE FROM sessions WHERE id = ?')
      .run(createHash('sha256').update(token).digest('hex'));
  }

  getAttempt(owner: string): ReservedWalletSetup | null {
    const row = this.db
      .prepare('SELECT data FROM attempts WHERE owner = ?')
      .get(owner);
    if (!row) return null;
    return attemptSchema.parse(JSON.parse(z.string().parse(row.data)));
  }

  saveAttempt(attempt: ReservedWalletSetup): void {
    this.db
      .prepare(
        'INSERT INTO attempts VALUES (?, ?) ON CONFLICT(owner) DO UPDATE SET data = excluded.data',
      )
      .run(attempt.owner, JSON.stringify(attemptSchema.parse(attempt)));
  }
}
