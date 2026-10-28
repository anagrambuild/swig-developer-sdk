import type { Network } from './common.js';

export type DexProtocol =
  | 'pump-swap'
  | 'meteora-dlmm'
  | 'raydium-amm-v4'
  | 'raydium-cpmm'
  | 'raydium-clmm'
  | 'orca-whirlpool';

export type DexActionType =
  | 'swap'
  | 'position-open'
  | 'liquidity-add'
  | 'liquidity-remove'
  | 'position-rebalance'
  | 'fee-claim'
  | 'reward-claim'
  | 'position-close';

export type DexPositionStatus = 'open' | 'closed';

/**
 * Signed for the action's `protocolActorAddress`: a negative amount left its
 * accounts. A decimal integer string, so a protocol u128 survives exactly.
 */
export interface DexAssetAmount {
  mintAddress: string;
  amountRaw: string;
  /** Absent when the transaction never states the mint's decimals. */
  decimals?: number;
}

interface DexActionBase {
  protocolInstruction: string;
  /** The Swig vault on a direct call; the router's account when an aggregator holds the assets. */
  protocolActorAddress: string;
}

export interface DexSwapAction extends DexActionBase {
  type: 'swap';
  dexPoolAddress: string;
  input: DexAssetAmount;
  output: DexAssetAmount;
}

export interface DexPositionOpenAction extends DexActionBase {
  type: 'position-open';
  dexPoolAddress: string;
  dexPositionAddress: string;
  deposits: DexAssetAmount[];
}

export interface DexLiquidityAddAction extends DexActionBase {
  type: 'liquidity-add';
  dexPoolAddress: string;
  dexPositionAddress: string;
  deposits: DexAssetAmount[];
}

export interface DexLiquidityRemoveAction extends DexActionBase {
  type: 'liquidity-remove';
  dexPoolAddress: string;
  dexPositionAddress: string;
  withdrawals: DexAssetAmount[];
}

export interface DexPositionRebalanceAction extends DexActionBase {
  type: 'position-rebalance';
  dexPoolAddress: string;
  dexPositionAddress: string;
  deposits: DexAssetAmount[];
  withdrawals: DexAssetAmount[];
}

export interface DexFeeClaimAction extends DexActionBase {
  type: 'fee-claim';
  dexPoolAddress: string;
  dexPositionAddress: string;
  claimed: DexAssetAmount[];
}

export interface DexRewardClaimAction extends DexActionBase {
  type: 'reward-claim';
  dexPoolAddress: string;
  dexPositionAddress: string;
  claimed: DexAssetAmount[];
}

export interface DexPositionCloseAction extends DexActionBase {
  type: 'position-close';
  /** Absent where the instruction names no pool, as Raydium CLMM's ClosePosition. */
  dexPoolAddress?: string;
  dexPositionAddress: string;
}

export type DexAction =
  | DexSwapAction
  | DexPositionOpenAction
  | DexLiquidityAddAction
  | DexLiquidityRemoveAction
  | DexPositionRebalanceAction
  | DexFeeClaimAction
  | DexRewardClaimAction
  | DexPositionCloseAction;

export interface DexTransactionAction {
  actionIndex: number;
  swigVaultAddress: string;
  /** The Swig role that authorized the action. */
  actingRoleId: number;
  protocol: DexProtocol;
  dexProgramAddress: string;
  action: DexAction;
}

/** One Solana transaction the DEX index interpreted for a Swig. */
export interface DexTransaction {
  swigConfigAddress: string;
  transactionSignature: string;
  slot: number;
  blockTime?: string;
  /** Every DEX action the Swig executed in the transaction, in execution order. */
  actions: DexTransactionAction[];
}

/**
 * Every filter is optional and they combine with AND. Slots are inclusive.
 * `protocol`, `actionType` and `swigVaultAddress` must match the same action;
 * a matching transaction keeps all of the Swig's actions.
 */
export interface ListDexTransactionsArgs {
  swigConfigAddress: string;
  network?: Network;
  pageSize?: number;
  pageToken?: string;
  startSlot?: number;
  endSlot?: number;
  protocol?: DexProtocol;
  actionType?: DexActionType;
  swigVaultAddress?: string;
}

export interface ListDexTransactionsResult {
  transactions: DexTransaction[];
  /** Absent on the last page; otherwise pass it back as `pageToken` with the same filters. */
  nextPageToken?: string;
}

export interface GetDexTransactionArgs {
  swigConfigAddress: string;
  network?: Network;
  transactionSignature: string;
}

/** Raydium CLMM's `PersonalPositionState`, named as the account names its fields. */
export interface DexRaydiumClmmPositionState {
  type: 'raydium-clmm';
  liquidityRaw: string;
  tickLowerIndex: number;
  tickUpperIndex: number;
  tokenFeesOwed0: string;
  tokenFeesOwed1: string;
  /** One per reward slot, in order. */
  rewardAmountsOwed: string[];
}

export type DexPositionAccountState = DexRaydiumClmmPositionState;

interface DexPositionBase {
  swigConfigAddress: string;
  swigVaultAddress: string;
  protocol: DexProtocol;
  dexPositionAddress: string;
  /** The slot of the on-chain read the state came from. */
  updatedSlot: number;
  sourceTransactionSignature: string;
}

export interface DexOpenPosition extends DexPositionBase {
  status: 'open';
  dexPoolAddress: string;
  state: DexPositionAccountState;
}

/** The position account no longer exists on chain. */
export interface DexClosedPosition extends DexPositionBase {
  status: 'closed';
}

export type DexPosition = DexOpenPosition | DexClosedPosition;

/** Every filter is optional, applies to the position, and they combine with AND. */
export interface ListDexPositionsArgs {
  swigConfigAddress: string;
  network?: Network;
  pageSize?: number;
  pageToken?: string;
  positionStatus?: DexPositionStatus;
  protocol?: DexProtocol;
  swigVaultAddress?: string;
}

export interface ListDexPositionsResult {
  positions: DexPosition[];
  /** Absent on the last page; otherwise pass it back as `pageToken` with the same filters. */
  nextPageToken?: string;
}

export interface DexAssetAmountWire {
  mintAddress?: string;
  amountRaw?: string;
  decimals?: number;
}

/** The fields every action message may carry; each action uses its own subset. */
export interface DexActionWire {
  protocolInstruction?: string;
  protocolActorAddress?: string;
  dexPoolAddress?: string;
  dexPositionAddress?: string;
  input?: DexAssetAmountWire;
  output?: DexAssetAmountWire;
  deposits?: DexAssetAmountWire[];
  withdrawals?: DexAssetAmountWire[];
  claimed?: DexAssetAmountWire[];
}

export interface DexActionPayloadWire {
  swap?: DexActionWire;
  positionOpen?: DexActionWire;
  liquidityAdd?: DexActionWire;
  liquidityRemove?: DexActionWire;
  positionRebalance?: DexActionWire;
  feeClaim?: DexActionWire;
  rewardClaim?: DexActionWire;
  positionClose?: DexActionWire;
}

export interface DexTransactionActionWire {
  actionIndex?: number;
  swigVaultAddress?: string;
  actingRoleId?: number;
  protocol?: string;
  dexProgramAddress?: string;
  action?: DexActionPayloadWire;
}

export interface DexTransactionWire {
  swigConfigAddress?: string;
  transactionSignature?: string;
  slot?: number | string;
  blockTime?: string;
  actions?: DexTransactionActionWire[];
}

export interface ListDexTransactionsWire {
  transactions?: DexTransactionWire[];
  nextPageToken?: string;
}

export interface GetDexTransactionWire {
  transaction?: DexTransactionWire;
}

export interface DexRaydiumClmmPositionWire {
  liquidityRaw?: string;
  tickLowerIndex?: number;
  tickUpperIndex?: number;
  tokenFeesOwed0?: string;
  tokenFeesOwed1?: string;
  rewardAmountsOwed?: string[];
}

export interface DexPositionWire {
  swigConfigAddress?: string;
  swigVaultAddress?: string;
  protocol?: string;
  dexPositionAddress?: string;
  open?: {
    dexPoolAddress?: string;
    state?: { raydiumClmm?: DexRaydiumClmmPositionWire };
  };
  closed?: Record<string, never>;
  updatedSlot?: number | string;
  sourceTransactionSignature?: string;
}

export interface ListDexPositionsWire {
  positions?: DexPositionWire[];
  nextPageToken?: string;
}
