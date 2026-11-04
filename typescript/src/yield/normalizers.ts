import { SwigDeveloperSdkError } from '../core/errors.js';
import type {
  AddRoleAction,
  PreparedYieldDeposit,
  PreparedYieldExit,
  PreparedYieldWithdrawal,
  WalletAuthority,
  YieldOption,
  YieldPosition,
  YieldPositionState,
  YieldPositionsPage,
  YieldProjection,
  YieldRate,
  YieldRewardsPage,
  YieldRoleResult,
  YieldSummary,
} from '../types/index.js';
import { normalizePreparedTransaction } from '../wallets/normalizers.js';

function invalid(field: string): never {
  throw new SwigDeveloperSdkError(
    `Invalid yield response: ${field}`,
    'INVALID_RESPONSE',
    502,
  );
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (!isRecord(value)) invalid(`${field} must be an object`);
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function collection(value: unknown, field: string): unknown[] {
  if (value === undefined || value === null) return [];
  if (!Array.isArray(value)) invalid(`${field} must be an array`);
  return value;
}

function required(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0)
    invalid(`${field} must be a nonempty string`);
  return value;
}

function optionalString(value: unknown, field: string): string | undefined {
  if (value === undefined || value === null || value === '') return undefined;
  return required(value, field);
}

function lamports(value: unknown): string {
  if (value === undefined || value === null) return '0';
  if (typeof value === 'number' && Number.isSafeInteger(value) && value >= 0)
    return String(value);
  if (
    typeof value === 'string' &&
    /^[0-9]+$/.test(value) &&
    BigInt(value) <= 0xffffffffffffffffn
  )
    return value;
  return invalid('lamports must be an unsigned 64-bit integer');
}

function uint32(value: unknown, field: string): number {
  const amount = Number(lamports(value));
  if (!Number.isInteger(amount) || amount > 0xffffffff)
    invalid(`${field} must fit in uint32`);
  return amount;
}

function boolean(value: unknown, field: string): boolean {
  if (value === undefined || value === null) return false;
  if (typeof value !== 'boolean') invalid(`${field} must be a boolean`);
  return value;
}

function transaction(value: unknown) {
  if (value == null) invalid('missing transaction');
  const wire = record(value, 'transaction');
  required(
    wire.transaction ?? wire.unsignedTransaction ?? wire.unsigned_transaction,
    'transaction',
  );
  return normalizePreparedTransaction(wire);
}

function authority(value: unknown): WalletAuthority {
  const wire = record(value, 'yieldAuthority');
  const variants = Object.entries(wire).filter(([, value]) => value != null);
  if (variants.length !== 1)
    invalid('yieldAuthority must contain one authority');
  const [name, valueBody] = variants[0];
  const body = record(valueBody, 'yieldAuthority body');
  switch (name) {
    case 'ed25519':
      return {
        ed25519: {
          publicKey: required(body.publicKey ?? body.public_key, 'publicKey'),
        },
      };
    case 'secp256k1':
      return {
        secp256k1: {
          publicKey: required(body.publicKey ?? body.public_key, 'publicKey'),
        },
      };
    case 'secp256r1':
      return {
        secp256r1: {
          publicKey: required(body.publicKey ?? body.public_key, 'publicKey'),
        },
      };
    case 'participantSet':
    case 'participant_set': {
      const roleId = body.roleId ?? body.role_id;
      return {
        participantSet: {
          address: required(
            body.participantSetAddress ?? body.participant_set_address,
            'participantSetAddress',
          ),
          ...(roleId == null ? {} : { roleId: uint32(roleId, 'roleId') }),
        },
      };
    }
    default:
      return invalid('unsupported yieldAuthority');
  }
}

/** Decode a transcoded `api_transaction.v1.AddRoleAction` oneof. */
export function normalizeAddRoleAction(value: unknown): AddRoleAction {
  const wire = record(value, 'role action');
  const variants = Object.entries(wire).filter(([, value]) => value != null);
  if (variants.length !== 1) invalid('role action must contain one variant');
  const [name, valueBody] = variants[0];
  const body = record(valueBody, 'role action body');
  const key = (camel: string) =>
    camel in body
      ? camel
      : camel.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
  const amount = (camel: string) => lamports(body[key(camel)]);
  const text = (camel: string) => required(body[key(camel)], camel);
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
      return invalid('unsupported role action');
  }
}

export function normalizeYieldRole(value: unknown): YieldRoleResult {
  const wire = record(value, 'normalizeYieldRole');
  if (wire.existing != null && wire.create != null)
    invalid('role response has both existing and create');
  if (wire.existing != null) {
    const existing = record(wire.existing, 'existing');
    if ((existing.roleId ?? existing.role_id) == null)
      invalid('missing existing.roleId');
    return {
      type: 'existing',
      roleId: uint32(existing.roleId ?? existing.role_id, 'existing.roleId'),
      actions: collection(existing.actions, 'actions').map(
        normalizeAddRoleAction,
      ),
    };
  }
  if (wire.create != null) {
    const create = record(wire.create, 'create');
    return {
      type: 'create',
      transaction: transaction(create.transaction),
      yieldAuthority: authority(
        create.yieldAuthority ?? create.yield_authority,
      ),
      actions: collection(create.actions, 'actions').map(
        normalizeAddRoleAction,
      ),
    };
  }
  return invalid('role response has neither existing nor create');
}

export function normalizeYieldDeposit(value: unknown): PreparedYieldDeposit {
  const wire = record(value, 'normalizeYieldDeposit');
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

export function normalizeYieldExit(value: unknown): PreparedYieldExit {
  const wire = record(value, 'normalizeYieldExit');
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
  value: unknown,
): PreparedYieldWithdrawal {
  const wire = record(value, 'normalizeYieldWithdrawal');
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

export function normalizeYieldRate(value: unknown): YieldRate | undefined {
  if (value === undefined || value === null) return undefined;
  const wire = record(value, 'rate');
  const lastUpdatedAt = optionalString(
    wire.lastUpdatedAt ?? wire.last_updated_at,
    'lastUpdatedAt',
  );
  return {
    validatorVoteAddress: required(
      wire.validatorVoteAddress ?? wire.validator_vote_address,
      'validatorVoteAddress',
    ),
    estimatedNetAprBps: uint32(
      wire.estimatedNetAprBps ?? wire.estimated_net_apr_bps,
      'estimatedNetAprBps',
    ),
    commissionBps: uint32(
      wire.commissionBps ?? wire.commission_bps,
      'commissionBps',
    ),
    ...(lastUpdatedAt ? { lastUpdatedAt } : {}),
  };
}

export function normalizeYieldOptions(value: unknown): YieldOption[] {
  const wire = record(value, 'normalizeYieldOptions');
  return collection(wire.options, 'options').map((value) => {
    const option = record(value, 'option');
    const rate = normalizeYieldRate(option.rate);
    return {
      voteAddress: required(
        option.voteAddress ?? option.vote_address,
        'voteAddress',
      ),
      name: optionalString(option.name, 'name') ?? '',
      acceptsDeposits: boolean(
        option.acceptsDeposits ?? option.accepts_deposits,
        'acceptsDeposits',
      ),
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

export function normalizeYieldPosition(value: unknown): YieldPosition {
  const wire = record(value, 'normalizeYieldPosition');
  const validatorVoteAddress = optionalString(
    wire.validatorVoteAddress ?? wire.validator_vote_address,
    'validatorVoteAddress',
  );
  return {
    swigConfigAddress: required(
      wire.swigConfigAddress ?? wire.swig_config_address,
      'swigConfigAddress',
    ),
    walletAddress: required(
      wire.walletAddress ?? wire.wallet_address,
      'walletAddress',
    ),
    positionAddress: required(
      wire.positionAddress ?? wire.position_address,
      'positionAddress',
    ),
    ...(validatorVoteAddress ? { validatorVoteAddress } : {}),
    state: Object.hasOwn(POSITION_STATES, String(wire.state))
      ? POSITION_STATES[String(wire.state)]
      : 'unspecified',
    balanceLamports: lamports(wire.balanceLamports ?? wire.balance_lamports),
    rentLamports: lamports(wire.rentLamports ?? wire.rent_lamports),
    positionLamports: lamports(wire.positionLamports ?? wire.position_lamports),
    withdrawableLamports: lamports(
      wire.withdrawableLamports ?? wire.withdrawable_lamports,
    ),
    earningLamports: lamports(wire.earningLamports ?? wire.earning_lamports),
  };
}

export function normalizeYieldPositions(value: unknown): YieldPositionsPage {
  const wire = record(value, 'normalizeYieldPositions');
  const nextPageToken = optionalString(
    wire.nextPageToken ?? wire.next_page_token,
    'nextPageToken',
  );
  const lastUpdatedAt = optionalString(
    wire.lastUpdatedAt ?? wire.last_updated_at,
    'lastUpdatedAt',
  );
  return {
    positions: collection(wire.positions, 'positions').map(
      normalizeYieldPosition,
    ),
    ...(nextPageToken ? { nextPageToken } : {}),
    ...(lastUpdatedAt ? { lastUpdatedAt } : {}),
  };
}

export function normalizeYieldSummary(value: unknown): YieldSummary {
  const wire = record(value, 'normalizeYieldSummary');
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

export function normalizeYieldRewards(value: unknown): YieldRewardsPage {
  const wire = record(value, 'normalizeYieldRewards');
  const nextPageToken = optionalString(
    wire.nextPageToken ?? wire.next_page_token,
    'nextPageToken',
  );
  return {
    rewards: collection(wire.rewards, 'rewards').map((value) => {
      const reward = record(value, 'reward');
      const earnedAt = optionalString(
        reward.earnedAt ?? reward.earned_at,
        'earnedAt',
      );
      return {
        swigConfigAddress: required(
          reward.swigConfigAddress ?? reward.swig_config_address,
          'swigConfigAddress',
        ),
        positionAddress: required(
          reward.positionAddress ?? reward.position_address,
          'positionAddress',
        ),
        amountLamports: lamports(
          reward.amountLamports ?? reward.amount_lamports,
        ),
        ...(earnedAt ? { earnedAt } : {}),
      };
    }),
    ...(nextPageToken ? { nextPageToken } : {}),
  };
}

export function normalizeYieldProjection(value: unknown): YieldProjection {
  const wire = record(value, 'normalizeYieldProjection');
  const projection = record(wire.projection, 'projection');
  const rate = normalizeYieldRate(projection.rate);
  return {
    principalLamports: lamports(
      projection.principalLamports ?? projection.principal_lamports,
    ),
    horizonDays: uint32(
      projection.horizonDays ?? projection.horizon_days,
      'horizonDays',
    ),
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
