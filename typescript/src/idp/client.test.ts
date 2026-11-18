import { Keypair } from '@solana/web3.js';
import { describe, expect, test } from 'bun:test';
import { SwigClient, type EnsureEmailWalletAppInput } from '../index.js';

const input: EnsureEmailWalletAppInput = {
  network: 'devnet',
  callbackUrl: 'http://localhost:4179/auth/callback',
  walletTemplateId: 'wallet-policy',
  agentPublicKey: Keypair.generate().publicKey.toBase58(),
};
const payer = Keypair.generate().publicKey.toBase58();

describe('IdP email app registration', () => {
  test.each([undefined, payer])(
    'registers the exact paymaster choice',
    async (projectPaymasterAddress) => {
      const selection = projectPaymasterAddress
        ? { projectPaymasterAddress }
        : {};
      const expected = {
        callbackUrl: input.callbackUrl,
        walletTemplateId: input.walletTemplateId,
        agentPublicKey: input.agentPublicKey,
        ...selection,
      };
      const sdk = new SwigClient({
        apiKey: 'sk_test',
        fetch: Object.assign(
          async (url: RequestInfo | URL, init?: RequestInit) => {
            const request = new Request(url, init);
            expect(new URL(request.url).pathname).toBe(
              '/identity/api/email-wallet-apps/ensure',
            );
            expect(request.headers.get('authorization')).toBe('Bearer sk_test');
            expect(await request.json()).toEqual({
              ...expected,
              network: 'NETWORK_DEVNET',
            });
            return Response.json({ ...expected, clientId: 'client' });
          },
          { preconnect: fetch.preconnect },
        ),
      });
      expect(
        await sdk.idp.ensureEmailWalletApp({ ...input, ...selection }),
      ).toEqual({ ...expected, clientId: 'client' });
    },
  );
  test.each([undefined, Keypair.generate().publicKey.toBase58()])(
    'rejects a missing or substituted project paymaster',
    async (returned) => {
      const sdk = new SwigClient({
        apiKey: 'sk_test',
        fetch: Object.assign(
          async () =>
            Response.json({
              ...input,
              clientId: 'client',
              projectPaymasterAddress: returned,
            }),
          { preconnect: fetch.preconnect },
        ),
      });
      await expect(
        sdk.idp.ensureEmailWalletApp({
          ...input,
          projectPaymasterAddress: payer,
        }),
      ).rejects.toThrow('does not match');
    },
  );
});
