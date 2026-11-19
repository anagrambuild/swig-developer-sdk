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
  allowance: z.object({
    token: z.string().regex(/^[1-9]\d*$/),
    sol: z.string().regex(/^[1-9]\d*$/),
    window: z.string().regex(/^[1-9]\d*$/),
  }),
  state: z.enum([
    'reserved',
    'creating',
    'awaiting-approval',
    'adding-agent',
    'verifying',
    'ready',
    'failed',
  ]),
  walletAddress: z.string(),
  configAddress: z.string(),
  owner: z.string(),
  agentPublicKey: z.string(),
  failedSignature: z.string().optional(),
  transaction: z.string().optional(),
});
export type WalletFlow = z.infer<typeof flowSchema>;
