import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { SwigClient } from '../server/typescript/index.js';
import type {
  DexActionType,
  DexPositionStatus,
  DexProtocol,
  GetDexTransactionWire,
  ListDexPositionsWire,
  ListDexTransactionsArgs,
  ListDexTransactionsWire,
  Network,
  RetryOptions,
} from '../types/index.js';

type CapturedRequest = { url: URL; method: string; body?: unknown };

/** The handler returns a body to send as JSON, or a whole `Response`. */
function client(
  handler: (request: CapturedRequest) => unknown,
  network: Network | null = 'mainnet',
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

/** A body the backend returned, verbatim. fixtures/dex/README.md lists each request. */
function capture(name: string): string {
  return readFileSync(
    new URL(`../../../fixtures/dex/${name}.json`, import.meta.url),
    'utf8',
  );
}

const HISTORY: ListDexTransactionsWire = JSON.parse(
  capture('list_transactions'),
);
const LIQUIDITY_ADDS: ListDexTransactionsWire = JSON.parse(
  capture('list_transactions_filtered'),
);
const LIQUIDITY_ADDS_NEXT: ListDexTransactionsWire = JSON.parse(
  capture('list_transactions_filtered_next'),
);
const NO_TRANSACTIONS: ListDexTransactionsWire = JSON.parse(
  capture('list_transactions_empty'),
);
const SWAP: GetDexTransactionWire = JSON.parse(capture('get_transaction_swap'));
const ALL_POSITIONS: ListDexPositionsWire = JSON.parse(
  capture('list_positions_all'),
);
const OPEN_POSITIONS: ListDexPositionsWire = JSON.parse(
  capture('list_positions_open'),
);
const CLOSED_POSITIONS: ListDexPositionsWire = JSON.parse(
  capture('list_positions_closed'),
);
const POSITIONS_PAGE: ListDexPositionsWire = JSON.parse(
  capture('list_positions'),
);
const POSITIONS_NEXT_PAGE: ListDexPositionsWire = JSON.parse(
  capture('list_positions_next'),
);
const NO_POSITIONS: ListDexPositionsWire = JSON.parse(
  capture('list_positions_empty'),
);

const SWIG = '8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX';
const VAULT = '4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej';
const POOL = 'EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber';
const CLMM_PROGRAM = 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK';
const USDV = 'USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

/** The request behind list_transactions_filtered.json. */
const LIQUIDITY_ADD_ARGS: ListDexTransactionsArgs = {
  swigConfigAddress: SWIG,
  pageSize: 2,
  startSlot: 449_000_000,
  endSlot: 450_000_000,
  protocol: 'raydium-clmm',
  actionType: 'liquidity-add',
  swigVaultAddress: VAULT,
};
const LIQUIDITY_ADD_BODY = {
  network: 'NETWORK_MAINNET',
  pageSize: 2,
  filters: [
    { startSlot: '449000000' },
    { endSlot: '450000000' },
    { protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM' },
    { actionType: 'DEX_ACTION_TYPE_LIQUIDITY_ADD' },
    { swigVaultAddress: VAULT },
  ],
};

const PROTOCOLS: Array<[DexProtocol, string]> = [
  ['pump-swap', 'DEX_PROTOCOL_PUMP_SWAP'],
  ['meteora-dlmm', 'DEX_PROTOCOL_METEORA_DLMM'],
  ['raydium-amm-v4', 'DEX_PROTOCOL_RAYDIUM_AMM_V4'],
  ['raydium-cpmm', 'DEX_PROTOCOL_RAYDIUM_CPMM'],
  ['raydium-clmm', 'DEX_PROTOCOL_RAYDIUM_CLMM'],
  ['orca-whirlpool', 'DEX_PROTOCOL_ORCA_WHIRLPOOL'],
];
const ACTION_TYPES: Array<[DexActionType, string]> = [
  ['swap', 'DEX_ACTION_TYPE_SWAP'],
  ['position-open', 'DEX_ACTION_TYPE_POSITION_OPEN'],
  ['liquidity-add', 'DEX_ACTION_TYPE_LIQUIDITY_ADD'],
  ['liquidity-remove', 'DEX_ACTION_TYPE_LIQUIDITY_REMOVE'],
  ['position-rebalance', 'DEX_ACTION_TYPE_POSITION_REBALANCE'],
  ['fee-claim', 'DEX_ACTION_TYPE_FEE_CLAIM'],
  ['reward-claim', 'DEX_ACTION_TYPE_REWARD_CLAIM'],
  ['position-close', 'DEX_ACTION_TYPE_POSITION_CLOSE'],
];
const POSITION_STATUSES: Array<[DexPositionStatus, string]> = [
  ['open', 'POSITION_STATUS_OPEN'],
  ['closed', 'POSITION_STATUS_CLOSED'],
];

describe('DexClient', () => {
  test('lists transactions with every filter in the body', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return LIQUIDITY_ADDS;
    });

    const page = await swig.dex.transactions.list(LIQUIDITY_ADD_ARGS);

    expect(requests[0].method).toBe('POST');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/transactions`,
    );
    expect(requests[0].body).toStrictEqual(LIQUIDITY_ADD_BODY);
    expect(page.nextCursor).toStrictEqual({
      slot: 449761733,
      transactionSignature:
        '1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es',
    });
    expect(page.transactions.map((transaction) => transaction.slot)).toEqual([
      449776519, 449761733,
    ]);
    expect(page.transactions[0]).toStrictEqual({
      swigConfigAddress: SWIG,
      transactionSignature:
        '4nT1pQZcoX3ypGHGsM21EK6X7CFvdeNq7At8naf3mixX8JsvxztBecchqCWkn2JwErM9334s3NM5hpc2rYvSrjCJ',
      slot: 449776519,
      blockTime: '2026-09-23T17:52:56Z',
      actions: [
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
            dexPositionAddress: 'H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm',
            deposits: [
              { mintAddress: USDV, amountRaw: '-500000000000', decimals: 6 },
            ],
          },
        },
      ],
    });
  });

  test('sends only the client network when no filter is set', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return HISTORY;
    });

    const page = await swig.dex.transactions.list({ swigConfigAddress: SWIG });

    expect(requests[0].body).toStrictEqual({ network: 'NETWORK_MAINNET' });
    // The whole history fits the default page, so there is no next cursor.
    expect(Object.keys(page)).toEqual(['transactions']);
    expect(page.transactions).toHaveLength(24);
  });

  test('gets one transaction by signature', async () => {
    const signature =
      '5ic74krw5dwJAHpdagUqheKqj7cRE1wSTPTGzFjRSLFTewpGjsXA1DVk8C2d5QEJSefbjqBr7QczLLVfK7kbF9ra';
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return SWAP;
    });

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: SWIG,
      transactionSignature: signature,
    });

    expect(requests[0].method).toBe('GET');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/transactions/${signature}`,
    );
    expect(requests[0].url.search).toBe('?network=NETWORK_MAINNET');
    expect(transaction).toStrictEqual({
      swigConfigAddress: SWIG,
      transactionSignature: signature,
      slot: 449761692,
      blockTime: '2026-09-23T16:47:01Z',
      actions: [
        {
          actionIndex: 0,
          swigVaultAddress: VAULT,
          actingRoleId: 1,
          protocol: 'raydium-clmm',
          dexProgramAddress: CLMM_PROGRAM,
          action: {
            type: 'swap',
            protocolInstruction: 'SwapV2',
            protocolActorAddress: VAULT,
            dexPoolAddress: POOL,
            input: { mintAddress: USDV, amountRaw: '-85238320', decimals: 6 },
            output: { mintAddress: USDC, amountRaw: '85263893', decimals: 6 },
          },
        },
      ],
    });
  });

  test('lists positions with every filter and decodes open and closed', async () => {
    const requests: CapturedRequest[] = [];
    const swig = client((request) => {
      requests.push(request);
      return requests.length === 1 ? OPEN_POSITIONS : CLOSED_POSITIONS;
    });

    const open = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      pageSize: 25,
      positionStatus: 'open',
      protocol: 'raydium-clmm',
      swigVaultAddress: VAULT,
    });
    const closed = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      positionStatus: 'closed',
    });

    expect(requests[0].method).toBe('POST');
    expect(requests[0].url.pathname).toBe(
      `/wallet/api/swig/${SWIG}/dex/positions`,
    );
    expect(requests.map((request) => request.body)).toStrictEqual([
      {
        network: 'NETWORK_MAINNET',
        pageSize: 25,
        filters: [
          { positionStatus: 'POSITION_STATUS_OPEN' },
          { protocol: 'DEX_PROTOCOL_RAYDIUM_CLMM' },
          { swigVaultAddress: VAULT },
        ],
      },
      {
        network: 'NETWORK_MAINNET',
        filters: [{ positionStatus: 'POSITION_STATUS_CLOSED' }],
      },
    ]);
    expect(Object.keys(open)).toEqual(['positions']);
    expect(open.positions.map((position) => position.status)).toEqual([
      'open',
      'open',
      'open',
    ]);
    expect(open.positions[0]).toStrictEqual({
      swigConfigAddress: SWIG,
      swigVaultAddress: VAULT,
      protocol: 'raydium-clmm',
      dexPositionAddress: '8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH',
      updatedSlot: 463003457,
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
    });
    expect(closed.positions).toStrictEqual([
      {
        swigConfigAddress: SWIG,
        swigVaultAddress: VAULT,
        protocol: 'raydium-clmm',
        dexPositionAddress: 'GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv',
        updatedSlot: 463003458,
        sourceTransactionSignature:
          '7EzDhDeqh6m5Z196MCCGuwXyJgXMyjYWHkSvAa4HiCVCfrFqTrc8NUHpE4gwUtZ56mFnyNpRMPzumvwoNZoz4Re',
        status: 'closed',
      },
      {
        swigConfigAddress: SWIG,
        swigVaultAddress: VAULT,
        protocol: 'raydium-clmm',
        dexPositionAddress: 'HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp',
        updatedSlot: 463003456,
        sourceTransactionSignature:
          '5HsMkaQJXy1vZydQJXavx7keRtXV5BuR1pBQDJR93fhsfZMGYRTvFY13wm56JmEXg3uMP8ndUcUMeR8LpsznWhx1',
        status: 'closed',
      },
    ]);
  });

  test('pages by passing nextCursor back as cursor', async () => {
    const replies = [
      LIQUIDITY_ADDS,
      LIQUIDITY_ADDS_NEXT,
      POSITIONS_PAGE,
      POSITIONS_NEXT_PAGE,
    ];
    const bodies: unknown[] = [];
    const swig = client((request) => {
      bodies.push(request.body);
      return replies[bodies.length - 1];
    });

    const transactions = await swig.dex.transactions.list(LIQUIDITY_ADD_ARGS);
    const nextTransactions = await swig.dex.transactions.list({
      ...LIQUIDITY_ADD_ARGS,
      cursor: transactions.nextCursor,
    });
    const positions = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      pageSize: 2,
    });
    const nextPositions = await swig.dex.positions.list({
      swigConfigAddress: SWIG,
      pageSize: 2,
      cursor: positions.nextCursor,
    });

    // Each cursor goes back out exactly as the backend handed it over.
    expect(bodies).toStrictEqual([
      LIQUIDITY_ADD_BODY,
      { ...LIQUIDITY_ADD_BODY, cursor: LIQUIDITY_ADDS.nextCursor },
      { network: 'NETWORK_MAINNET', pageSize: 2 },
      {
        network: 'NETWORK_MAINNET',
        pageSize: 2,
        cursor: POSITIONS_PAGE.nextCursor,
      },
    ]);
    expect(
      nextTransactions.transactions.map((transaction) => transaction.slot),
    ).toEqual([449760470, 449755940]);
    expect(Object.keys(nextTransactions)).toEqual(['transactions']);
    expect(positions.nextCursor).toStrictEqual({
      protocol: 'raydium-clmm',
      dexPositionAddress: 'BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7',
    });
    expect(nextPositions.positions.map((position) => position.status)).toEqual([
      'closed',
      'open',
    ]);
    expect(nextPositions.nextCursor).toStrictEqual({
      protocol: 'raydium-clmm',
      dexPositionAddress: 'H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm',
    });
  });

  test('decodes every action kind in the recorded history', async () => {
    const page = await client(() => HISTORY).dex.transactions.list({
      swigConfigAddress: SWIG,
    });
    const actionOf = (signature: string) =>
      page.transactions.find(
        (transaction) => transaction.transactionSignature === signature,
      )?.actions[0];

    expect(
      [
        ...new Set(
          page.transactions.flatMap((transaction) =>
            transaction.actions.map((action) => action.action.type),
          ),
        ),
      ].sort(),
    ).toEqual([
      'liquidity-add',
      'liquidity-remove',
      'position-close',
      'position-open',
      'swap',
    ]);
    expect(
      actionOf(
        '2RVmaGZYw4wvKTnEz4ueYYjKYAWgA2dHdMeos9EJMrkEqDQCnBtk8hLGf5AkRw42b9H7K2aXC8KDXCtd9MmZ6L5j',
      ),
    ).toStrictEqual({
      actionIndex: 0,
      swigVaultAddress: VAULT,
      actingRoleId: 1,
      protocol: 'raydium-clmm',
      dexProgramAddress: CLMM_PROGRAM,
      action: {
        type: 'position-open',
        protocolInstruction: 'OpenPositionWithToken22Nft',
        protocolActorAddress: VAULT,
        dexPoolAddress: POOL,
        dexPositionAddress: 'BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7',
        deposits: [
          { mintAddress: USDV, amountRaw: '-940793839009', decimals: 6 },
        ],
      },
    });
    expect(
      actionOf(
        '4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V',
      ),
    ).toStrictEqual({
      actionIndex: 0,
      swigVaultAddress: VAULT,
      actingRoleId: 3,
      protocol: 'raydium-clmm',
      dexProgramAddress: CLMM_PROGRAM,
      action: {
        type: 'liquidity-remove',
        protocolInstruction: 'DecreaseLiquidityV2',
        protocolActorAddress: VAULT,
        dexPoolAddress: POOL,
        dexPositionAddress: '8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH',
        withdrawals: [{ mintAddress: USDC, amountRaw: '1092908', decimals: 6 }],
      },
    });
    // Raydium CLMM's ClosePosition names no pool. Role 0 is a real role.
    expect(
      actionOf(
        '5HsMkaQJXy1vZydQJXavx7keRtXV5BuR1pBQDJR93fhsfZMGYRTvFY13wm56JmEXg3uMP8ndUcUMeR8LpsznWhx1',
      ),
    ).toStrictEqual({
      actionIndex: 0,
      swigVaultAddress: VAULT,
      actingRoleId: 0,
      protocol: 'raydium-clmm',
      dexProgramAddress: CLMM_PROGRAM,
      action: {
        type: 'position-close',
        protocolInstruction: 'ClosePosition',
        protocolActorAddress: VAULT,
        dexPositionAddress: 'HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp',
      },
    });
  });

  test('decodes the action kinds no venue records yet', async () => {
    // Nothing in the recorded history carries these, so the bodies follow
    // swig_dex.proto rather than a capture. A close may also name its pool.
    const [swap] = SWAP.transaction?.actions ?? [];
    const position = {
      protocolActorAddress: VAULT,
      dexPoolAddress: POOL,
      dexPositionAddress: 'H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm',
    };
    const deposit = { mintAddress: USDV, amountRaw: '-1000000', decimals: 6 };
    const received = { mintAddress: USDC, amountRaw: '1000', decimals: 6 };
    const swig = client(() => ({
      transaction: {
        ...SWAP.transaction,
        actions: [
          {
            positionRebalance: {
              protocolInstruction: 'Rebalance',
              ...position,
              deposits: [deposit],
              withdrawals: [received],
            },
          },
          {
            feeClaim: {
              protocolInstruction: 'DecreaseLiquidityV2',
              ...position,
              claimed: [received],
            },
          },
          {
            rewardClaim: {
              protocolInstruction: 'CollectReward',
              ...position,
              claimed: [received],
            },
          },
          {
            positionClose: {
              protocolInstruction: 'ClosePosition',
              ...position,
            },
          },
        ].map((action, actionIndex) => ({ ...swap, actionIndex, action })),
      },
    }));

    const transaction = await swig.dex.transactions.get({
      swigConfigAddress: SWIG,
      transactionSignature: SWAP.transaction?.transactionSignature ?? '',
    });

    expect(transaction.actions.map((action) => action.action)).toStrictEqual([
      {
        type: 'position-rebalance',
        protocolInstruction: 'Rebalance',
        ...position,
        deposits: [deposit],
        withdrawals: [received],
      },
      {
        type: 'fee-claim',
        protocolInstruction: 'DecreaseLiquidityV2',
        ...position,
        claimed: [received],
      },
      {
        type: 'reward-claim',
        protocolInstruction: 'CollectReward',
        ...position,
        claimed: [received],
      },
      {
        type: 'position-close',
        protocolInstruction: 'ClosePosition',
        ...position,
      },
    ]);
  });

  test('sends and reads every enum value by its proto name', async () => {
    // Only the request matters for the filters, so an empty page answers them.
    const bodies: unknown[] = [];
    const swig = client((request) => {
      bodies.push(request.body);
      return request.url.pathname.endsWith('/dex/positions')
        ? NO_POSITIONS
        : NO_TRANSACTIONS;
    });
    for (const [protocol] of PROTOCOLS) {
      await swig.dex.transactions.list({ swigConfigAddress: SWIG, protocol });
    }
    for (const [actionType] of ACTION_TYPES) {
      await swig.dex.transactions.list({ swigConfigAddress: SWIG, actionType });
    }
    for (const [positionStatus] of POSITION_STATUSES) {
      await swig.dex.positions.list({
        swigConfigAddress: SWIG,
        positionStatus,
      });
    }

    expect(bodies).toStrictEqual([
      ...PROTOCOLS.map(([, protocol]) => ({
        network: 'NETWORK_MAINNET',
        filters: [{ protocol }],
      })),
      ...ACTION_TYPES.map(([, actionType]) => ({
        network: 'NETWORK_MAINNET',
        filters: [{ actionType }],
      })),
      ...POSITION_STATUSES.map(([, positionStatus]) => ({
        network: 'NETWORK_MAINNET',
        filters: [{ positionStatus }],
      })),
    ]);

    // Only Raydium CLMM is recorded, so the other names go on a captured action.
    const [first] = LIQUIDITY_ADDS.transactions ?? [];
    const page = await client(() => ({
      transactions: PROTOCOLS.map(([, protocol]) => ({
        ...first,
        actions: first?.actions?.map((action) => ({ ...action, protocol })),
      })),
    })).dex.transactions.list({ swigConfigAddress: SWIG });

    expect(
      page.transactions.map((transaction) => transaction.actions[0].protocol),
    ).toEqual(PROTOCOLS.map(([protocol]) => protocol));
  });

  test('retries a list on a transient failure, as it only reads', async () => {
    for (const [list, reply] of [
      [
        (swig: SwigClient) =>
          swig.dex.transactions.list({ swigConfigAddress: SWIG }),
        HISTORY,
      ],
      [
        (swig: SwigClient) =>
          swig.dex.positions.list({ swigConfigAddress: SWIG }),
        ALL_POSITIONS,
      ],
    ] as const) {
      let attempts = 0;
      const swig = client(
        () => {
          attempts += 1;
          return attempts === 1 ? new Response('', { status: 503 }) : reply;
        },
        'mainnet',
        { maxRetries: 1, retryDelay: 0 },
      );

      await list(swig);
      expect(attempts).toBe(2);
    }
  });

  test('refuses a response outside the contract', async () => {
    const [first] = LIQUIDITY_ADDS.transactions ?? [];
    const [added] = first?.actions ?? [];
    const withTransaction = (change: Record<string, unknown>) => ({
      transactions: [{ ...first, ...change }],
    });
    const withAction = (change: Record<string, unknown>) =>
      withTransaction({ actions: [{ ...added, ...change }] });

    for (const [response, message] of [
      [
        withAction({ protocol: 'DEX_PROTOCOL_UNSPECIFIED' }),
        /^DEX response has invalid protocol$/,
      ],
      [
        withAction({ protocol: 'DEX_PROTOCOL_UNISWAP' }),
        /^DEX response has invalid protocol$/,
      ],
      [
        withAction({ protocol: 'toString' }),
        /^DEX response has invalid protocol$/,
      ],
      [withAction({ action: {} }), /^DEX response has an unknown action$/],
      [
        withAction({
          action: {
            liquidityAdd: {
              ...added?.action?.liquidityAdd,
              deposits: [{ mintAddress: USDV, amountRaw: '1.5' }],
            },
          },
        }),
        /^DEX response has invalid amountRaw$/,
      ],
      [
        withAction({ actingRoleId: undefined }),
        /^DEX response has invalid actingRoleId$/,
      ],
      [
        withAction({ actingRoleId: -1 }),
        /^DEX response has invalid actingRoleId$/,
      ],
      [
        withTransaction({ blockTime: 1758649976 }),
        /^DEX response has invalid blockTime$/,
      ],
      // 2^53: past what a number holds exactly, so a cursor could not send it back.
      [
        withTransaction({ slot: '9007199254740992' }),
        /^DEX response has invalid slot$/,
      ],
      [
        { transactions: [], nextCursor: { slot: '449761733' } },
        /^DEX response is missing transactionSignature$/,
      ],
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
    ).rejects.toThrow(/^DEX response is missing transaction$/);

    const [closed] = CLOSED_POSITIONS.positions ?? [];
    for (const [response, message] of [
      // JSON drops an undefined value, so the key is absent on the wire.
      [
        { positions: [{ ...closed, closed: undefined }] },
        /^DEX response is missing position state$/,
      ],
      [
        { positions: [{ ...closed, closed: null }] },
        /^DEX response is missing position state$/,
      ],
      [
        {
          positions: [],
          nextCursor: {
            protocol: 'DEX_PROTOCOL_UNSPECIFIED',
            dexPositionAddress: closed?.dexPositionAddress,
          },
        },
        /^DEX response has invalid protocol$/,
      ],
    ] as const) {
      await expect(
        client(() => response).dex.positions.list({ swigConfigAddress: SWIG }),
      ).rejects.toThrow(message);
    }
  });

  test('refuses a malformed body instead of reading it as empty', async () => {
    const listTransactions = (reply: unknown) =>
      client(() => reply).dex.transactions.list({ swigConfigAddress: SWIG });
    const listPositions = (reply: unknown) =>
      client(() => reply).dex.positions.list({ swigConfigAddress: SWIG });
    const getTransaction = (reply: unknown) =>
      client(() => reply).dex.transactions.get({
        swigConfigAddress: SWIG,
        transactionSignature: 'signature',
      });

    // Each a 200: an upstream error page, an empty body, and JSON that is not
    // an object. A Response body reads once, so each call gets a new one.
    for (const reply of [
      () =>
        new Response('<html>Upstream unavailable</html>', {
          headers: { 'Content-Type': 'text/html' },
        }),
      () => new Response(null),
      () => false,
      () => [],
    ]) {
      await expect(listTransactions(reply())).rejects.toThrow(
        /^DEX transactions response must be an object$/,
      );
      await expect(listPositions(reply())).rejects.toThrow(
        /^DEX positions response must be an object$/,
      );
      await expect(getTransaction(reply())).rejects.toThrow(
        /^DEX transaction response must be an object$/,
      );
    }

    const [first] = LIQUIDITY_ADDS.transactions ?? [];
    const [added] = first?.actions ?? [];
    const [opened] = OPEN_POSITIONS.positions ?? [];
    for (const value of [false, {}, '']) {
      await expect(listTransactions({ transactions: value })).rejects.toThrow(
        /^DEX response has invalid transactions$/,
      );
      await expect(listPositions({ positions: value })).rejects.toThrow(
        /^DEX response has invalid positions$/,
      );
      await expect(
        listTransactions({ transactions: [{ ...first, actions: value }] }),
      ).rejects.toThrow(/^DEX response has invalid actions$/);
      await expect(
        listTransactions({
          transactions: [
            {
              ...first,
              actions: [
                {
                  ...added,
                  action: {
                    liquidityAdd: {
                      ...added?.action?.liquidityAdd,
                      deposits: value,
                    },
                  },
                },
              ],
            },
          ],
        }),
      ).rejects.toThrow(/^DEX response has invalid deposits$/);
      await expect(
        listPositions({
          positions: [
            {
              ...opened,
              open: {
                ...opened?.open,
                state: {
                  raydiumClmm: {
                    ...opened?.open?.state?.raydiumClmm,
                    rewardAmountsOwed: value,
                  },
                },
              },
            },
          ],
        }),
      ).rejects.toThrow(/^DEX response has invalid rewardAmountsOwed$/);
    }

    // A falsy cursor is not the last page.
    await expect(
      listTransactions({ transactions: [], nextCursor: false }),
    ).rejects.toThrow(/^DEX cursor must be an object$/);
    await expect(
      listPositions({ positions: [], nextCursor: '' }),
    ).rejects.toThrow(/^DEX cursor must be an object$/);
    await expect(
      listTransactions({ transactions: [{ ...first, blockTime: '' }] }),
    ).rejects.toThrow(/^DEX response has invalid blockTime$/);
  });

  test('reads an omitted or null field as absent', async () => {
    for (const reply of [{}, { transactions: null, nextCursor: null }]) {
      expect(
        await client(() => reply).dex.transactions.list({
          swigConfigAddress: SWIG,
        }),
      ).toStrictEqual({ transactions: [] });
    }
    for (const reply of [{}, { positions: null, nextCursor: null }]) {
      expect(
        await client(() => reply).dex.positions.list({
          swigConfigAddress: SWIG,
        }),
      ).toStrictEqual({ positions: [] });
    }

    const [first] = LIQUIDITY_ADDS.transactions ?? [];
    const page = await client(() => ({
      transactions: [{ ...first, blockTime: null, actions: null }],
    })).dex.transactions.list({ swigConfigAddress: SWIG });
    expect(page.transactions[0]).not.toHaveProperty('blockTime');
    expect(page.transactions[0]?.actions).toStrictEqual([]);
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
        /^protocol must be a supported DEX protocol$/,
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            actionType: 'stake' as never,
          }),
        /^actionType must be a supported DEX action type$/,
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            startSlot: -1,
          }),
        /^startSlot must be an integer from 0 to 9007199254740991$/,
      ],
      [
        () =>
          swig.dex.transactions.list({ swigConfigAddress: SWIG, endSlot: 1.5 }),
        /^endSlot must be an integer from 0 to 9007199254740991$/,
      ],
      [
        () =>
          swig.dex.transactions.list({
            swigConfigAddress: SWIG,
            cursor: { slot: -1, transactionSignature: 'signature' },
          }),
        /^cursor\.slot must be an integer from 0 to 9007199254740991$/,
      ],
      [
        () =>
          swig.dex.positions.list({
            swigConfigAddress: SWIG,
            positionStatus: 'pending' as never,
          }),
        /^positionStatus must be "open" or "closed"$/,
      ],
      [
        () =>
          swig.dex.positions.list({ swigConfigAddress: SWIG, pageSize: -1 }),
        /^pageSize must be an integer from 0 to 4294967295$/,
      ],
      [
        () =>
          swig.dex.positions.list({
            swigConfigAddress: SWIG,
            pageSize: 2 ** 32,
          }),
        /^pageSize must be an integer from 0 to 4294967295$/,
      ],
      [
        () =>
          client(unreachable, null).dex.positions.list({
            swigConfigAddress: SWIG,
          }),
        /^network is required$/,
      ],
    ] as const) {
      await expect(call()).rejects.toThrow(message);
    }
  });
});
