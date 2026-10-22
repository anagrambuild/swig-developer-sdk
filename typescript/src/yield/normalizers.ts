import type {
  AddRoleAction,
  AddRoleActionWire,
  EstimateYieldResponseWire,
  GetYieldSummaryResponseWire,
  ListYieldOptionsResponseWire,
  ListYieldPositionsResponseWire,
  ListYieldRewardsResponseWire,
  PrepareYieldDepositResponseWire,
  PrepareYieldExitResponseWire,
  PrepareYieldRoleResponseWire,
  PrepareYieldWithdrawalResponseWire,
  PreparedTransactionWire,
  PreparedYieldDeposit,
  PreparedYieldExit,
  PreparedYieldWithdrawal,
  YieldOption,
  YieldPosition,
  YieldPositionState,
  YieldPositionWire,
  YieldPositionsPage,
  YieldProjection,
  YieldRate,
  YieldRateWire,
  YieldRewardsPage,
  YieldRoleResult,
  YieldSummary,
} from '../types/index.js';
import { normalizePreparedTransaction } from '../wallets/normalizers.js';

function lamports(value: string | number | undefined): string {
  return value === undefined || value === null ? '0' : String(value);
}

function required<T>(value: T | undefined | null, field: string): T {
  if (value === undefined || value === null || value === '') {
    throw new Error(`Yield response is missing ${field}`);
  }
  return value;
}

function transaction(wire: PreparedTransactionWire | undefined) {
  return normalizePreparedTransaction(required(wire, 'transaction'));
}

function u64(field: Record<string, unknown> | undefined, key: string): string {
  return lamports(field?.[key] as string | number | undefined);
}

function str(field: Record<string, unknown> | undefined, key: string): string {
  return String(field?.[key] ?? '');
}

/** Decode a transcoded `api_transaction.v1.AddRoleAction` oneof. */
export function normalizeAddRoleAction(wire: AddRoleActionWire): AddRoleAction {
  const [name, value] =
    Object.entries(wire).find(([, v]) => v !== undefined) ?? [];
  const body = value ?? {};
  const key = (camel: string) =>
    camel in body
      ? camel
      : camel.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
  const amount = (camel: string) => u64(body, key(camel));
  const text = (camel: string) => str(body, key(camel));
  switch (name) {
    case 'all':
      return { type: 'all' };
    case 'allButManageAuthority':
    case 'all_but_manage_authority':
      return { type: 'allButManageAuthority' };
    case 'manageAuthority':
    case 'manage_authority':
      return { type: 'manageAuthority' };
    case 'solLimit':
    case 'sol_limit':
      return { type: 'solLimit', amount: amount('amount') };
    case 'solRecurringLimit':
    case 'sol_recurring_limit':
      return {
        type: 'solRecurringLimit',
        recurringAmount: amount('recurringAmount'),
        window: amount('window'),
      };
    case 'solDestinationLimit':
    case 'sol_destination_limit':
      return {
        type: 'solDestinationLimit',
        amount: amount('amount'),
        destination: text('destination'),
      };
    case 'solRecurringDestinationLimit':
    case 'sol_recurring_destination_limit':
      return {
        type: 'solRecurringDestinationLimit',
        recurringAmount: amount('recurringAmount'),
        window: amount('window'),
        destination: text('destination'),
      };
    case 'program':
      return { type: 'program', programId: text('programId') };
    case 'programAll':
    case 'program_all':
      return { type: 'programAll' };
    case 'programCurated':
    case 'program_curated':
      return { type: 'programCurated' };
    case 'stakeLimit':
    case 'stake_limit':
      return { type: 'stakeLimit', amount: amount('amount') };
    case 'stakeRecurringLimit':
    case 'stake_recurring_limit':
      return {
        type: 'stakeRecurringLimit',
        recurringAmount: amount('recurringAmount'),
        window: amount('window'),
      };
    case 'stakeAll':
    case 'stake_all':
      return { type: 'stakeAll' };
    default:
      throw new Error(
        `Unsupported role action in yield response: ${String(name)}`,
      );
  }
}

export function normalizeYieldRole(
  wire: PrepareYieldRoleResponseWire,
): YieldRoleResult {
  if (wire.existing) {
    return {
      type: 'existing',
      roleId: required(
        wire.existing.roleId ?? wire.existing.role_id,
        'existing.roleId',
      ),
      actions: (wire.existing.actions ?? []).map(normalizeAddRoleAction),
    };
  }
  if (wire.create) {
    return {
      type: 'create',
      transaction: transaction(wire.create.transaction),
      yieldAuthority: required(
        wire.create.yieldAuthority ?? wire.create.yield_authority,
        'create.yieldAuthority',
      ),
      actions: (wire.create.actions ?? []).map(normalizeAddRoleAction),
    };
  }
  throw new Error('Yield role response has neither existing nor create');
}

export function normalizeYieldDeposit(
  wire: PrepareYieldDepositResponseWire,
): PreparedYieldDeposit {
  return {
    transaction: transaction(wire.transaction),
    positionAddress: required(
      wire.positionAddress ?? wire.position_address,
      'positionAddress',
    ),
    validatorVoteAddress: required(
      wire.validatorVoteAddress ?? wire.validator_vote_address,
      'validatorVoteAddress',
    ),
    delegatedLamports: lamports(
      wire.delegatedLamports ?? wire.delegated_lamports,
    ),
    rentLamports: lamports(wire.rentLamports ?? wire.rent_lamports),
  };
}

export function normalizeYieldExit(
  wire: PrepareYieldExitResponseWire,
): PreparedYieldExit {
  return {
    transaction: transaction(wire.transaction),
    exitingPositionAddress: required(
      wire.exitingPositionAddress ?? wire.exiting_position_address,
      'exitingPositionAddress',
    ),
    deactivatingLamports: lamports(
      wire.deactivatingLamports ?? wire.deactivating_lamports,
    ),
    additionalRentLamports: lamports(
      wire.additionalRentLamports ?? wire.additional_rent_lamports,
    ),
  };
}

export function normalizeYieldWithdrawal(
  wire: PrepareYieldWithdrawalResponseWire,
): PreparedYieldWithdrawal {
  return {
    transaction: transaction(wire.transaction),
    withdrawalLamports: lamports(
      wire.withdrawalLamports ?? wire.withdrawal_lamports,
    ),
    destinationWalletAddress: required(
      wire.destinationWalletAddress ?? wire.destination_wallet_address,
      'destinationWalletAddress',
    ),
  };
}

export function normalizeYieldRate(
  wire: YieldRateWire | undefined,
): YieldRate | undefined {
  if (!wire) return undefined;
  const lastUpdatedAt = wire.lastUpdatedAt ?? wire.last_updated_at;
  return {
    validatorVoteAddress:
      wire.validatorVoteAddress ?? wire.validator_vote_address ?? '',
    estimatedNetAprBps:
      wire.estimatedNetAprBps ?? wire.estimated_net_apr_bps ?? 0,
    commissionBps: wire.commissionBps ?? wire.commission_bps ?? 0,
    ...(lastUpdatedAt ? { lastUpdatedAt } : {}),
  };
}

export function normalizeYieldOptions(
  wire: ListYieldOptionsResponseWire,
): YieldOption[] {
  return (wire.options ?? []).map((option) => {
    const rate = normalizeYieldRate(option.rate);
    return {
      voteAddress: option.voteAddress ?? option.vote_address ?? '',
      name: option.name ?? '',
      acceptsDeposits:
        option.acceptsDeposits ?? option.accepts_deposits ?? false,
      ...(rate ? { rate } : {}),
    };
  });
}

const POSITION_STATES: Record<string, YieldPositionState> = {
  YIELD_POSITION_STATE_ACTIVATING: 'activating',
  YIELD_POSITION_STATE_ACTIVE: 'active',
  YIELD_POSITION_STATE_DEACTIVATING: 'deactivating',
  YIELD_POSITION_STATE_INACTIVE: 'inactive',
  '1': 'activating',
  '2': 'active',
  '3': 'deactivating',
  '4': 'inactive',
};

export function normalizeYieldPosition(wire: YieldPositionWire): YieldPosition {
  const validatorVoteAddress =
    wire.validatorVoteAddress ?? wire.validator_vote_address;
  return {
    swigConfigAddress: wire.swigConfigAddress ?? wire.swig_config_address ?? '',
    walletAddress: wire.walletAddress ?? wire.wallet_address ?? '',
    positionAddress: wire.positionAddress ?? wire.position_address ?? '',
    ...(validatorVoteAddress ? { validatorVoteAddress } : {}),
    state: POSITION_STATES[String(wire.state)] ?? 'unspecified',
    balanceLamports: lamports(wire.balanceLamports ?? wire.balance_lamports),
    rentLamports: lamports(wire.rentLamports ?? wire.rent_lamports),
    positionLamports: lamports(wire.positionLamports ?? wire.position_lamports),
    withdrawableLamports: lamports(
      wire.withdrawableLamports ?? wire.withdrawable_lamports,
    ),
    earningLamports: lamports(wire.earningLamports ?? wire.earning_lamports),
  };
}

export function normalizeYieldPositions(
  wire: ListYieldPositionsResponseWire,
): YieldPositionsPage {
  const nextPageToken = wire.nextPageToken ?? wire.next_page_token;
  const lastUpdatedAt = wire.lastUpdatedAt ?? wire.last_updated_at;
  return {
    positions: (wire.positions ?? []).map(normalizeYieldPosition),
    ...(nextPageToken ? { nextPageToken } : {}),
    ...(lastUpdatedAt ? { lastUpdatedAt } : {}),
  };
}

export function normalizeYieldSummary(
  wire: GetYieldSummaryResponseWire,
): YieldSummary {
  return {
    totalPositionLamports: lamports(
      wire.totalPositionLamports ?? wire.total_position_lamports,
    ),
    withdrawableLamports: lamports(
      wire.withdrawableLamports ?? wire.withdrawable_lamports,
    ),
    recordedRewardsLamports: lamports(
      wire.recordedRewardsLamports ?? wire.recorded_rewards_lamports,
    ),
  };
}

export function normalizeYieldRewards(
  wire: ListYieldRewardsResponseWire,
): YieldRewardsPage {
  const nextPageToken = wire.nextPageToken ?? wire.next_page_token;
  return {
    rewards: (wire.rewards ?? []).map((reward) => {
      const earnedAt = reward.earnedAt ?? reward.earned_at;
      return {
        swigConfigAddress:
          reward.swigConfigAddress ?? reward.swig_config_address ?? '',
        positionAddress:
          reward.positionAddress ?? reward.position_address ?? '',
        amountLamports: lamports(
          reward.amountLamports ?? reward.amount_lamports,
        ),
        ...(earnedAt ? { earnedAt } : {}),
      };
    }),
    ...(nextPageToken ? { nextPageToken } : {}),
  };
}

export function normalizeYieldProjection(
  wire: EstimateYieldResponseWire,
): YieldProjection {
  const projection = required(wire.projection, 'projection');
  const rate = normalizeYieldRate(projection.rate);
  return {
    principalLamports: lamports(
      projection.principalLamports ?? projection.principal_lamports,
    ),
    horizonDays: projection.horizonDays ?? projection.horizon_days ?? 0,
    estimatedRewardsLamports: lamports(
      projection.estimatedRewardsLamports ??
        projection.estimated_rewards_lamports,
    ),
    estimatedTotalLamports: lamports(
      projection.estimatedTotalLamports ?? projection.estimated_total_lamports,
    ),
    ...(rate ? { rate } : {}),
  };
}
