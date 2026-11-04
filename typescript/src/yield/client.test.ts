import { describe, expect, test } from 'bun:test';

import { SwigDeveloperSdkError } from '../core/errors.js';
import { SwigClient } from '../server/typescript/index.js';

type CapturedRequest = { url: string; method: string; body?: any };

function client(
  handler: (request: CapturedRequest) => unknown,
  network?: 'devnet' | 'mainnet',
) {
  const calls: CapturedRequest[] = [];
  const swig = new SwigClient({
    apiKey: 'sk_test',
    baseUrl: 'http://localhost:8080',
    ...(network ? { network } : {}),
    fetch: (async (input: RequestInfo | URL, init?: RequestInit) => {
      const request = new Request(input, init);
      const text = await request.text();
      const captured = {
        url: request.url,
        method: request.method,
        ...(text ? { body: JSON.parse(text) } : {}),
      };
      calls.push(captured);
      return Response.json(handler(captured));
    }) as typeof fetch,
  });
  return { swig, calls };
}

const CONFIG = 'Conf1g11111111111111111111111111111111111111';
const PAYER = 'Payer111111111111111111111111111111111111111';
const AUTHORITY = {
  ed25519: { publicKey: 'Auth1111111111111111111111111111111111111111' },
};
const VOTE = 'Vote111111111111111111111111111111111111111';
const POSITION = 'Pos11111111111111111111111111111111111111111';
const PREPARED = {
  transaction: 'AQID',
  expiresAt: '2026-10-22T00:00:00Z',
  kind: 'PREPARED_TRANSACTION_KIND_YIELD_DEPOSIT',
};

describe('YieldClient', () => {
  test('prepares deposits under the wallet-scoped path with string amounts', async () => {
    const { swig, calls } = client(
      () => ({
        transaction: PREPARED,
        positionAddress: POSITION,
        validatorVoteAddress: VOTE,
        delegatedLamports: '1000000000',
        rentLamports: '2282880',
      }),
      'devnet',
    );
    const deposit = await swig.yield.deposit({
      swigConfigAddress: CONFIG,
      feePayer: PAYER,
      requesterAuthority: AUTHORITY,
      roleId: 3,
      amount: 1_000_000_000n,
      validatorVoteAddress: VOTE,
      positionSeed: 'deposit-1',
    });
    expect(calls[0]!.method).toBe('POST');
    expect(calls[0]!.url).toBe(
      `http://localhost:8080/api/yield/${CONFIG}/deposit/prepare`,
    );
    expect(calls[0]!.body).toEqual({
      wallet: {
        network: 'NETWORK_DEVNET',
        swigConfigAddress: CONFIG,
        feePayer: PAYER,
        roleId: 3,
        requesterAuthority: AUTHORITY,
      },
      lamports: '1000000000',
      validatorVoteAddress: VOTE,
      positionSeed: 'deposit-1',
    });
    expect(deposit.positionAddress).toBe(POSITION);
    expect(deposit.rentLamports).toBe('2282880');
    expect(deposit.transaction.transaction).toBe('AQID');
  });

  test('encodes full and partial exits as the proto oneof', async () => {
    const { swig, calls } = client(() => ({
      transaction: PREPARED,
      exiting_position_address: POSITION,
      deactivating_lamports: 3,
      additional_rent_lamports: '0',
    }));
    const base = {
      swigConfigAddress: CONFIG,
      network: 'mainnet' as const,
      feePayer: PAYER,
      requesterAuthority: AUTHORITY,
      positionAddress: POSITION,
    };
    const all = await swig.yield.exit({ ...base, amount: 'all' });
    await swig.yield.exit({
      ...base,
      amount: { lamports: 5, positionSeed: 'exit-1' },
    });
    expect(calls[0]!.body.all).toEqual({});
    expect(calls[0]!.body.partial).toBeUndefined();
    expect(calls[0]!.body.wallet.network).toBe('NETWORK_MAINNET');
    expect(calls[0]!.body.wallet.roleId).toBeUndefined();
    expect(calls[1]!.body.partial).toEqual({
      lamports: '5',
      positionSeed: 'exit-1',
    });
    expect(all.exitingPositionAddress).toBe(POSITION);
    expect(all.deactivatingLamports).toBe('3');
  });

  test('prepares withdrawals back to the wallet', async () => {
    const { swig, calls } = client(
      () => ({
        transaction: PREPARED,
        withdrawalLamports: '5000000000',
        destinationWalletAddress: 'Wallet',
      }),
      'devnet',
    );
    const withdrawal = await swig.yield.withdraw({
      swigConfigAddress: CONFIG,
      feePayer: PAYER,
      requesterAuthority: AUTHORITY,
      positionAddress: POSITION,
    });
    expect(calls[0]!.url).toEndWith(`/api/yield/${CONFIG}/withdraw/prepare`);
    expect(calls[0]!.body.positionAddress).toBe(POSITION);
    expect(withdrawal).toMatchObject({
      withdrawalLamports: '5000000000',
      destinationWalletAddress: 'Wallet',
    });
  });

  test('decodes reused and newly prepared yield roles', async () => {
    let reply: unknown = {
      existing: {
        roleId: 2,
        actions: [
          { program: { programId: '11111111111111111111111111111111' } },
          { solLimit: { amount: '9' } },
          { stake_all: {} },
          { stakeRecurringLimit: { recurringAmount: '5', window: '100' } },
        ],
      },
    };
    const { swig, calls } = client(() => reply, 'devnet');
    const args = {
      swigConfigAddress: CONFIG,
      feePayer: PAYER,
      yieldAuthority: AUTHORITY,
      validatorVoteAddress: VOTE,
      maxDeposit: 5_000_000_000,
      roleManagerAuthority: { secp256r1: { publicKey: '02ab' } },
    };
    const existing = await swig.yield.prepareRole(args);
    expect(calls[0]!.url).toEndWith(`/api/yield/${CONFIG}/role/prepare`);
    expect(calls[0]!.body).toEqual({
      network: 'NETWORK_DEVNET',
      swigConfigAddress: CONFIG,
      feePayer: PAYER,
      yieldAuthority: AUTHORITY,
      validatorVoteAddress: VOTE,
      maxDepositLamports: '5000000000',
      roleManagerAuthority: { secp256r1: { publicKey: '02ab' } },
    });
    expect(existing).toEqual({
      type: 'existing',
      roleId: 2,
      actions: [
        { type: 'program', programId: '11111111111111111111111111111111' },
        { type: 'solLimit', amount: '9' },
        { type: 'stakeAll' },
        { type: 'stakeRecurringLimit', recurringAmount: '5', window: '100' },
      ],
    });

    reply = {
      create: {
        transaction: PREPARED,
        yieldAuthority: AUTHORITY,
        actions: [{ stakeAll: {} }],
      },
    };
    const created = await swig.yield.prepareRole(args);
    expect(created.type).toBe('create');
    if (created.type === 'create') {
      expect(created.transaction.transaction).toBe('AQID');
      expect(created.actions).toEqual([{ type: 'stakeAll' }]);
    }

    reply = {};
    await expect(swig.yield.prepareRole(args)).rejects.toThrow(
      'neither existing nor create',
    );
  });

  test('queries positions, summary, rewards, options and estimates with GET query strings', async () => {
    const { swig, calls } = client((request) => {
      const path = new URL(request.url).pathname;
      if (path.endsWith('/positions')) {
        return {
          positions: [
            {
              swig_config_address: CONFIG,
              walletAddress: 'Wallet',
              positionAddress: POSITION,
              state: 'YIELD_POSITION_STATE_ACTIVE',
              balanceLamports: '12',
              rentLamports: '2',
              positionLamports: '10',
              withdrawableLamports: '0',
              earningLamports: '10',
            },
            {
              swigConfigAddress: CONFIG,
              walletAddress: 'Wallet',
              positionAddress: 'Other',
              state: 4,
            },
          ],
          nextPageToken: 'next',
          lastUpdatedAt: '2026-10-22T00:00:00Z',
        };
      }
      if (path.endsWith('/summary'))
        return {
          totalPositionLamports: '10',
          withdrawable_lamports: '0',
          recordedRewardsLamports: '1',
        };
      if (path.endsWith('/rewards'))
        return {
          rewards: [
            {
              swigConfigAddress: CONFIG,
              positionAddress: POSITION,
              amountLamports: '7',
              earnedAt: '2026-10-21T00:00:00Z',
            },
          ],
        };
      if (path.endsWith('/options')) {
        return {
          options: [
            {
              voteAddress: VOTE,
              name: 'Anagram',
              acceptsDeposits: true,
              rate: {
                validatorVoteAddress: VOTE,
                estimatedNetAprBps: 665,
                commissionBps: 500,
                lastUpdatedAt: '2026-10-22T00:00:00Z',
              },
            },
          ],
        };
      }
      return {
        projection: {
          principalLamports: '100',
          horizonDays: 30,
          estimatedRewardsLamports: '1',
          estimatedTotalLamports: '101',
        },
      };
    }, 'devnet');
    const wallet = swig.yield.forWallet({ swigConfigAddress: CONFIG });

    const positions = await wallet.listPositions({
      pageSize: 2,
      pageToken: 'abc',
    });
    expect(calls[0]!.method).toBe('GET');
    expect(calls[0]!.url).toBe(
      `http://localhost:8080/api/yield/${CONFIG}/positions?network=NETWORK_DEVNET&pageSize=2&pageToken=abc`,
    );
    expect(positions.nextPageToken).toBe('next');
    expect(positions.positions.map((p) => p.state)).toEqual([
      'active',
      'inactive',
    ]);
    expect(positions.positions[0]).toMatchObject({
      swigConfigAddress: CONFIG,
      positionLamports: '10',
    });
    expect(positions.positions[1]!.balanceLamports).toBe('0');

    expect(await wallet.getSummary()).toEqual({
      totalPositionLamports: '10',
      withdrawableLamports: '0',
      recordedRewardsLamports: '1',
    });
    const rewards = await wallet.listRewards();
    expect(calls[2]!.url).toEndWith('/rewards?network=NETWORK_DEVNET');
    expect(rewards.rewards[0]).toMatchObject({
      amountLamports: '7',
      earnedAt: '2026-10-21T00:00:00Z',
    });
    expect(rewards.nextPageToken).toBeUndefined();

    const options = await wallet.listOptions();
    expect(options[0]).toMatchObject({
      name: 'Anagram',
      acceptsDeposits: true,
      rate: { estimatedNetAprBps: 665 },
    });

    const projection = await wallet.estimate({
      validatorVoteAddress: VOTE,
      principal: 100n,
      horizonDays: 30,
    });
    expect(new URL(calls[4]!.url).searchParams.get('principalLamports')).toBe(
      '100',
    );
    expect(new URL(calls[4]!.url).searchParams.get('horizonDays')).toBe('30');
    expect(projection.estimatedTotalLamports).toBe('101');
  });

  test('validates inputs before sending requests', async () => {
    const { swig, calls } = client(() => ({}));
    await expect(
      swig.yield.getSummary({ swigConfigAddress: CONFIG }),
    ).rejects.toThrow('network is required');
    await expect(
      swig.yield.listPositions({
        swigConfigAddress: CONFIG,
        network: 'devnet',
        pageSize: 201,
      }),
    ).rejects.toThrow('pageSize');
    await expect(
      swig.yield.estimate({
        swigConfigAddress: CONFIG,
        network: 'devnet',
        validatorVoteAddress: VOTE,
        principal: 1,
        horizonDays: 366,
      }),
    ).rejects.toThrow('horizonDays');
    await expect(
      swig.yield.listOptions({ swigConfigAddress: '', network: 'devnet' }),
    ).rejects.toThrow('swigConfigAddress is required');
    expect(calls).toHaveLength(0);
  });

  test('rejects prepare responses without a transaction', async () => {
    const { swig } = client(
      () => ({ positionAddress: POSITION, validatorVoteAddress: VOTE }),
      'devnet',
    );
    await expect(
      swig.yield.deposit({
        swigConfigAddress: CONFIG,
        feePayer: PAYER,
        requesterAuthority: AUTHORITY,
        amount: 1,
        validatorVoteAddress: VOTE,
        positionSeed: 's',
      }),
    ).rejects.toThrow('missing transaction');
  });
});

describe('Yield response contracts', () => {
  const wallet = { swigConfigAddress: CONFIG };
  const roleArgs = {
    ...wallet,
    feePayer: PAYER,
    yieldAuthority: AUTHORITY,
    validatorVoteAddress: VOTE,
    maxDeposit: 1,
  };

  test.each(['camel', 'snake'])(
    'round-trips a %s ParticipantSet role authority into a deposit',
    async (casing) => {
      const returnedAuthority =
        casing === 'camel'
          ? {
              participantSet: {
                participantSetAddress: 'set-address',
                roleId: 0,
              },
            }
          : {
              participant_set: {
                participant_set_address: 'set-address',
                role_id: 0,
              },
            };
      const { swig, calls } = client(
        ({ url }) =>
          url.endsWith('/role/prepare')
            ? {
                create: {
                  transaction: PREPARED,
                  yieldAuthority: returnedAuthority,
                },
              }
            : {
                transaction: PREPARED,
                positionAddress: POSITION,
                validatorVoteAddress: VOTE,
              },
        'devnet',
      );
      const role = await swig.yield.prepareRole(roleArgs);
      expect(role.type).toBe('create');
      if (role.type !== 'create') throw new Error('Expected role creation');
      expect(role.yieldAuthority).toEqual({
        participantSet: { address: 'set-address', roleId: 0 },
      });
      await swig.yield.deposit({
        ...wallet,
        feePayer: PAYER,
        requesterAuthority: role.yieldAuthority,
        validatorVoteAddress: VOTE,
        amount: 1,
        positionSeed: 'deposit',
      });
      expect(calls[1]?.body.wallet.requesterAuthority).toEqual({
        participantSet: { participantSetAddress: 'set-address', roleId: 0 },
      });
    },
  );

  test('rejects malformed collections and elements with SDK response errors', async () => {
    for (const [method, field] of [
      ['listOptions', 'options'],
      ['listPositions', 'positions'],
      ['listRewards', 'rewards'],
    ] as const) {
      for (const value of [{}, 'not-a-list', 1, false, [null], [false]]) {
        const { swig } = client(() => ({ [field]: value }), 'devnet');
        await expect(swig.yield[method](wallet)).rejects.toMatchObject({
          code: 'INVALID_RESPONSE',
          statusCode: 502,
        });
      }
    }
    const { swig } = client(() => [], 'devnet');
    await expect(swig.yield.getSummary(wallet)).rejects.toBeInstanceOf(
      SwigDeveloperSdkError,
    );
  });

  test('requires every position, reward and validator identity', async () => {
    const validPosition = {
      swigConfigAddress: CONFIG,
      walletAddress: 'Wallet',
      positionAddress: POSITION,
    };
    for (const field of Object.keys(validPosition)) {
      for (const bad of [undefined, null, '', 123, {}]) {
        const { swig } = client(
          () => ({ positions: [{ ...validPosition, [field]: bad }] }),
          'devnet',
        );
        await expect(swig.yield.listPositions(wallet)).rejects.toMatchObject({
          code: 'INVALID_RESPONSE',
        });
      }
    }
    for (const reward of [
      { positionAddress: POSITION },
      { swigConfigAddress: CONFIG },
    ]) {
      const { swig } = client(() => ({ rewards: [reward] }), 'devnet');
      await expect(swig.yield.listRewards(wallet)).rejects.toMatchObject({
        code: 'INVALID_RESPONSE',
      });
    }
    const { swig } = client(
      () => ({ options: [{ acceptsDeposits: true }] }),
      'devnet',
    );
    await expect(swig.yield.listOptions(wallet)).rejects.toMatchObject({
      code: 'INVALID_RESPONSE',
    });
  });

  test('rejects malformed numbers and ambiguous role variants', async () => {
    for (const amount of [
      true,
      {},
      [],
      'abc',
      '-1',
      -1,
      1.5,
      Number.MAX_SAFE_INTEGER + 1,
      '18446744073709551616',
    ]) {
      const { swig } = client(
        () => ({ totalPositionLamports: amount }),
        'devnet',
      );
      await expect(swig.yield.getSummary(wallet)).rejects.toMatchObject({
        code: 'INVALID_RESPONSE',
      });
    }
    for (const reply of [
      { existing: { roleId: true } },
      { existing: { roleId: 1, actions: {} } },
      { existing: { roleId: 1, actions: [{ stakeAll: {}, all: {} }] } },
      {
        existing: { roleId: 1 },
        create: { transaction: PREPARED, yieldAuthority: AUTHORITY },
      },
      {
        create: {
          transaction: PREPARED,
          yieldAuthority: { participantSet: {} },
        },
      },
    ]) {
      const { swig } = client(() => reply, 'devnet');
      await expect(swig.yield.prepareRole(roleArgs)).rejects.toMatchObject({
        code: 'INVALID_RESPONSE',
      });
    }
  });

  test('preserves empty protobuf defaults and exact uint64 strings', async () => {
    const { swig } = client(() => ({}), 'devnet');
    expect(await swig.yield.listOptions(wallet)).toEqual([]);
    expect((await swig.yield.listPositions(wallet)).positions).toEqual([]);
    expect((await swig.yield.listRewards(wallet)).rewards).toEqual([]);
    expect((await swig.yield.getSummary(wallet)).totalPositionLamports).toBe(
      '0',
    );
    const { swig: large } = client(
      () => ({ totalPositionLamports: '18446744073709551615' }),
      'devnet',
    );
    expect((await large.yield.getSummary(wallet)).totalPositionLamports).toBe(
      '18446744073709551615',
    );
  });

  test.each([
    ['yield-deposit', 'PREPARED_TRANSACTION_KIND_YIELD_DEPOSIT', 7],
    ['yield-exit', 'PREPARED_TRANSACTION_KIND_YIELD_EXIT', 8],
    ['yield-withdrawal', 'PREPARED_TRANSACTION_KIND_YIELD_WITHDRAWAL', 9],
  ] as const)(
    'preserves %s through preparation and approval compilation',
    async (kind, proto, number) => {
      for (const wire of [kind, proto, number]) {
        const { swig, calls } = client(
          ({ url }) =>
            url.endsWith('/compile')
              ? {
                  transaction: { ...PREPARED, kind: wire },
                  authorizationExpirationSlot: '100',
                }
              : {
                  transaction: { ...PREPARED, kind: wire },
                  positionAddress: POSITION,
                  validatorVoteAddress: VOTE,
                  exitingPositionAddress: POSITION,
                  destinationWalletAddress: 'Wallet',
                },
          'devnet',
        );
        const base = {
          ...wallet,
          feePayer: PAYER,
          requesterAuthority: AUTHORITY,
        };
        const result =
          kind === 'yield-deposit'
            ? await swig.yield.deposit({
                ...base,
                amount: 1,
                validatorVoteAddress: VOTE,
                positionSeed: 'deposit',
              })
            : kind === 'yield-exit'
              ? await swig.yield.exit({
                  ...base,
                  positionAddress: POSITION,
                  amount: 'all',
                })
              : await swig.yield.withdraw({
                  ...base,
                  positionAddress: POSITION,
                });
        expect(result.transaction.kind).toBe(kind);
        const compiled = await swig.transactions.compileParticipantSetApprovals(
          { preparedTransaction: result.transaction, approvals: [] },
        );
        expect(calls[1]?.body.preparedTransaction.kind).toBe(proto);
        expect(compiled.transaction.kind).toBe(kind);
      }
    },
  );
});
