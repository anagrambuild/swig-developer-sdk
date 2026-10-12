import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { SwigClient } from '../server/typescript/index.js';
import type { Network, RetryOptions } from '../types/index.js';

type CapturedRequest = { url: URL; method: string; body?: unknown };

/** Responses captured from the Developer API over mainnet data. */
function capture(name: string): Record<string, unknown> {
  return JSON.parse(
    readFileSync(
      new URL(`../../../fixtures/dex/${name}.json`, import.meta.url),
      'utf8',
    ),
  );
}

/** The handler returns a body to send as JSON, or a whole `Response`. */
function client(
  handler: (request: CapturedRequest) => unknown,
  network: Network | null = 'devnet',
  retryOptions?: RetryOptions,
) {
  return new SwigClient({
    apiKey: 'sk_test',
    baseUrl: 'http://localhost:8080',
    ...(network ? { network } : {}),
    ...(retryOptions ? { retryOptions } : {}),
    fetch: (async (input, init) => {
      const request = new Request(input, init);
      const body = await request.text();
      const reply = handler({
        url: new URL(request.url),
        method: request.method,
        ...(body ? { body: JSON.parse(body) } : {}),
      });
      return reply instanceof Response ? reply : Response.json(reply);
    }) as typeof fetch,
  });
}

function unreachable(): never {
  throw new Error('no request should be sent');
}

const SWIG = '8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX';
const VAULT = '4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej';

describe('DexClient', () => {
  test('searches transactions with every filter in the body', async () => {
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
      filters: [
        { kind: 'startSlot', value: 449_000_000n },
        { kind: 'endSlot', value: 450_000_000n },
        { kind: 'protocol', value: 'raydium-clmm' },
        { kind: 'actionType', value: 'liquidity-add' },
        { kind: 'swigVaultAddress', value: VAULT },
      ],
    });

    const [request] = requests;
    expect(request.method).toBe('POST');
    expect(request.url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/transactions:search`,
    );
    expect(request.url.search).toBe('');
    expect(request.body).toEqual({
      network: 'NETWORK_MAINNET',
      pageSize: 2,
      pageToken: 'previous-token',
      filters: [
        { startSlot: '449000000' },
        { endSlot: '450000000' },
        { protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM' },
        { actionType: 'DEX_ACTION_TYPE_LIQUIDITY_ADD' },
        { swigVaultAddress: VAULT },
      ],
    });
    expect(page.nextPageToken).toBe(
      capture('list_transactions_clmm_page').nextPageToken as string,
    );
    expect(page.transactions.map((transaction) => transaction.slot)).toEqual([
      449761733, 449761692,
    ]);
    const [newest] = page.transactions;
    expect(newest.blockTime).toBe('2026-09-23T16:47:12Z');
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
          dexPositionAddress: 'GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv',
          deposits: [
            {
              mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
              amountRaw: '-679914761680',
              decimals: 6,
            },
          ],
        },
      },
    ]);
    expect(page.transactions[1].actions[0].action.type).toBe('swap');
  });

  test('sends only the network the client defaults to when no filter is set', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return { transactions: [], nextPageToken: '' };
    });

    const page = await swig.dex.transactions.list({ swigConfigAddress: SWIG });

    expect(requests[0].body).toEqual({ network: 'NETWORK_DEVNET' });
    expect(page).toEqual({ transactions: [] });
  });

  test('decodes a close without a pool, authorized by role 0', async () => {
    const swig = client(() => capture('list_transactions_position_close'));

    const page = await swig.dex.transactions.list({
      swigConfigAddress: SWIG,
      filters: [{ kind: 'actionType', value: 'position-close' }],
    });

    expect(page.nextPageToken).toBeUndefined();
    const [transaction] = page.transactions;
    expect(transaction.slot).toBe(449615165);
    expect(transaction.actions).toEqual([
      {
        actionIndex: 0,
        swigVaultAddress: VAULT,
        actingRoleId: 0,
        protocol: 'raydium-clmm',
        dexProgramAddress: 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
        action: {
          type: 'position-close',
          protocolInstruction: 'ClosePosition',
          protocolActorAddress: VAULT,
          dexPositionAddress: 'HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp',
        },
      },
    ]);
  });

  test('decodes a captured position open and liquidity removal', async () => {
    const opened = await client(() =>
      capture('list_transactions_position_open'),
    ).dex.transactions.list({
      swigConfigAddress: SWIG,
      pageSize: 1,
      filters: [{ kind: 'actionType', value: 'position-open' }],
    });

    expect(opened.nextPageToken).toBe(
      capture('list_transactions_position_open').nextPageToken as string,
    );
    expect(opened.transactions.map((transaction) => transaction.slot)).toEqual([
      449753334,
    ]);
    expect(opened.transactions[0].actions).toEqual([
      {
        actionIndex: 0,
        swigVaultAddress: VAULT,
        actingRoleId: 1,
        protocol: 'raydium-clmm',
        dexProgramAddress: 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
        action: {
          type: 'position-open',
          protocolInstruction: 'OpenPositionWithToken22Nft',
          protocolActorAddress: VAULT,
          dexPoolAddress: 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber',
          dexPositionAddress: 'GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv',
          deposits: [
            {
              mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
              amountRaw: '-11120779129',
              decimals: 6,
            },
          ],
        },
      },
    ]);

    const removed = await client(() =>
      capture('list_transactions_liquidity_remove'),
    ).dex.transactions.list({
      swigConfigAddress: SWIG,
      pageSize: 1,
      filters: [{ kind: 'actionType', value: 'liquidity-remove' }],
    });

    expect(removed.nextPageToken).toBe(
      capture('list_transactions_liquidity_remove').nextPageToken as string,
    );
    expect(removed.transactions.map((transaction) => transaction.slot)).toEqual(
      [449691047],
    );
    expect(removed.transactions[0].actions).toEqual([
      {
        actionIndex: 0,
        swigVaultAddress: VAULT,
        actingRoleId: 3,
        protocol: 'raydium-clmm',
        dexProgramAddress: 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
        action: {
          type: 'liquidity-remove',
          protocolInstruction: 'DecreaseLiquidityV2',
          protocolActorAddress: VAULT,
          dexPoolAddress: 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber',
          dexPositionAddress: '8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH',
          withdrawals: [
            {
              mintAddress: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
              amountRaw: '1092908',
              decimals: 6,
            },
          ],
        },
      },
    ]);
  });

  test('gets one transaction by signature', async () => {
    const signature =
      '2GsnxzTAfzcZATGCj6VraZZAJ8zHFbBDRDvc3geDuuBdB8DLvwxPkYQWJURXSWC7oooU1PEbY1jUXLiW9HZcsvGv';
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return capture('get_transaction_swap');
    });

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: 'H2JPxSKQJU22CbegDVZZGd6jGhTLGJYtCLXtKRDzJL6c',
      network: 'mainnet',
      transactionSignature: signature,
    });

    expect(requests[0].method).toBe('GET');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/H2JPxSKQJU22CbegDVZZGd6jGhTLGJYtCLXtKRDzJL6c/dex/transactions/${signature}`,
    );
    expect(requests[0].url.search).toBe('?network=NETWORK_MAINNET');
    expect(transaction.transactionSignature).toBe(signature);
    expect(transaction.slot).toBe(454177312);
    expect(transaction.actions[0]).toEqual({
      actionIndex: 0,
      swigVaultAddress: '9BySxhQYrvGgJnDekxNWtkQWnZutiMBSH3wuH43HRhg2',
      actingRoleId: 1,
      protocol: 'orca-whirlpool',
      dexProgramAddress: 'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc',
      action: {
        type: 'swap',
        protocolInstruction: 'SwapV2',
        protocolActorAddress: 'D5YqVMoSxnqeZAKAUUE1Dm3bmjtdxQ5DCF356ozqN9cM',
        dexPoolAddress: 'CUsHj4Xf1dAGoCHUX3YYgsmN1PWCn7aSgCUCJ3Wvz7tq',
        input: {
          mintAddress: 'So11111111111111111111111111111111111111112',
          amountRaw: '-8813489',
          decimals: 9,
        },
        output: {
          mintAddress: 'GN3Njbg9dP13pBX2MvdXurr4cuxzVmsVtKwnhPPymWZh',
          amountRaw: '470167',
          decimals: 4,
        },
      },
    });
  });

  test('searches positions with every filter and decodes open and closed', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return capture('list_positions');
    });

    const page = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      network: 'mainnet',
      pageSize: 25,
      filters: [
        { kind: 'positionStatus', value: 'open' },
        { kind: 'protocol', value: 'raydium-clmm' },
        { kind: 'swigVaultAddress', value: VAULT },
      ],
    });

    expect(requests[0].method).toBe('POST');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/positions:search`,
    );
    expect(requests[0].body).toEqual({
      network: 'NETWORK_MAINNET',
      pageSize: 25,
      filters: [
        { positionStatus: 'POSITION_STATUS_OPEN' },
        { protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM' },
        { swigVaultAddress: VAULT },
      ],
    });
    expect(page.nextPageToken).toBeUndefined();
    expect(page.positions.map((position) => position.status)).toEqual([
      'open',
      'closed',
      'closed',
    ]);
    expect(page.positions[0]).toEqual({
      swigConfigAddress: SWIG,
      swigVaultAddress: VAULT,
      protocol: 'raydium-clmm',
      dexPositionAddress: '8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH',
      updatedSlot: 454625145,
      sourceTransactionSignature:
        '4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V',
      status: 'open',
      dexPoolAddress: 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber',
      state: {
        type: 'raydium-clmm',
        liquidityRaw: '22090506953617',
        tickLowerIndex: 8,
        tickUpperIndex: 9,
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
      updatedSlot: 454625146,
      sourceTransactionSignature:
        '1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es',
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
      actingRoleId: 1,
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

  test('refuses an action without actingRoleId', async () => {
    const response = capture('get_transaction_swap') as {
      transaction: { actions: Array<Record<string, unknown>> };
    };
    delete response.transaction.actions[0].actingRoleId;

    await expect(
      client(() => response).dex.transactions.get({
        swigConfigAddress: SWIG,
        transactionSignature: 'signature',
      }),
    ).rejects.toThrow('invalid actingRoleId');
  });

  test('retries a search on a transient failure, as it only reads', async () => {
    for (const [search, empty] of [
      [
        (swig: SwigClient) =>
          swig.dex.transactions.list({ swigConfigAddress: SWIG }),
        { transactions: [], nextPageToken: '' },
      ],
      [
        (swig: SwigClient) =>
          swig.dex.positions.list({ swigConfigAddress: SWIG }),
        { positions: [], nextPageToken: '' },
      ],
    ] as const) {
      let attempts = 0;
      const swig = client(
        () => {
          attempts += 1;
          return attempts === 1 ? new Response('', { status: 503 }) : empty;
        },
        'devnet',
        { maxRetries: 1, retryDelay: 0 },
      );

      await search(swig);
      expect(attempts).toBe(2);
    }
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
            filters: [{ kind: 'protocol', value: 'uniswap' as never }],
          }),
        'protocol must be a supported DEX protocol',
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            filters: [{ kind: 'actionType', value: 'stake' as never }],
          }),
        'actionType must be a supported DEX action type',
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            filters: [{ kind: 'startSlot', value: -1n }],
          }),
        'startSlot must be a non-negative bigint',
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            filters: [{ kind: 'endSlot', value: 1.5 as never }],
          }),
        'endSlot must be a non-negative bigint',
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            filters: [{ kind: 'positionStatus', value: 'open' } as never],
          }),
        'filter kind must be startSlot, endSlot, protocol, actionType or swigVaultAddress',
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
            filters: [{ kind: 'positionStatus', value: 'pending' as never }],
          }),
        'positionStatus must be "open" or "closed"',
      ],
      [
        () =>
          swig.dex.positions.list({
            swigConfigAddress: SWIG,
            filters: [{ kind: 'actionType', value: 'swap' } as never],
          }),
        'filter kind must be positionStatus, protocol or swigVaultAddress',
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
