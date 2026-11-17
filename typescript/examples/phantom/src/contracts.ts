import { z } from 'zod';

export const appConfigSchema = z.object({
  configured: z.boolean(),
  agentPublicKey: z.string(),
  policyId: z.string(),
  network: z.literal('devnet'),
});
export const challengeSchema = z.object({ message: z.string() });
export const loginSchema = z.object({ owner: z.string() });
export const flowSchema = z.object({
  state: z.enum([
    'reserved',
    'creating',
    'awaiting-approval',
    'adding-agent',
    'verifying',
    'ready',
  ]),
  walletAddress: z.string(),
  configAddress: z.string(),
  owner: z.string(),
  agentPublicKey: z.string(),
  transaction: z.string().optional(),
});
export type WalletFlow = z.infer<typeof flowSchema>;
