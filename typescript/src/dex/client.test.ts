import { describe, expect, test } from 'bun:test';

import { SwigClient } from '../server/typescript/index.js';
import type { Network, RetryOptions } from '../types/index.js';

type CapturedRequest = { url: URL; method: string; body?: unknown };

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
const POOL = 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber';
const POSITION = 'GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv';
const CLMM_PROGRAM = 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK';

/** A Raydium CLMM action as the backend returns it. */
function actionOf(
  actionIndex: number,
  action: Record<string, unknown>,
  actingRoleId = 1,
) {
  return {
    actionIndex,
    swigVaultAddress: VAULT,
    actingRoleId,
    protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM',
    dexProgramAddress: CLMM_PROGRAM,
    action,
  };
}

// Responses below are copied from the backend over mainnet data.

const TRANSACTIONS_PAGE = {
  transactions: [
    {
      swigConfigAddress: SWIG,
      transactionSignature:
        '1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es',
      slot: '449761733',
      blockTime: '2026-09-23T16:47:12Z',
      actions: [
        actionOf(0, {
          liquidityAdd: {
            protocolInstruction: 'IncreaseLiquidityV2',
            protocolActorAddress: VAULT,
            dexPoolAddress: POOL,
            dexPositionAddress: POSITION,
            deposits: [
              {
                mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
                amountRaw: '-679914761680',
                decimals: 6,
              },
            ],
          },
        }),
      ],
    },
    {
      swigConfigAddress: SWIG,
      transactionSignature:
        '5ic74krw5dwJAHpdagUqheKqj7cRE1wSTPTGzFjRSLFTewpGjsXA1DVk8C2d5QEJSefbjqBr7QczLLVfK7kbF9ra',
      slot: '449761692',
      blockTime: '2026-09-23T16:47:01Z',
      actions: [
        actionOf(0, {
          swap: {
            protocolInstruction: 'SwapV2',
            protocolActorAddress: VAULT,
            dexPoolAddress: POOL,
            input: {
              mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
              amountRaw: '-85238320',
              decimals: 6,
            },
            output: {
              mintAddress: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
              amountRaw: '85263893',
              decimals: 6,
            },
          },
        }),
      ],
    },
  ],
  nextPageToken: 'next-page-token',
};

const POSITIONS_PAGE = {
  positions: [
    {
      swigConfigAddress: SWIG,
      swigVaultAddress: VAULT,
      protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM',
      dexPositionAddress: '8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH',
      open: {
        dexPoolAddress: POOL,
        state: {
          raydiumClmm: {
            liquidityRaw: '22090506953617',
            tickLowerIndex: 8,
            tickUpperIndex: 9,
            tokenFeesOwed0: '0',
            tokenFeesOwed1: '0',
            rewardAmountsOwed: ['0', '0', '0'],
          },
        },
      },
      updatedSlot: '454625145',
      sourceTransactionSignature:
        '4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V',
    },
    {
      swigConfigAddress: SWIG,
      swigVaultAddress: VAULT,
      protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM',
      dexPositionAddress: POSITION,
      closed: {},
      updatedSlot: '454625146',
      sourceTransactionSignature:
        '1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es',
    },
  ],
  nextPageToken: '',
};

describe('DexClient', () => {
  test('searches transactions with every filter in the body', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return TRANSACTIONS_PAGE;
    });

    const page = await swig.dex.transactions.list({
      swigConfigAddress: SWIG,
      network: 'mainnet',
      pageSize: 2,
      pageToken: 'previous-token',
      startSlot: 449_000_000,
      endSlot: 450_000_000,
      protocol: 'raydium-clmm',
      actionType: 'liquidity-add',
      swigVaultAddress: VAULT,
    });

    expect(requests[0].method).toBe('POST');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/transactions:search`,
    );
    expect(requests[0].body).toEqual({
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
    expect(page.nextPageToken).toBe('next-page-token');
    expect(page.transactions.map((transaction) => transaction.slot)).toEqual([
      449761733, 449761692,
    ]);
    expect(page.transactions[0].blockTime).toBe('2026-09-23T16:47:12Z');
    expect(page.transactions[0].actions).toEqual([
      {
        actionIndex: 0,
        swigVaultAddress: VAULT,
        actingRoleId: 1,
        protocol: 'raydium-clmm',
        dexProgramAddress: CLMM_PROGRAM,
        action: {
          type: 'liquidity-add',
          protocolInstruction: 'IncreaseLiquidityV2',
          protocolActorAddress: VAULT,
          dexPoolAddress: POOL,
          dexPositionAddress: POSITION,
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

  test('sends only the client network when no filter is set', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return { transactions: [], nextPageToken: '' };
    });

    const page = await swig.dex.transactions.list({ swigConfigAddress: SWIG });

    expect(requests[0].body).toEqual({ network: 'NETWORK_DEVNET' });
    expect(page).toEqual({ transactions: [] });
  });

  test('gets one transaction by signature', async () => {
    const swap = TRANSACTIONS_PAGE.transactions[1];
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return { transaction: swap };
    });

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: SWIG,
      network: 'mainnet',
      transactionSignature: swap.transactionSignature,
    });

    expect(requests[0].method).toBe('GET');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/transactions/${swap.transactionSignature}`,
    );
    expect(requests[0].url.search).toBe('?network=NETWORK_MAINNET');
    expect(transaction.slot).toBe(449761692);
    expect(transaction.actions[0].action).toEqual({
      type: 'swap',
      protocolInstruction: 'SwapV2',
      protocolActorAddress: VAULT,
      dexPoolAddress: POOL,
      input: {
        mintAddress: 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk',
        amountRaw: '-85238320',
        decimals: 6,
      },
      output: {
        mintAddress: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
        amountRaw: '85263893',
        decimals: 6,
      },
    });
  });

  test('searches positions with every filter and decodes open and closed', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return POSITIONS_PAGE;
    });

    const page = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      network: 'mainnet',
      pageSize: 25,
      positionStatus: 'open',
      protocol: 'raydium-clmm',
      swigVaultAddress: VAULT,
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
    expect(page.positions).toEqual([
      {
        swigConfigAddress: SWIG,
        swigVaultAddress: VAULT,
        protocol: 'raydium-clmm',
        dexPositionAddress: '8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH',
        updatedSlot: 454625145,
        sourceTransactionSignature:
          '4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V',
        status: 'open',
        dexPoolAddress: POOL,
        state: {
          type: 'raydium-clmm',
          liquidityRaw: '22090506953617',
          tickLowerIndex: 8,
          tickUpperIndex: 9,
          tokenFeesOwed0: '0',
          tokenFeesOwed1: '0',
          rewardAmountsOwed: ['0', '0', '0'],
        },
      },
      {
        swigConfigAddress: SWIG,
        swigVaultAddress: VAULT,
        protocol: 'raydium-clmm',
        dexPositionAddress: POSITION,
        updatedSlot: 454625146,
        sourceTransactionSignature:
          '1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es',
        status: 'closed',
      },
    ]);
  });

  test('decodes every other action kind', async () => {
    const amount = { mintAddress: 'MINT', amountRaw: '5' };
    const base = {
      protocolInstruction: 'Harvest',
      protocolActorAddress: VAULT,
      dexPoolAddress: POOL,
      dexPositionAddress: POSITION,
    };
    const swig = client(() => ({
      transaction: {
        swigConfigAddress: SWIG,
        transactionSignature: 'signature',
        slot: '1',
        actions: [
          actionOf(0, {
            positionOpen: {
              protocolInstruction: 'OpenPositionWithToken22Nft',
              protocolActorAddress: VAULT,
              dexPoolAddress: POOL,
              dexPositionAddress: POSITION,
              deposits: [amount],
            },
          }),
          actionOf(1, {
            liquidityRemove: {
              protocolInstruction: 'DecreaseLiquidityV2',
              protocolActorAddress: VAULT,
              dexPoolAddress: POOL,
              dexPositionAddress: POSITION,
              withdrawals: [amount],
            },
          }),
          // Raydium CLMM's ClosePosition names no pool. Role 0 is a real role.
          actionOf(
            2,
            {
              positionClose: {
                protocolInstruction: 'ClosePosition',
                protocolActorAddress: VAULT,
                dexPositionAddress: POSITION,
              },
            },
            0,
          ),
          actionOf(3, {
            positionRebalance: {
              ...base,
              deposits: [amount],
              withdrawals: [amount],
            },
          }),
          actionOf(4, { feeClaim: { ...base, claimed: [amount] } }),
          actionOf(5, { rewardClaim: { ...base, claimed: [amount] } }),
        ],
      },
    }));

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: SWIG,
      transactionSignature: 'signature',
    });

    expect(transaction.blockTime).toBeUndefined();
    expect(transaction.actions[2].actingRoleId).toBe(0);
    expect(transaction.actions.map((action) => action.action)).toEqual([
      {
        type: 'position-open',
        protocolInstruction: 'OpenPositionWithToken22Nft',
        protocolActorAddress: VAULT,
        dexPoolAddress: POOL,
        dexPositionAddress: POSITION,
        deposits: [amount],
      },
      {
        type: 'liquidity-remove',
        protocolInstruction: 'DecreaseLiquidityV2',
        protocolActorAddress: VAULT,
        dexPoolAddress: POOL,
        dexPositionAddress: POSITION,
        withdrawals: [amount],
      },
      {
        type: 'position-close',
        protocolInstruction: 'ClosePosition',
        protocolActorAddress: VAULT,
        dexPositionAddress: POSITION,
      },
      {
        type: 'position-rebalance',
        ...base,
        deposits: [amount],
        withdrawals: [amount],
      },
      { type: 'fee-claim', ...base, claimed: [amount] },
      { type: 'reward-claim', ...base, claimed: [amount] },
    ]);
  });

  test('retries a search on a transient failure, as it only reads', async () => {
    for (const [search, empty] of [
      [
        (swig: SwigClient) =>
          swig.dex.transactions.list({ swigConfigAddress: SWIG }),
        { transactions: [] },
      ],
      [
        (swig: SwigClient) =>
          swig.dex.positions.list({ swigConfigAddress: SWIG }),
        { positions: [] },
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
    const [first] = TRANSACTIONS_PAGE.transactions;
    const withAction = (change: Record<string, unknown>) => ({
      transactions: [
        { ...first, actions: [{ ...first.actions[0], ...change }] },
      ],
    });
    // JSON drops an undefined value, so these keys are absent on the wire.
    const stateless = { ...POSITIONS_PAGE.positions[1], closed: undefined };

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
              dexPoolAddress: POOL,
              input: { mintAddress: 'MINT', amountRaw: '1.5' },
              output: { mintAddress: 'MINT', amountRaw: '1' },
            },
          },
        }),
        'invalid amountRaw',
      ],
      [withAction({ actingRoleId: undefined }), 'invalid actingRoleId'],
    ] as const) {
      await expect(
        client(() => response).dex.transactions.list({
          swigConfigAddress: SWIG,
        }),
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
          swig.dex.positions.list({
            swigConfigAddress: SWIG,
            positionStatus: 'pending' as never,
          }),
        'positionStatus must be "open" or "closed"',
      ],
      [
        () =>
          swig.dex.positions.list({ swigConfigAddress: SWIG, pageSize: -1 }),
        'pageSize must be a non-negative integer',
      ],
      [
        () =>
          client(unreachable, null).dex.positions.list({
            swigConfigAddress: SWIG,
          }),
        'network is required',
      ],
    ] as const) {
      await expect(call()).rejects.toThrow(message);
    }
  });
});
