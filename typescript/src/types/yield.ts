import type { Amount, Network } from './common.js';
import type {
  PreparedTransaction,
  PreparedTransactionWire,
} from './transaction.js';
import type { AddRoleAction, WalletAuthority } from './wallet-actions.js';

/**
 * Yield is the umbrella for earning on a Swig wallet. Native Solana staking to
 * an allowlisted validator is the first method; every call is scoped to one
 * Swig wallet config, and transactions are prepared unsigned for the caller to
 * authorize, sign and submit.
 */

/** Identifies the wallet a yield call acts on. */
export interface YieldWalletArgs {
  swigConfigAddress: string;
  network?: Network;
}

/** Wallet, fee payer and the authority that signs a yield transaction. */
export interface YieldSignerArgs extends YieldWalletArgs {
  feePayer: string;
  requesterAuthority: WalletAuthority;
  /** Role to act with. Omitted: the backend picks the lowest capable role. */
  roleId?: number;
}

export interface PrepareYieldRoleArgs extends YieldWalletArgs {
  feePayer: string;
  /** Authority that will sign deposits, exits and withdrawals. */
  yieldAuthority: WalletAuthority;
  validatorVoteAddress: string;
  /** Funding the role must still allow, and the cap a new role receives. */
  maxDeposit: Amount;
  /** Candidate role to validate and reuse. */
  roleId?: number;
  /** Role holding All or ManageAuthority. Required only to create a role. */
  roleManagerAuthority?: WalletAuthority;
}

export type YieldRoleResult =
  | {
      type: 'existing';
      roleId: number;
      /** Yield-relevant permissions the role currently holds. */
      actions: AddRoleAction[];
    }
  | {
      type: 'create';
      /** Unsigned AddRole transaction for the role manager and fee payer. */
      transaction: PreparedTransaction;
      yieldAuthority: WalletAuthority;
      actions: AddRoleAction[];
    };

export interface PrepareYieldDepositArgs extends YieldSignerArgs {
  /** Principal entering yield, excluding rent and fees. */
  amount: Amount;
  validatorVoteAddress: string;
  /** ASCII, 1..32 bytes. Reuse it when retrying the same deposit. */
  positionSeed: string;
}

export interface PreparedYieldDeposit {
  transaction: PreparedTransaction;
  positionAddress: string;
  validatorVoteAddress: string;
  delegatedLamports: string;
  /** Rent the fee payer funds for the new position. */
  rentLamports: string;
}

export type YieldExitAmount =
  | 'all'
  | {
      /** Delegated stake to exit; it and the remainder must meet the network minimum. */
      lamports: Amount;
      /** ASCII, 1..32 bytes, for the new exiting position. */
      positionSeed: string;
    };

export interface PrepareYieldExitArgs extends YieldSignerArgs {
  positionAddress: string;
  amount: YieldExitAmount;
}

export interface PreparedYieldExit {
  transaction: PreparedTransaction;
  /** The position entering cooldown: the original, or the split for partial exits. */
  exitingPositionAddress: string;
  deactivatingLamports: string;
  additionalRentLamports: string;
}

export interface PrepareYieldWithdrawalArgs extends YieldSignerArgs {
  positionAddress: string;
}

export interface PreparedYieldWithdrawal {
  transaction: PreparedTransaction;
  withdrawalLamports: string;
  destinationWalletAddress: string;
}

export interface YieldRate {
  validatorVoteAddress: string;
  /** 600 = 6% APR, after validator commission. */
  estimatedNetAprBps: number;
  commissionBps: number;
  /** When the rate source observed this value. */
  lastUpdatedAt?: string;
}

export interface YieldOption {
  voteAddress: string;
  name: string;
  acceptsDeposits: boolean;
  rate?: YieldRate;
}

export type YieldPositionState =
  'activating' | 'active' | 'deactivating' | 'inactive' | 'unspecified';

export interface YieldPosition {
  swigConfigAddress: string;
  walletAddress: string;
  positionAddress: string;
  validatorVoteAddress?: string;
  state: YieldPositionState;
  balanceLamports: string;
  rentLamports: string;
  /** Balance excluding rent. */
  positionLamports: string;
  withdrawableLamports: string;
  earningLamports: string;
}

export interface YieldPageArgs extends YieldWalletArgs {
  /** Default 50, maximum 200. */
  pageSize?: number;
  pageToken?: string;
}

export interface YieldPositionsPage {
  positions: YieldPosition[];
  nextPageToken?: string;
  /** Latest indexed update, absent before the first observation. */
  lastUpdatedAt?: string;
}

export interface YieldSummary {
  /** SOL in positions excluding rent; rewards are already included. */
  totalPositionLamports: string;
  /** Withdrawable SOL, including returned rent. */
  withdrawableLamports: string;
  recordedRewardsLamports: string;
}

export interface YieldReward {
  swigConfigAddress: string;
  positionAddress: string;
  amountLamports: string;
  /** When the reward was credited on-chain. */
  earnedAt?: string;
}

export interface YieldRewardsPage {
  rewards: YieldReward[];
  nextPageToken?: string;
}

export interface EstimateYieldArgs extends YieldWalletArgs {
  validatorVoteAddress: string;
  principal: Amount;
  /** 1..365 earning days, starting once the position is fully active. */
  horizonDays: number;
}

export interface YieldProjection {
  principalLamports: string;
  horizonDays: number;
  estimatedRewardsLamports: string;
  estimatedTotalLamports: string;
  rate?: YieldRate;
}

// Wire shapes accept the gRPC-JSON transcoder's camelCase and snake_case.

type U64Wire = string | number;

export type AddRoleActionWire = Record<
  string,
  Record<string, unknown> | undefined
>;

export interface PrepareYieldRoleResponseWire {
  existing?: {
    roleId?: number;
    role_id?: number;
    actions?: AddRoleActionWire[];
  };
  create?: {
    transaction?: PreparedTransactionWire;
    yieldAuthority?: WalletAuthority;
    yield_authority?: WalletAuthority;
    actions?: AddRoleActionWire[];
  };
}

export interface PrepareYieldDepositResponseWire {
  transaction?: PreparedTransactionWire;
  positionAddress?: string;
  position_address?: string;
  validatorVoteAddress?: string;
  validator_vote_address?: string;
  delegatedLamports?: U64Wire;
  delegated_lamports?: U64Wire;
  rentLamports?: U64Wire;
  rent_lamports?: U64Wire;
}

export interface PrepareYieldExitResponseWire {
  transaction?: PreparedTransactionWire;
  exitingPositionAddress?: string;
  exiting_position_address?: string;
  deactivatingLamports?: U64Wire;
  deactivating_lamports?: U64Wire;
  additionalRentLamports?: U64Wire;
  additional_rent_lamports?: U64Wire;
}

export interface PrepareYieldWithdrawalResponseWire {
  transaction?: PreparedTransactionWire;
  withdrawalLamports?: U64Wire;
  withdrawal_lamports?: U64Wire;
  destinationWalletAddress?: string;
  destination_wallet_address?: string;
}

export interface YieldRateWire {
  validatorVoteAddress?: string;
  validator_vote_address?: string;
  estimatedNetAprBps?: number;
  estimated_net_apr_bps?: number;
  commissionBps?: number;
  commission_bps?: number;
  lastUpdatedAt?: string;
  last_updated_at?: string;
}

export interface ListYieldOptionsResponseWire {
  options?: Array<{
    voteAddress?: string;
    vote_address?: string;
    name?: string;
    acceptsDeposits?: boolean;
    accepts_deposits?: boolean;
    rate?: YieldRateWire;
  }>;
}

export interface YieldPositionWire {
  swigConfigAddress?: string;
  swig_config_address?: string;
  walletAddress?: string;
  wallet_address?: string;
  positionAddress?: string;
  position_address?: string;
  validatorVoteAddress?: string;
  validator_vote_address?: string;
  state?: string | number;
  balanceLamports?: U64Wire;
  balance_lamports?: U64Wire;
  rentLamports?: U64Wire;
  rent_lamports?: U64Wire;
  positionLamports?: U64Wire;
  position_lamports?: U64Wire;
  withdrawableLamports?: U64Wire;
  withdrawable_lamports?: U64Wire;
  earningLamports?: U64Wire;
  earning_lamports?: U64Wire;
}

export interface ListYieldPositionsResponseWire {
  positions?: YieldPositionWire[];
  nextPageToken?: string;
  next_page_token?: string;
  lastUpdatedAt?: string;
  last_updated_at?: string;
}

export interface GetYieldSummaryResponseWire {
  totalPositionLamports?: U64Wire;
  total_position_lamports?: U64Wire;
  withdrawableLamports?: U64Wire;
  withdrawable_lamports?: U64Wire;
  recordedRewardsLamports?: U64Wire;
  recorded_rewards_lamports?: U64Wire;
}

export interface ListYieldRewardsResponseWire {
  rewards?: Array<{
    swigConfigAddress?: string;
    swig_config_address?: string;
    positionAddress?: string;
    position_address?: string;
    amountLamports?: U64Wire;
    amount_lamports?: U64Wire;
    earnedAt?: string;
    earned_at?: string;
  }>;
  nextPageToken?: string;
  next_page_token?: string;
}

export interface EstimateYieldResponseWire {
  projection?: {
    principalLamports?: U64Wire;
    principal_lamports?: U64Wire;
    horizonDays?: number;
    horizon_days?: number;
    estimatedRewardsLamports?: U64Wire;
    estimated_rewards_lamports?: U64Wire;
    estimatedTotalLamports?: U64Wire;
    estimated_total_lamports?: U64Wire;
    rate?: YieldRateWire;
  };
}
