import { VersionedTransaction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import type { WalletFlow } from './contracts';

export async function signReviewedAgentGrant(
  reviewed: WalletFlow | undefined,
  next: WalletFlow,
  sign: (transaction: VersionedTransaction) => Promise<VersionedTransaction>,
): Promise<string | undefined> {
  if (next.state !== 'awaiting-approval' || !next.transaction)
    throw new Error('The agent approval transaction is missing.');
  if (
    reviewed?.state !== 'awaiting-approval' ||
    reviewed.owner !== next.owner ||
    reviewed.walletAddress !== next.walletAddress ||
    reviewed.configAddress !== next.configAddress ||
    reviewed.agentPublicKey !== next.agentPublicKey ||
    reviewed.transaction !== next.transaction ||
    reviewed.allowance.sol !== next.allowance.sol ||
    reviewed.allowance.token !== next.allowance.token ||
    reviewed.allowance.window !== next.allowance.window
  )
    return undefined;
  const transaction = VersionedTransaction.deserialize(
    Buffer.from(next.transaction, 'base64'),
  );
  const originalMessage = Buffer.from(transaction.message.serialize());
  const signed = await sign(transaction);
  if (!Buffer.from(signed.message.serialize()).equals(originalMessage))
    throw new Error(
      'Phantom changed the transaction message. Nothing was submitted.',
    );
  return Buffer.from(signed.serialize()).toString('base64');
}
