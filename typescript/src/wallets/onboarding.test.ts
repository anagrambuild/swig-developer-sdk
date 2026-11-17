import {
  Connection,
  Keypair,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import { expect, spyOn, test } from 'bun:test';
import { SwigClient } from '../server/typescript/index.js';
import type { WalletTemplate } from '../types/policy.js';
import {
  advanceReservedWalletSetup,
  startReservedWalletSetup,
} from './onboarding.js';

const owner = Keypair.generate(),
  agent = Keypair.generate(),
  payer = Keypair.generate();
const key = owner.publicKey.toBase58(),
  agentKey = agent.publicKey.toBase58(),
  payerKey = payer.publicKey.toBase58();
const wallet = {
  network: 'devnet' as const,
  swigConfigAddress: Keypair.generate().publicKey.toBase58(),
  walletAddress: Keypair.generate().publicKey.toBase58(),
};
const template: WalletTemplate = {
  id: 'policy',
  name: 'policy',
  description: '',
  initialRole: {
    id: 'root',
    name: 'Owner',
    keySource: 'atCreation',
    permissions: [{ type: 'all' }],
  },
  additionalRoles: [
    {
      id: 'agent',
      name: 'Agent',
      keySource: 'atCreation',
      permissions: [
        { type: 'solRecurringLimit', recurringAmount: '10', window: '20' },
      ],
    },
  ],
};
const args = {
  network: 'devnet' as const,
  owner: key,
  agent: agentKey,
  salt: new Uint8Array(32),
  template,
};
function transaction(from: Keypair) {
  return new VersionedTransaction(
    new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: Keypair.generate().publicKey.toBase58(),
      instructions: [
        SystemProgram.transfer({
          fromPubkey: from.publicKey,
          toPubkey: agent.publicKey,
          lamports: 1,
        }),
      ],
    }).compileToV0Message(),
  );
}
test('reserved activation rejects unsupported or colliding authorities before lookup', async () => {
  const client = new SwigClient({ apiKey: 'test' });
  const lookup = spyOn(client.wallets, 'reserveAddress');
  await expect(
    startReservedWalletSetup(client, {
      ...args,
      template: { ...template, rentClaimer: key },
    }),
  ).rejects.toThrow('at-creation');
  await expect(
    startReservedWalletSetup(client, { ...args, agent: key }),
  ).rejects.toThrow('distinct');
  expect(lookup).not.toHaveBeenCalled();
});

test('resumes the exact sponsorship, binds Phantom approval, and verifies capped roles', async () => {
  const client = new SwigClient({ apiKey: 'test' });
  const rpc = new Connection('http://localhost:8899');
  const creation = {
    transaction: Buffer.from(transaction(payer).serialize()).toString('base64'),
    signatureRequests: [],
  };
  const role = transaction(owner);
  const handle = client.wallets.use(wallet.swigConfigAddress);
  spyOn(client.wallets, 'use').mockReturnValue(handle);
  spyOn(client.wallets, 'reserveAddress').mockResolvedValue({
    network: 'devnet',
    wallet,
  });
  spyOn(client.wallets, 'createReserved').mockResolvedValue({
    network: 'devnet',
    wallet,
    creationTransaction: creation,
    transactions: [creation],
    feePayerOnlyTransactions: [creation],
    clientAuthorityTransactions: [],
    operatorSignedTransactions: [],
  });
  spyOn(client.paymaster, 'getBalance').mockResolvedValue({
    configured: true,
    address: payerKey,
    balanceLamports: '1',
    balanceSol: 0.000000001,
    id: 'p',
    kind: 'api',
    label: 'p',
  });
  spyOn(handle.roles, 'add').mockResolvedValue({
    transaction: Buffer.from(role.serialize()).toString('base64'),
    signatureRequests: [],
  });
  const sponsor = spyOn(client.transactions, 'sponsor').mockResolvedValue({
    signature: 'receipt',
    requestId: 'request',
    spentByPaymaster: '1',
  });
  spyOn(rpc, 'getSignatureStatuses').mockResolvedValue({
    context: { slot: 10 },
    value: [
      {
        slot: 10,
        confirmations: 2,
        confirmationStatus: 'confirmed',
        err: null,
      },
    ],
  });
  const blockhash = spyOn(rpc, 'isBlockhashValid').mockResolvedValue({
    context: { slot: 10 },
    value: true,
  });
  const indexed = [
    {
      roleId: 0,
      authorityType: 1,
      authorityValue: key,
      actions: [{ actionIndex: 0, actionCode: 7, actionData: {} }],
    },
    {
      roleId: 1,
      authorityType: 1,
      authorityValue: agentKey,
      actions: [
        {
          actionIndex: 0,
          actionCode: 2,
          actionData: { recurring_amount: '10', window: '20' },
        },
      ],
    },
  ];
  const roles = spyOn(handle, 'listRoles').mockResolvedValue({
    ...wallet,
    roles: indexed,
  });
  let setup = await startReservedWalletSetup(client, args);
  setup = await advanceReservedWalletSetup(client, rpc, setup);
  expect(setup.step.kind).toBe('submit');
  const confirm = await advanceReservedWalletSetup(client, rpc, setup);
  await advanceReservedWalletSetup(client, rpc, structuredClone(setup));
  expect(sponsor.mock.calls[0]).toEqual(sponsor.mock.calls[1]);
  setup = await advanceReservedWalletSetup(client, rpc, confirm);
  setup = await advanceReservedWalletSetup(client, rpc, setup);
  expect(setup.step.kind).toBe('sign');
  await expect(
    advanceReservedWalletSetup(
      client,
      rpc,
      setup,
      Buffer.from(role.serialize()).toString('base64'),
    ),
  ).rejects.toThrow('signature');
  const other = transaction(owner);
  other.sign([owner]);
  await expect(
    advanceReservedWalletSetup(
      client,
      rpc,
      setup,
      Buffer.from(other.serialize()).toString('base64'),
    ),
  ).rejects.toThrow('signature');
  blockhash.mockResolvedValueOnce({ context: { slot: 11 }, value: false });
  expect((await advanceReservedWalletSetup(client, rpc, setup)).step).toEqual({
    kind: 'prepare',
    operation: 'agent',
  });
  role.sign([owner]);
  setup = await advanceReservedWalletSetup(
    client,
    rpc,
    setup,
    Buffer.from(role.serialize()).toString('base64'),
  );
  setup = await advanceReservedWalletSetup(client, rpc, setup);
  setup = await advanceReservedWalletSetup(client, rpc, setup);
  expect(setup.step.kind).toBe('verify');
  roles.mockResolvedValueOnce({
    ...wallet,
    roles: [{ ...indexed[0], authorityValue: agentKey }],
  });
  await expect(advanceReservedWalletSetup(client, rpc, setup)).rejects.toThrow(
    'approved policy',
  );
  roles.mockResolvedValueOnce({
    ...wallet,
    roles: [
      indexed[0],
      {
        ...indexed[1],
        actions: [{ actionIndex: 0, actionCode: 7, actionData: {} }],
      },
    ],
  });
  await expect(advanceReservedWalletSetup(client, rpc, setup)).rejects.toThrow(
    'permissions',
  );
  expect((await advanceReservedWalletSetup(client, rpc, setup)).step.kind).toBe(
    'ready',
  );
});
