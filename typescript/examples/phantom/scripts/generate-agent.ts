import { mkdir, readFile, writeFile } from 'node:fs/promises';

import { Keypair } from '@solana/web3.js';
import { join } from 'node:path';
import { z } from 'zod';

const directory = join(process.cwd(), '.local');
const secretPath = join(directory, 'agent-keypair.json');
await mkdir(directory, { recursive: true, mode: 0o700 });
let keypair: Keypair;
try {
  const bytes = z
    .array(z.number().int().min(0).max(255))
    .length(64)
    .parse(JSON.parse(await readFile(secretPath, 'utf8')));
  keypair = Keypair.fromSecretKey(Uint8Array.from(bytes));
} catch (error) {
  if (!(error instanceof Error && 'code' in error && error.code === 'ENOENT'))
    throw error;
  keypair = Keypair.generate();
  await writeFile(secretPath, JSON.stringify([...keypair.secretKey]), {
    mode: 0o600,
    flag: 'wx',
  });
}
await writeFile(
  join(directory, 'agent-public-key'),
  keypair.publicKey.toBase58(),
  { mode: 0o600 },
);
console.log('Agent public key:', keypair.publicKey.toBase58());
