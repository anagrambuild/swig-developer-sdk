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

describe('project identity providers', () => {
  const email = {
    providerKey: 'email',
    label: 'Email',
    enabled: true,
    eligible: true,
  };
  function clientWithResponse(
    body: unknown,
    expectedMethod: string,
    expectedPath: string,
  ) {
    return new SwigClient({
      apiKey: 'sk_project',
      fetch: Object.assign(
        async (url: RequestInfo | URL, init?: RequestInit) => {
          const req = new Request(url, init);
          expect(req.method).toBe(expectedMethod);
          expect(new URL(req.url).pathname).toBe(expectedPath);
          expect(req.headers.get('authorization')).toBe('Bearer sk_project');
          if (req.method === 'POST') expect(await req.json()).toEqual({});
          return Response.json(body);
        },
        { preconnect: fetch.preconnect },
      ),
    });
  }
  test('lists active and inactive providers for the authenticated project', async () => {
    const catalog = [
      email,
      { ...email, providerKey: 'google', enabled: false, eligible: false },
    ];
    expect(
      await clientWithResponse(
        { providers: catalog },
        'GET',
        '/identity/api/providers',
      ).idp.listProviders(),
    ).toEqual(catalog);
    await expect(
      clientWithResponse(
        { providers: [null] },
        'GET',
        '/identity/api/providers',
      ).idp.listProviders(),
    ).rejects.toThrow('Invalid project identity provider');
  });
  test('enables the selected provider and rejects a substituted provider', async () => {
    const path = '/identity/api/providers/email/enable';
    expect(
      await clientWithResponse(
        { provider: email },
        'POST',
        path,
      ).idp.enableProvider('email'),
    ).toEqual(email);
    const client = clientWithResponse(
      { provider: { ...email, providerKey: 'google' } },
      'POST',
      path,
    );
    await expect(client.idp.enableProvider('email')).rejects.toThrow(
      'did not confirm',
    );
    await expect(client.idp.enableProvider('../other')).rejects.toThrow(
      'Invalid identity provider key',
    );
  });
});
