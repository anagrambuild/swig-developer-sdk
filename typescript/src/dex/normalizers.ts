import type {
  DexAction,
  DexActionPayloadWire,
  DexActionWire,
  DexAssetAmount,
  DexAssetAmountWire,
  DexPosition,
  DexPositionAccountState,
  DexPositionWire,
  DexProtocol,
  DexTransaction,
  DexTransactionAction,
  DexTransactionActionWire,
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
  return {
    transactions: (response.transactions ?? []).map(normalizeDexTransaction),
    ...(response.nextPageToken
      ? { nextPageToken: response.nextPageToken }
      : {}),
  };
}

export function normalizeDexTransaction(
  transaction: DexTransactionWire,
): DexTransaction {
  return {
    swigConfigAddress: requiredString(
      transaction.swigConfigAddress,
      'swigConfigAddress',
    ),
    transactionSignature: requiredString(
      transaction.transactionSignature,
      'transactionSignature',
    ),
    slot: integerField(transaction.slot, 'slot'),
    ...(transaction.blockTime ? { blockTime: transaction.blockTime } : {}),
    actions: (transaction.actions ?? []).map(normalizeDexTransactionAction),
  };
}

function normalizeDexTransactionAction(
  action: DexTransactionActionWire,
): DexTransactionAction {
  return {
    actionIndex: integerField(action.actionIndex, 'actionIndex'),
    swigVaultAddress: requiredString(
      action.swigVaultAddress,
      'swigVaultAddress',
    ),
    actingRoleId: integerField(action.actingRoleId, 'actingRoleId'),
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
      deposits: amounts(wire.positionOpen.deposits),
    };
  }
  if (wire.liquidityAdd) {
    return {
      type: 'liquidity-add',
      ...positionActionBase(wire.liquidityAdd),
      deposits: amounts(wire.liquidityAdd.deposits),
    };
  }
  if (wire.liquidityRemove) {
    return {
      type: 'liquidity-remove',
      ...positionActionBase(wire.liquidityRemove),
      withdrawals: amounts(wire.liquidityRemove.withdrawals),
    };
  }
  if (wire.positionRebalance) {
    return {
      type: 'position-rebalance',
      ...positionActionBase(wire.positionRebalance),
      deposits: amounts(wire.positionRebalance.deposits),
      withdrawals: amounts(wire.positionRebalance.withdrawals),
    };
  }
  if (wire.feeClaim) {
    return {
      type: 'fee-claim',
      ...positionActionBase(wire.feeClaim),
      claimed: amounts(wire.feeClaim.claimed),
    };
  }
  if (wire.rewardClaim) {
    return {
      type: 'reward-claim',
      ...positionActionBase(wire.rewardClaim),
      claimed: amounts(wire.rewardClaim.claimed),
    };
  }
  if (wire.positionClose) {
    const { dexPoolAddress } = wire.positionClose;
    return {
      type: 'position-close',
      ...actionBase(wire.positionClose),
      ...(dexPoolAddress ? { dexPoolAddress } : {}),
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

function amounts(list?: DexAssetAmountWire[]): DexAssetAmount[] {
  return (list ?? []).map((amount) =>
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
      : { decimals: integerField(wire.decimals, 'decimals') }),
  };
}

export function normalizeDexPositionsPage(
  response: ListDexPositionsWire,
): ListDexPositionsResult {
  return {
    positions: (response.positions ?? []).map(normalizeDexPosition),
    ...(response.nextPageToken
      ? { nextPageToken: response.nextPageToken }
      : {}),
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
    updatedSlot: integerField(position.updatedSlot, 'updatedSlot'),
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
  // Key presence, not truthiness: a closed position arrives as an empty object.
  if ('closed' in position) return { ...base, status: 'closed' };
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
    rewardAmountsOwed: (clmm.rewardAmountsOwed ?? []).map((amount) =>
      unsignedUnitsField(amount, 'rewardAmountsOwed'),
    ),
  };
}

function normalizeDexProtocol(value?: string): DexProtocol {
  const protocol = value ? PROTOCOLS[value] : undefined;
  if (!protocol) throw new Error('DEX response has invalid protocol');
  return protocol;
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

/** uint64 arrives as a decimal string and 32-bit integers as numbers. */
function integerField(value: unknown, field: string): number {
  const parsed =
    typeof value === 'string' && /^-?\d+$/.test(value) ? Number(value) : value;
  if (typeof parsed !== 'number' || !Number.isSafeInteger(parsed)) {
    throw new Error(`DEX response has invalid ${field}`);
  }
  return parsed;
}
