import { PublicKey } from '@solana/web3.js';
import { createPublicKey, verify } from 'node:crypto';

export function verifyEd25519(
  publicKey: string,
  message: Uint8Array,
  signature: Uint8Array,
): boolean {
  if (signature.length !== 64) return false;
  const key = createPublicKey({
    key: Buffer.concat([
      Buffer.from('302a300506032b6570032100', 'hex'),
      new PublicKey(publicKey).toBuffer(),
    ]),
    format: 'der',
    type: 'spki',
  });
  return verify(null, message, key, signature);
}
