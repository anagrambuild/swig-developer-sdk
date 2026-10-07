import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { SwigClient } from '../server/typescript/index.js';
import type { Network } from '../types/index.js';

type CapturedRequest = { url: URL; method: string };

/** Responses captured from the Developer API over mainnet data. */
function capture(name: string): Record<string, unknown> {
  return JSON.parse(
    readFileSync(
      new URL(`../../../fixtures/dex/${name}.json`, import.meta.url),
      'utf8',
    ),
  );
}

function client(
  handler: (request: CapturedRequest) => unknown,
  network: Network | null = 'devnet',
) {
  return new SwigClient({
    apiKey: 'sk_test',
    baseUrl: 'http://localhost:8080',
    ...(network ? { network } : {}),
    fetch: (async (input, init) => {
      const request = new Request(input, init);
      return Response.json(
        handler({ url: new URL(request.url), method: request.method }),
      );
    }) as typeof fetch,
  });
}

function unreachable(): never {
  throw new Error('no request should be sent');
}

const SWIG = '8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX';
const VAULT = '4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej';

describe('DexClient', () => {
  test('lists transactions with every filter on the query', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return capture('list_transactions_clmm_page');
    });

    const page = await swig.dex.transactions.list({
      swigConfigAddress: SWIG,
      network: 'mainnet',
      pageSize: 2,
      pageToken: 'previous-token',
      startSlot: 449_000_000n,
      endSlot: 450_000_000,
      protocol: 'raydium-clmm',
      actionType: 'liquidity-add',
      swigVaultAddress: VAULT,
    });

    const [request] = requests;
    expect(request.method).toBe('GET');
    expect(request.url.pathname).toBe(`/wallet/swig/${SWIG}/dex/transactions`);
    expect(Object.fromEntries(request.url.searchParams)).toEqual({
      network: 'NETWORK_MAINNET',
      pageSize: '2',
      pageToken: 'previous-token',
      startSlot: '449000000',
      endSlot: '450000000',
      protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM',
      actionType: 'DEX_ACTION_TYPE_LIQUIDITY_ADD',
      swigVaultAddress: VAULT,
    });
    expect(page.nextPageToken).toBe(
      capture('list_transactions_clmm_page').nextPageToken as string,
    );
    expect(page.transactions.map((transaction) => transaction.slot)).toEqual([
      449776519, 449775094,
    ]);
    const [newest] = page.transactions;
    expect(newest.blockTime).toBe('2026-09-23T17:52:56Z');
    expect(newest.actions).toEqual([
      {
        actionIndex: 0,
        swigVaultAddress: VAULT,
        actingRoleId: 1,
        protocol: 'raydium-clmm',
        dexProgramAddress: 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
        action: {
          type: 'liquidity-add',
          protocolInstruction: 'IncreaseLiquidityV2',
          protocolActorAddress: VAULT,
          dexPoolAddress: 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber',
          dexPositionAddress: 'H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm',
          deposits: [
            {
              mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
              amountRaw: '-500000000000',
              decimals: 6,
            },
          ],
        },
      },
    ]);
    expect(page.transactions[1].actions[0].action.type).toBe('position-open');
  });

  test('sends only the network the client defaults to when no filter is set', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return { transactions: [], nextPageToken: '' };
    });

    const page = await swig.dex.transactions.list({ swigConfigAddress: SWIG });

    expect(requests[0].url.search).toBe('?network=NETWORK_DEVNET');
    expect(page).toEqual({ transactions: [] });
  });

  test('decodes a close without a pool and keeps the transaction’s other actions', async () => {
    const swig = client(() => capture('list_transactions_position_close'));

    const page = await swig.dex.transactions.list({
      swigConfigAddress: SWIG,
      actionType: 'position-close',
    });

    expect(page.nextPageToken).toBeUndefined();
    const [transaction] = page.transactions;
    expect(transaction.actions.map((action) => action.action.type)).toEqual([
      'liquidity-remove',
      'position-close',
      'position-open',
    ]);
    const close = transaction.actions[1].action;
    expect(close).toEqual({
      type: 'position-close',
      protocolInstruction: 'ClosePosition',
      protocolActorAddress: VAULT,
      dexPositionAddress: 'GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv',
    });
    const remove = transaction.actions[0].action;
    expect(remove.type === 'liquidity-remove' && remove.withdrawals).toEqual([
      {
        mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
        amountRaw: '990788839009',
        decimals: 6,
      },
      {
        mintAddress: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
        amountRaw: '401407',
        decimals: 6,
      },
    ]);
  });

  test('gets one transaction by signature', async () => {
    const signature =
      'AdSK8SeJFrC7w4E1vwkgVwMpRzaNN87GdTrfCXvMpLyTvXJuFoGTBLAnW6wJ2BM1w7PAwuTytvHcAf2bPFqJpRH';
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return capture('get_transaction_swap');
    });

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: '4m3ptUjYAYLXJUDSKiTDemKUWht98UbEFPgEGvxHnkBz',
      network: 'mainnet',
      transactionSignature: signature,
    });

    expect(requests[0].url.pathname).toBe(
      `/wallet/swig/4m3ptUjYAYLXJUDSKiTDemKUWht98UbEFPgEGvxHnkBz/dex/transactions/${signature}`,
    );
    expect(requests[0].url.search).toBe('?network=NETWORK_MAINNET');
    expect(transaction.transactionSignature).toBe(signature);
    expect(transaction.slot).toBe(452163114);
    expect(transaction.actions[0]).toEqual({
      actionIndex: 0,
      swigVaultAddress: 'EnU8v4oYX6dzedmBm78MvJNzJUiJGmyaXyQiTcspPUAe',
      actingRoleId: 10,
      protocol: 'orca-whirlpool',
      dexProgramAddress: 'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc',
      action: {
        type: 'swap',
        protocolInstruction: 'SwapV2',
        protocolActorAddress: 'D5YqVMoSxnqeZAKAUUE1Dm3bmjtdxQ5DCF356ozqN9cM',
        dexPoolAddress: 'Ckp1kwZqosaLU1h3zWtuaMBubyWM7LX3cxYezRVin7p2',
        input: {
          mintAddress: 'So11111111111111111111111111111111111111112',
          amountRaw: '-12994222',
          decimals: 9,
        },
        output: {
          mintAddress: '6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN',
          amountRaw: '701624',
          decimals: 6,
        },
      },
    });
  });

  test('lists positions with every filter and decodes open and closed', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return capture('list_positions');
    });

    const page = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      network: 'mainnet',
      pageSize: 25,
      positionStatus: 'open',
      protocol: 'raydium-clmm',
      swigVaultAddress: VAULT,
    });

    expect(requests[0].url.pathname).toBe(`/wallet/swig/${SWIG}/dex/positions`);
    expect(Object.fromEntries(requests[0].url.searchParams)).toEqual({
      network: 'NETWORK_MAINNET',
      pageSize: '25',
      positionStatus: 'POSITION_STATUS_OPEN',
      protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM',
      swigVaultAddress: VAULT,
    });
    expect(page.nextPageToken).toBeUndefined();
    expect(page.positions.map((position) => position.status)).toEqual([
      'open',
      'closed',
      'open',
    ]);
    expect(page.positions[0]).toEqual({
      swigConfigAddress: SWIG,
      swigVaultAddress: VAULT,
      protocol: 'raydium-clmm',
      dexPositionAddress: 'BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7',
      observedSlot: 454258554,
      sourceTransactionSignature:
        '2RVmaGZYw4wvKTnEz4ueYYjKYAWgA2dHdMeos9EJMrkEqDQCnBtk8hLGf5AkRw42b9H7K2aXC8KDXCtd9MmZ6L5j',
      status: 'open',
      dexPoolAddress: 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber',
      state: {
        type: 'raydium-clmm',
        liquidityRaw: '18817287959235504',
        tickLowerIndex: 0,
        tickUpperIndex: 1,
        tokenFeesOwed0: '0',
        tokenFeesOwed1: '0',
        rewardAmountsOwed: ['0', '0', '0'],
      },
    });
    expect(page.positions[1]).toEqual({
      swigConfigAddress: SWIG,
      swigVaultAddress: VAULT,
      protocol: 'raydium-clmm',
      dexPositionAddress: 'GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv',
      observedSlot: 454258553,
      sourceTransactionSignature:
        '4KheMDLmCjiasYy7vHX6VxFJEJR3gH69QDAifUqkE91HSMqEVksDAcxEraEY1UVzoS28yEwdA3BBgHAoWZUaMuhX',
      status: 'closed',
    });
  });

  test('decodes the action kinds the captures do not contain from their ProtoJSON shape', async () => {
    const base = {
      protocolInstruction: 'Harvest',
      protocolActorAddress: VAULT,
      dexPoolAddress: 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber',
      dexPositionAddress: 'HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp',
    };
    const amount = { mintAddress: 'MINT', amountRaw: '5' };
    const actionOf = (index: number, action: Record<string, unknown>) => ({
      actionIndex: index,
      swigVaultAddress: VAULT,
      actingRoleId: '1',
      protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM',
      dexProgramAddress: 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
      action,
    });
    const swig = client(() => ({
      transaction: {
        swigConfigAddress: SWIG,
        transactionSignature: 'signature',
        slot: '1',
        actions: [
          actionOf(0, { feeClaim: { ...base, claimed: [amount] } }),
          actionOf(1, { reward_claim: { ...base, claimed: [amount] } }),
          actionOf(2, {
            positionRebalance: {
              ...base,
              deposits: [amount],
              withdrawals: [amount],
            },
          }),
        ],
      },
    }));

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: SWIG,
      transactionSignature: 'signature',
    });

    expect(transaction.blockTime).toBeUndefined();
    expect(transaction.actions.map((action) => action.action)).toEqual([
      { type: 'fee-claim', ...base, claimed: [amount] },
      { type: 'reward-claim', ...base, claimed: [amount] },
      {
        type: 'position-rebalance',
        ...base,
        deposits: [amount],
        withdrawals: [amount],
      },
    ]);
    expect(transaction.actions[0].actingRoleId).toBe(1);
  });

  test('refuses a response outside the contract', async () => {
    const listed = capture('list_transactions_clmm_page') as {
      transactions: Array<{ actions: Array<Record<string, unknown>> }>;
    };
    const withAction = (action: Record<string, unknown>) =>
      structuredClone({
        ...listed,
        transactions: [
          {
            ...listed.transactions[0],
            actions: [{ ...listed.transactions[0].actions[0], ...action }],
          },
        ],
      });
    const positions = capture('list_positions') as {
      positions: Array<Record<string, unknown>>;
    };
    const stateless = Object.fromEntries(
      Object.entries(positions.positions[1]).filter(
        ([key]) => key !== 'closed',
      ),
    );

    for (const [response, message] of [
      [
        withAction({ protocol: 'DEX_PROTOCOL_UNSPECIFIED' }),
        'invalid protocol',
      ],
      [withAction({ protocol: 'DEX_PROTOCOL_UNISWAP' }), 'invalid protocol'],
      [withAction({ action: {} }), 'unknown action'],
      [
        withAction({
          action: {
            swap: {
              protocolInstruction: 'Buy',
              protocolActorAddress: VAULT,
              dexPoolAddress: 'POOL',
              input: { mintAddress: 'MINT', amountRaw: '1.5' },
              output: { mintAddress: 'MINT', amountRaw: '1' },
            },
          },
        }),
        'invalid amountRaw',
      ],
      [withAction({ actingRoleId: 'two' }), 'invalid actingRoleId'],
    ] as const) {
      const swig = client(() => response);
      await expect(
        swig.dex.transactions.list({ swigConfigAddress: SWIG }),
      ).rejects.toThrow(message);
    }

    await expect(
      client(() => ({})).dex.transactions.get({
        swigConfigAddress: SWIG,
        transactionSignature: 'signature',
      }),
    ).rejects.toThrow('missing transaction');
    await expect(
      client(() => ({ positions: [stateless] })).dex.positions.list({
        swigConfigAddress: SWIG,
      }),
    ).rejects.toThrow('missing position state');
  });

  test('rejects invalid arguments before sending a request', async () => {
    const swig = client(unreachable);
    const withoutNetwork = client(unreachable, null);

    for (const [call, message] of [
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            protocol: 'uniswap' as never,
          }),
        'protocol must be a supported DEX protocol',
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            actionType: 'stake' as never,
          }),
        'actionType must be a supported DEX action type',
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            startSlot: -1,
          }),
        'startSlot must be a non-negative integer',
      ],
      [
        () =>
          swig.dex.transactions.list({ swigConfigAddress: SWIG, endSlot: 1.5 }),
        'endSlot must be a non-negative integer',
      ],
      [
        () =>
          swig.dex.positions.list({ swigConfigAddress: SWIG, pageSize: -1 }),
        'pageSize must be a non-negative integer',
      ],
      [
        () =>
          swig.dex.positions.list({
            swigConfigAddress: SWIG,
            positionStatus: 'pending' as never,
          }),
        'positionStatus must be "open" or "closed"',
      ],
      [
        () => withoutNetwork.dex.positions.list({ swigConfigAddress: SWIG }),
        'network is required',
      ],
    ] as const) {
      await expect(call()).rejects.toThrow(message);
    }
  });
});
