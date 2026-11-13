import { describe, expect, test } from 'bun:test';
import { SwigClient } from '../server/typescript/index.js';
import {
  createPolicyRequest,
  normalizeCreatedPolicy,
  normalizePolicyList,
} from './policy.js';

const userKey = '11111111111111111111111111111111';
const fixture = {
  id: 'policy',
  name: 'Phantom and agent',
  description: '',
  initialUserLabel: 'Phantom owner',
  initialKeySource: 'WALLET_POLICY_KEY_SOURCE_AT_CREATION',
  initialActions: [{ type: 'All' }],
  guardianEnabled: false,
  guardianKeySource: 'WALLET_POLICY_KEY_SOURCE_UNSPECIFIED',
  guardianDelaySeconds: 86400,
  additionalUsers: [
    {
      id: 'agent-role',
      label: 'Application agent',
      authority: { ed25519: { publicKey: userKey } },
      actions: [{ type: 'SolLimit', amount: '42' }],
    },
  ],
};

describe('policy templates', () => {
  test('saves a runtime owner and explicit agent permissions with exact amounts', () => {
    expect(
      createPolicyRequest({
        name: 'Phantom and agent',
        initialUser: { keySource: 'atCreation', label: 'Phantom owner' },
        initialActions: [{ type: 'all' }],
        additionalUsers: [
          {
            label: 'Agent',
            authority: { ed25519: { publicKey: userKey } },
            actions: [
              { type: 'program', programId: userKey },
              { type: 'solLimit', amount: 9007199254740993n },
            ],
          },
        ],
      }),
    ).toEqual({
      name: 'Phantom and agent',
      description: undefined,
      initialUser: {
        keySource: 'WALLET_POLICY_KEY_SOURCE_AT_CREATION',
        label: 'Phantom owner',
        authority: undefined,
      },
      initialActions: [{ type: 'All' }],
      additionalUsers: [
        {
          label: 'Agent',
          authority: { ed25519: { publicKey: userKey } },
          actions: [
            { type: 'Program', programId: userKey },
            { type: 'SolLimit', amount: '9007199254740993' },
          ],
        },
      ],
    });
  });

  test('rejects ambiguous grants and unsafe integer amounts before making a request', () => {
    const base = {
      name: 'Policy',
      initialUser: { keySource: 'atCreation' as const },
      initialActions: [{ type: 'all' as const }],
    };
    expect(() => createPolicyRequest({ ...base, initialActions: [] })).toThrow(
      'Initial actions',
    );
    expect(() =>
      createPolicyRequest({
        ...base,
        additionalUsers: [
          { authority: { ed25519: { publicKey: userKey } }, actions: [] },
        ],
      }),
    ).toThrow('Additional user actions');
    expect(() =>
      createPolicyRequest({
        ...base,
        initialActions: [
          { type: 'solLimit', amount: Number.MAX_SAFE_INTEGER + 1 },
        ],
      }),
    ).toThrow('Invalid policy amount');
    expect(() =>
      createPolicyRequest({
        ...base,
        initialActions: [{ type: 'solLimit', amount: '18446744073709551616' }],
      }),
    ).toThrow('Invalid policy amount');
  });

  test('returns stored role layouts and refuses malformed or incomplete responses', () => {
    expect(normalizePolicyList({ policies: [fixture] })[0]).toMatchObject({
      id: 'policy',
      actions: [{ type: 'All' }],
      additionalUsers: fixture.additionalUsers,
    });
    expect(normalizePolicyList({ policies: [] })).toEqual([]);
    expect(() => normalizePolicyList({})).toThrow('Invalid policy list');
    expect(() =>
      normalizePolicyList({
        policies: [
          { ...fixture, initialKeySource: 'WALLET_POLICY_KEY_SOURCE_STORED' },
        ],
      }),
    ).toThrow('authority is missing');
    expect(() =>
      normalizePolicyList({
        policies: [
          {
            ...fixture,
            additionalUsers: [{ ...fixture.additionalUsers[0], actions: null }],
          },
        ],
      }),
    ).toThrow('Invalid saved policy actions');
    expect(() => normalizeCreatedPolicy({})).toThrow('Invalid policy creation');
  });

  test('uses project API-key routes and never retries policy creation', async () => {
    const requests: Request[] = [];
    const fetcher: typeof fetch = Object.assign(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const request = new Request(input, init);
        requests.push(request);
        expect(request.headers.get('authorization')).toBe('Bearer sk_test');
        expect(new URL(request.url).pathname).toBe('/wallet/api/policies');
        if (request.method === 'GET')
          return Response.json({ policies: [fixture] });
        return Response.json({ message: 'Unavailable' }, { status: 503 });
      },
      { preconnect: fetch.preconnect },
    );
    const client = new SwigClient({
      apiKey: 'sk_test',
      fetch: fetcher,
      retryOptions: { maxRetries: 2, retryDelay: 0 },
    });
    expect((await client.wallets.listPolicies())[0]?.additionalUsers).toEqual(
      fixture.additionalUsers,
    );
    await expect(
      client.wallets.createPolicy({
        name: 'Policy',
        initialUser: { keySource: 'atCreation' },
        initialActions: [{ type: 'all' }],
      }),
    ).rejects.toThrow();
    expect(requests).toHaveLength(2);
  });
});
