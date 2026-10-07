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
  const nextPageToken = response.nextPageToken ?? response.next_page_token;
  return {
    transactions: (response.transactions ?? []).map(normalizeDexTransaction),
    ...(nextPageToken ? { nextPageToken } : {}),
  };
}

export function normalizeDexTransaction(
  transaction: DexTransactionWire,
): DexTransaction {
  const blockTime = transaction.blockTime ?? transaction.block_time;
  return {
    swigConfigAddress: requiredString(
      transaction.swigConfigAddress ?? transaction.swig_config_address,
      'swigConfigAddress',
    ),
    transactionSignature: requiredString(
      transaction.transactionSignature ?? transaction.transaction_signature,
      'transactionSignature',
    ),
    slot: integerField(transaction.slot, 'slot'),
    ...(blockTime ? { blockTime } : {}),
    actions: (transaction.actions ?? []).map(normalizeDexTransactionAction),
  };
}

function normalizeDexTransactionAction(
  action: DexTransactionActionWire,
): DexTransactionAction {
  return {
    actionIndex: integerField(
      action.actionIndex ?? action.action_index,
      'actionIndex',
    ),
    swigVaultAddress: requiredString(
      action.swigVaultAddress ?? action.swig_vault_address,
      'swigVaultAddress',
    ),
    actingRoleId: integerField(
      action.actingRoleId ?? action.acting_role_id,
      'actingRoleId',
    ),
    protocol: normalizeDexProtocol(action.protocol),
    dexProgramAddress: requiredString(
      action.dexProgramAddress ?? action.dex_program_address,
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
      dexPoolAddress: poolAddress(wire.swap),
      input: normalizeDexAssetAmount(wire.swap.input, 'input'),
      output: normalizeDexAssetAmount(wire.swap.output, 'output'),
    };
  }
  const positionOpen = wire.positionOpen ?? wire.position_open;
  if (positionOpen) {
    return {
      type: 'position-open',
      ...positionActionBase(positionOpen),
      deposits: amounts(positionOpen.deposits),
    };
  }
  const liquidityAdd = wire.liquidityAdd ?? wire.liquidity_add;
  if (liquidityAdd) {
    return {
      type: 'liquidity-add',
      ...positionActionBase(liquidityAdd),
      deposits: amounts(liquidityAdd.deposits),
    };
  }
  const liquidityRemove = wire.liquidityRemove ?? wire.liquidity_remove;
  if (liquidityRemove) {
    return {
      type: 'liquidity-remove',
      ...positionActionBase(liquidityRemove),
      withdrawals: amounts(liquidityRemove.withdrawals),
    };
  }
  const rebalance = wire.positionRebalance ?? wire.position_rebalance;
  if (rebalance) {
    return {
      type: 'position-rebalance',
      ...positionActionBase(rebalance),
      deposits: amounts(rebalance.deposits),
      withdrawals: amounts(rebalance.withdrawals),
    };
  }
  const feeClaim = wire.feeClaim ?? wire.fee_claim;
  if (feeClaim) {
    return {
      type: 'fee-claim',
      ...positionActionBase(feeClaim),
      claimed: amounts(feeClaim.claimed),
    };
  }
  const rewardClaim = wire.rewardClaim ?? wire.reward_claim;
  if (rewardClaim) {
    return {
      type: 'reward-claim',
      ...positionActionBase(rewardClaim),
      claimed: amounts(rewardClaim.claimed),
    };
  }
  const positionClose = wire.positionClose ?? wire.position_close;
  if (positionClose) {
    const dexPoolAddress =
      positionClose.dexPoolAddress ?? positionClose.dex_pool_address;
    return {
      type: 'position-close',
      ...actionBase(positionClose),
      ...(dexPoolAddress ? { dexPoolAddress } : {}),
      dexPositionAddress: positionAddress(positionClose),
    };
  }
  throw new Error('DEX response has an unknown action');
}

function actionBase(action: DexActionWire) {
  return {
    protocolInstruction: requiredString(
      action.protocolInstruction ?? action.protocol_instruction,
      'protocolInstruction',
    ),
    protocolActorAddress: requiredString(
      action.protocolActorAddress ?? action.protocol_actor_address,
      'protocolActorAddress',
    ),
  };
}

function positionActionBase(action: DexActionWire) {
  return {
    ...actionBase(action),
    dexPoolAddress: poolAddress(action),
    dexPositionAddress: positionAddress(action),
  };
}

function poolAddress(action: DexActionWire): string {
  return requiredString(
    action.dexPoolAddress ?? action.dex_pool_address,
    'dexPoolAddress',
  );
}

function positionAddress(action: DexActionWire): string {
  return requiredString(
    action.dexPositionAddress ?? action.dex_position_address,
    'dexPositionAddress',
  );
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
  const decimals = wire.decimals;
  return {
    mintAddress: requiredString(
      wire.mintAddress ?? wire.mint_address,
      'mintAddress',
    ),
    amountRaw: signedUnitsField(wire.amountRaw ?? wire.amount_raw, 'amountRaw'),
    ...(decimals === undefined
      ? {}
      : { decimals: integerField(decimals, 'decimals') }),
  };
}

export function normalizeDexPositionsPage(
  response: ListDexPositionsWire,
): ListDexPositionsResult {
  const nextPageToken = response.nextPageToken ?? response.next_page_token;
  return {
    positions: (response.positions ?? []).map(normalizeDexPosition),
    ...(nextPageToken ? { nextPageToken } : {}),
  };
}

function normalizeDexPosition(position: DexPositionWire): DexPosition {
  const base = {
    swigConfigAddress: requiredString(
      position.swigConfigAddress ?? position.swig_config_address,
      'swigConfigAddress',
    ),
    swigVaultAddress: requiredString(
      position.swigVaultAddress ?? position.swig_vault_address,
      'swigVaultAddress',
    ),
    protocol: normalizeDexProtocol(position.protocol),
    dexPositionAddress: requiredString(
      position.dexPositionAddress ?? position.dex_position_address,
      'dexPositionAddress',
    ),
    observedSlot: integerField(
      position.observedSlot ?? position.observed_slot,
      'observedSlot',
    ),
    sourceTransactionSignature: requiredString(
      position.sourceTransactionSignature ??
        position.source_transaction_signature,
      'sourceTransactionSignature',
    ),
  };
  if (position.open) {
    return {
      ...base,
      status: 'open',
      dexPoolAddress: requiredString(
        position.open.dexPoolAddress ?? position.open.dex_pool_address,
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
  const clmm = state?.raydiumClmm ?? state?.raydium_clmm;
  if (!clmm) throw new Error('DEX response is missing position account state');
  return {
    type: 'raydium-clmm',
    liquidityRaw: unsignedUnitsField(
      clmm.liquidityRaw ?? clmm.liquidity_raw,
      'liquidityRaw',
    ),
    tickLowerIndex: integerField(
      clmm.tickLowerIndex ?? clmm.tick_lower_index,
      'tickLowerIndex',
    ),
    tickUpperIndex: integerField(
      clmm.tickUpperIndex ?? clmm.tick_upper_index,
      'tickUpperIndex',
    ),
    tokenFeesOwed0: unsignedUnitsField(
      clmm.tokenFeesOwed0 ?? clmm.token_fees_owed_0,
      'tokenFeesOwed0',
    ),
    tokenFeesOwed1: unsignedUnitsField(
      clmm.tokenFeesOwed1 ?? clmm.token_fees_owed_1,
      'tokenFeesOwed1',
    ),
    rewardAmountsOwed: (
      clmm.rewardAmountsOwed ??
      clmm.reward_amounts_owed ??
      []
    ).map((amount) => unsignedUnitsField(amount, 'rewardAmountsOwed')),
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
