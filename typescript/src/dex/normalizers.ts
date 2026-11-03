import type {
  DexAction,
  DexActionPayloadWire,
  DexActionWire,
  DexAssetAmount,
  DexAssetAmountWire,
  DexPosition,
  DexPositionAccountState,
  DexPositionCursor,
  DexPositionCursorWire,
  DexPositionWire,
  DexProtocol,
  DexTransaction,
  DexTransactionAction,
  DexTransactionActionWire,
  DexTransactionCursor,
  DexTransactionCursorWire,
  DexTransactionWire,
  ListDexPositionsResult,
  ListDexPositionsWire,
  ListDexTransactionsResult,
  ListDexTransactionsWire,
} from '../types/index.js';

const PROTOCOLS: Record<string, DexProtocol> = {
  DEX_PROTOCOL_PUMP_SWAP: 'pump-swap',
  DEX_PROTOCOL_METEORA_DLMM: 'meteora-dlmm',
  DEX_PROTOCOL_RAYDIUM_AMM_V4: 'raydium-amm-v4',
  DEX_PROTOCOL_RAYDIUM_CPMM: 'raydium-cpmm',
  DEX_PROTOCOL_RAYDIUM_CLMM: 'raydium-clmm',
  DEX_PROTOCOL_ORCA_WHIRLPOOL: 'orca-whirlpool',
};

export function normalizeDexTransactionsPage(
  response: ListDexTransactionsWire,
): ListDexTransactionsResult {
  assertObject(response, 'DEX transactions response');
  const { nextCursor } = response;
  return {
    transactions: repeatedField(response.transactions, 'transactions').map(
      normalizeDexTransaction,
    ),
    ...(nextCursor === undefined || nextCursor === null
      ? {}
      : { nextCursor: normalizeDexTransactionCursor(nextCursor) }),
  };
}

function normalizeDexTransactionCursor(
  cursor: DexTransactionCursorWire,
): DexTransactionCursor {
  assertObject(cursor, 'DEX cursor');
  return {
    slot: unsignedIntegerField(cursor.slot, 'slot'),
    transactionSignature: requiredString(
      cursor.transactionSignature,
      'transactionSignature',
    ),
  };
}

export function normalizeDexTransaction(
  transaction: DexTransactionWire,
): DexTransaction {
  const blockTime = transaction.blockTime ?? undefined;
  if (
    blockTime !== undefined &&
    (typeof blockTime !== 'string' || blockTime === '')
  ) {
    throw new Error('DEX response has invalid blockTime');
  }
  return {
    swigConfigAddress: requiredString(
      transaction.swigConfigAddress,
      'swigConfigAddress',
    ),
    transactionSignature: requiredString(
      transaction.transactionSignature,
      'transactionSignature',
    ),
    slot: unsignedIntegerField(transaction.slot, 'slot'),
    ...(blockTime === undefined ? {} : { blockTime }),
    actions: repeatedField(transaction.actions, 'actions').map(
      normalizeDexTransactionAction,
    ),
  };
}

function normalizeDexTransactionAction(
  action: DexTransactionActionWire,
): DexTransactionAction {
  return {
    actionIndex: unsignedIntegerField(action.actionIndex, 'actionIndex'),
    swigVaultAddress: requiredString(
      action.swigVaultAddress,
      'swigVaultAddress',
    ),
    actingRoleId: unsignedIntegerField(action.actingRoleId, 'actingRoleId'),
    protocol: normalizeDexProtocol(action.protocol),
    dexProgramAddress: requiredString(
      action.dexProgramAddress,
      'dexProgramAddress',
    ),
    action: normalizeDexAction(action.action),
  };
}

/** The payload's one key names the action, so the type cannot disagree with it. */
function normalizeDexAction(payload?: DexActionPayloadWire): DexAction {
  const wire = requiredField(payload, 'action');
  if (wire.swap) {
    return {
      type: 'swap',
      ...actionBase(wire.swap),
      dexPoolAddress: requiredString(
        wire.swap.dexPoolAddress,
        'dexPoolAddress',
      ),
      input: normalizeDexAssetAmount(wire.swap.input, 'input'),
      output: normalizeDexAssetAmount(wire.swap.output, 'output'),
    };
  }
  if (wire.positionOpen) {
    return {
      type: 'position-open',
      ...positionActionBase(wire.positionOpen),
      deposits: amounts(wire.positionOpen.deposits, 'deposits'),
    };
  }
  if (wire.liquidityAdd) {
    return {
      type: 'liquidity-add',
      ...positionActionBase(wire.liquidityAdd),
      deposits: amounts(wire.liquidityAdd.deposits, 'deposits'),
    };
  }
  if (wire.liquidityRemove) {
    return {
      type: 'liquidity-remove',
      ...positionActionBase(wire.liquidityRemove),
      withdrawals: amounts(wire.liquidityRemove.withdrawals, 'withdrawals'),
    };
  }
  if (wire.positionRebalance) {
    return {
      type: 'position-rebalance',
      ...positionActionBase(wire.positionRebalance),
      deposits: amounts(wire.positionRebalance.deposits, 'deposits'),
      withdrawals: amounts(wire.positionRebalance.withdrawals, 'withdrawals'),
    };
  }
  if (wire.feeClaim) {
    return {
      type: 'fee-claim',
      ...positionActionBase(wire.feeClaim),
      claimed: amounts(wire.feeClaim.claimed, 'claimed'),
    };
  }
  if (wire.rewardClaim) {
    return {
      type: 'reward-claim',
      ...positionActionBase(wire.rewardClaim),
      claimed: amounts(wire.rewardClaim.claimed, 'claimed'),
    };
  }
  if (wire.positionClose) {
    const { dexPoolAddress } = wire.positionClose;
    return {
      type: 'position-close',
      ...actionBase(wire.positionClose),
      ...(dexPoolAddress === undefined
        ? {}
        : { dexPoolAddress: requiredString(dexPoolAddress, 'dexPoolAddress') }),
      dexPositionAddress: requiredString(
        wire.positionClose.dexPositionAddress,
        'dexPositionAddress',
      ),
    };
  }
  throw new Error('DEX response has an unknown action');
}

function actionBase(action: DexActionWire) {
  return {
    protocolInstruction: requiredString(
      action.protocolInstruction,
      'protocolInstruction',
    ),
    protocolActorAddress: requiredString(
      action.protocolActorAddress,
      'protocolActorAddress',
    ),
  };
}

function positionActionBase(action: DexActionWire) {
  return {
    ...actionBase(action),
    dexPoolAddress: requiredString(action.dexPoolAddress, 'dexPoolAddress'),
    dexPositionAddress: requiredString(
      action.dexPositionAddress,
      'dexPositionAddress',
    ),
  };
}

function amounts(
  list: DexAssetAmountWire[] | undefined,
  field: string,
): DexAssetAmount[] {
  return repeatedField(list, field).map((amount) =>
    normalizeDexAssetAmount(amount, 'amount'),
  );
}

function normalizeDexAssetAmount(
  amount: DexAssetAmountWire | undefined,
  field: string,
): DexAssetAmount {
  const wire = requiredField(amount, field);
  return {
    mintAddress: requiredString(wire.mintAddress, 'mintAddress'),
    amountRaw: signedUnitsField(wire.amountRaw, 'amountRaw'),
    ...(wire.decimals === undefined
      ? {}
      : { decimals: unsignedIntegerField(wire.decimals, 'decimals') }),
  };
}

export function normalizeDexPositionsPage(
  response: ListDexPositionsWire,
): ListDexPositionsResult {
  assertObject(response, 'DEX positions response');
  const { nextCursor } = response;
  return {
    positions: repeatedField(response.positions, 'positions').map(
      normalizeDexPosition,
    ),
    ...(nextCursor === undefined || nextCursor === null
      ? {}
      : { nextCursor: normalizeDexPositionCursor(nextCursor) }),
  };
}

function normalizeDexPositionCursor(
  cursor: DexPositionCursorWire,
): DexPositionCursor {
  assertObject(cursor, 'DEX cursor');
  return {
    protocol: normalizeDexProtocol(cursor.protocol),
    dexPositionAddress: requiredString(
      cursor.dexPositionAddress,
      'dexPositionAddress',
    ),
  };
}

function normalizeDexPosition(position: DexPositionWire): DexPosition {
  const base = {
    swigConfigAddress: requiredString(
      position.swigConfigAddress,
      'swigConfigAddress',
    ),
    swigVaultAddress: requiredString(
      position.swigVaultAddress,
      'swigVaultAddress',
    ),
    protocol: normalizeDexProtocol(position.protocol),
    dexPositionAddress: requiredString(
      position.dexPositionAddress,
      'dexPositionAddress',
    ),
    updatedSlot: unsignedIntegerField(position.updatedSlot, 'updatedSlot'),
    sourceTransactionSignature: requiredString(
      position.sourceTransactionSignature,
      'sourceTransactionSignature',
    ),
  };
  if (position.open) {
    return {
      ...base,
      status: 'open',
      dexPoolAddress: requiredString(
        position.open.dexPoolAddress,
        'dexPoolAddress',
      ),
      state: normalizeDexPositionAccountState(position.open.state),
    };
  }
  if (position.closed) return { ...base, status: 'closed' };
  throw new Error('DEX response is missing position state');
}

function normalizeDexPositionAccountState(
  state: NonNullable<DexPositionWire['open']>['state'],
): DexPositionAccountState {
  const clmm = state?.raydiumClmm;
  if (!clmm) throw new Error('DEX response is missing position account state');
  return {
    type: 'raydium-clmm',
    liquidityRaw: unsignedUnitsField(clmm.liquidityRaw, 'liquidityRaw'),
    tickLowerIndex: integerField(clmm.tickLowerIndex, 'tickLowerIndex'),
    tickUpperIndex: integerField(clmm.tickUpperIndex, 'tickUpperIndex'),
    tokenFeesOwed0: unsignedUnitsField(clmm.tokenFeesOwed0, 'tokenFeesOwed0'),
    tokenFeesOwed1: unsignedUnitsField(clmm.tokenFeesOwed1, 'tokenFeesOwed1'),
    rewardAmountsOwed: repeatedField(
      clmm.rewardAmountsOwed,
      'rewardAmountsOwed',
    ).map((amount) => unsignedUnitsField(amount, 'rewardAmountsOwed')),
  };
}

function normalizeDexProtocol(value?: string): DexProtocol {
  // Own keys only, so a name such as `toString` is not read off the prototype.
  if (value === undefined || !Object.hasOwn(PROTOCOLS, value)) {
    throw new Error('DEX response has invalid protocol');
  }
  return PROTOCOLS[value];
}

/** An HTML error page, `false` or `[]` is not a response, even with a 200. */
export function assertObject(
  value: unknown,
  label: string,
): asserts value is object {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
}

export function requiredField<TValue>(
  value: TValue | undefined,
  field: string,
): TValue {
  if (value === undefined || value === null) {
    throw new Error(`DEX response is missing ${field}`);
  }
  return value;
}

/** A repeated field may be omitted or null; any value that is present must be an array. */
function repeatedField<TItem>(
  value: TItem[] | undefined,
  field: string,
): TItem[] {
  if (value === undefined || value === null) return [];
  if (!Array.isArray(value)) {
    throw new Error(`DEX response has invalid ${field}`);
  }
  return value;
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`DEX response is missing ${field}`);
  }
  return value;
}

/** A decimal integer string, kept as one so a value past 2^53 survives. */
function unsignedUnitsField(value: unknown, field: string): string {
  if (typeof value !== 'string' || !/^\d+$/.test(value)) {
    throw new Error(`DEX response has invalid ${field}`);
  }
  return value;
}

function signedUnitsField(value: unknown, field: string): string {
  if (typeof value !== 'string' || !/^-?\d+$/.test(value)) {
    throw new Error(`DEX response has invalid ${field}`);
  }
  return value;
}

/** uint64 arrives as a decimal string and uint32 as a number. */
function unsignedIntegerField(value: unknown, field: string): number {
  const parsed = integerField(value, field);
  if (parsed < 0) throw new Error(`DEX response has invalid ${field}`);
  return parsed;
}

function integerField(value: unknown, field: string): number {
  const parsed =
    typeof value === 'string' && /^-?\d+$/.test(value) ? Number(value) : value;
  if (typeof parsed !== 'number' || !Number.isSafeInteger(parsed)) {
    throw new Error(`DEX response has invalid ${field}`);
  }
  return parsed;
}
