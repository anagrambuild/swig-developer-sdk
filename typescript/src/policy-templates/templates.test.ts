import type {
  RoleTemplateInput,
  WalletTemplateInput,
  WalletTemplateSetup,
} from '@swig-wallet/developer-sdk';
import { expect, test } from 'bun:test';
import { SwigClient } from '../server/typescript/index.js';
import {
  roleRequest,
  roleTemplate,
  templateSetup,
  walletRequest,
  walletTemplate,
} from './codec.js';
const owner = {
  id: 'owner',
  name: 'User key',
  keySource: 'WALLET_POLICY_KEY_SOURCE_AT_CREATION',
  permissions: [{ type: 'All' }],
};
const agent = {
  id: 'agent',
  name: 'Helper',
  keySource: 'WALLET_POLICY_KEY_SOURCE_STORED',
  authority: { ed25519: { publicKey: '11111111111111111111111111111111' } },
  permissions: [{ type: 'SolLimit', amount: '9007199254740993' }],
};
const template = {
  id: 'wallet',
  name: 'Wallet',
  initialRole: owner,
  additionalRoles: [agent],
};

test('public template inputs preserve explicit grants and exact amounts', () => {
  const input: RoleTemplateInput = {
    name: 'Trader',
    permissions: [{ type: 'solLimit', amount: 9007199254740993n }],
  };
  const wallet: WalletTemplateInput = {
    name: 'Wallet',
    initialRoleTemplateId: 'owner',
    additionalRoleTemplateIds: ['agent'],
  };
  expect(roleRequest(input).permissions).toEqual([
    { type: 'SolLimit', amount: '9007199254740993' },
  ]);
  expect(walletRequest(wallet).additionalRoleTemplateIds).toEqual(['agent']);
  expect(roleTemplate(agent).permissions).toEqual([
    { type: 'solLimit', amount: '9007199254740993' },
  ]);
});

test('rejects invalid grants, role references, stored authority, and unsigned integer loss', () => {
  expect(() => roleRequest({ name: 'Role', permissions: [] })).toThrow();
  expect(() =>
    roleRequest({
      name: 'Role',
      permissions: [{ type: 'solLimit', amount: Number.MAX_SAFE_INTEGER + 1 }],
    }),
  ).toThrow();
  expect(() =>
    walletRequest({
      name: 'Wallet',
      initialRoleTemplateId: 'owner',
      additionalRoleTemplateIds: ['owner'],
    }),
  ).toThrow();
  expect(() => roleTemplate({ ...agent, authority: undefined })).toThrow();
  expect(() =>
    roleTemplate({
      ...agent,
      permissions: [{ type: 'SolLimit', amount: '18446744073709551616' }],
    }),
  ).toThrow();
  expect(() =>
    walletTemplate({ ...template, additionalRoles: [owner] }),
  ).toThrow();
});

test('requires complete correlated role bindings for pending setup', () => {
  const bindings = [
    { roleTemplateId: 'owner', authority: agent.authority },
    { roleTemplateId: 'agent', authority: agent.authority },
  ];
  const setup: WalletTemplateSetup = templateSetup({
    template,
    authorities: bindings,
  });
  expect(setup.template.initialRole.keySource).toBe('atCreation');
  expect(() =>
    templateSetup({ template, authorities: bindings.slice(0, 1) }),
  ).toThrow();
  expect(() =>
    templateSetup({ template, authorities: [bindings[0], bindings[0]] }),
  ).toThrow();
});

test('template clients use normal API-key authentication and do not retry writes', async () => {
  const requests: Request[] = [];
  const fetcher: typeof fetch = Object.assign(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const request = new Request(input, init);
      requests.push(request);
      expect(request.headers.get('authorization')).toBe('Bearer sk_test');
      if (request.method === 'GET')
        return Response.json({ walletTemplates: [template] });
      return Response.json({ message: 'unavailable' }, { status: 503 });
    },
    { preconnect: fetch.preconnect },
  );
  const client = new SwigClient({
    apiKey: 'sk_test',
    fetch: fetcher,
    retryOptions: { maxRetries: 2, retryDelay: 0 },
  });
  expect((await client.walletTemplates.list())[0]?.additionalRoles[0]?.id).toBe(
    'agent',
  );
  await expect(
    client.roleTemplates.create({
      name: 'Role',
      permissions: [{ type: 'all' }],
    }),
  ).rejects.toThrow();
  await expect(
    client.roleTemplates.update('role/id', {
      name: 'Role',
      permissions: [{ type: 'all' }],
    }),
  ).rejects.toThrow();
  expect(
    requests.map((request) => [request.method, new URL(request.url).pathname]),
  ).toEqual([
    ['GET', '/wallet/api/wallet-templates'],
    ['POST', '/wallet/api/role-templates'],
    ['PATCH', '/wallet/api/role-templates/role%2Fid'],
  ]);
});

test('rejects unknown recovery modes before transport and unsupported resolved guardians', () => {
  for (const keySource of [undefined, 'typo', 0]) {
    expect(() =>
      Reflect.apply(walletRequest, undefined, [
        {
          name: 'Wallet',
          initialRoleTemplateId: 'owner',
          recovery: { keySource, delaySeconds: 86400 },
        },
      ]),
    ).toThrow('Invalid recovery key source');
  }
  for (const recovery of [
    { keySource: 'atCreation' as const, delaySeconds: 86400 },
    {
      keySource: 'stored' as const,
      authority: agent.authority,
      delaySeconds: 86400,
    },
  ]) {
    expect(
      walletRequest({
        name: 'Wallet',
        initialRoleTemplateId: 'owner',
        recovery,
      }).recovery?.delaySeconds,
    ).toBe(86400);
  }
  const setup = {
    template: {
      ...template,
      recovery: {
        keySource: 'WALLET_POLICY_KEY_SOURCE_AT_CREATION',
        delaySeconds: 86400,
      },
    },
    authorities: [
      { roleTemplateId: 'owner', authority: agent.authority },
      { roleTemplateId: 'agent', authority: agent.authority },
    ],
  };
  expect(() =>
    templateSetup({
      ...setup,
      guardianAuthority: { secp256r1: { publicKey: 'key' } },
    }),
  ).toThrow('Guardian must be Ed25519');
  expect(
    templateSetup({ ...setup, guardianAuthority: agent.authority })
      .guardianAuthority,
  ).toEqual(agent.authority);
});
