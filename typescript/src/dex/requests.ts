import type {
  DexActionType,
  DexPositionStatus,
  DexProtocol,
  GetDexTransactionArgs,
  ListDexPositionsArgs,
  ListDexTransactionsArgs,
  Network,
} from '../types/index.js';
import { toProtoNetwork } from '../wallets/normalizers.js';

export function transactionsPath(
  args: ListDexTransactionsArgs,
  defaultNetwork?: Network,
): string {
  const query = pageQuery(args, defaultNetwork);
  if (args.startSlot !== undefined) {
    query.set('startSlot', slotParam(args.startSlot, 'startSlot'));
  }
  if (args.endSlot !== undefined) {
    query.set('endSlot', slotParam(args.endSlot, 'endSlot'));
  }
  if (args.protocol !== undefined) {
    query.set('protocol', protocolWire(args.protocol));
  }
  if (args.actionType !== undefined) {
    query.set('actionType', actionTypeWire(args.actionType));
  }
  if (args.swigVaultAddress !== undefined) {
    query.set('swigVaultAddress', args.swigVaultAddress);
  }
  return `${swigPath(args.swigConfigAddress)}/dex/transactions?${query.toString()}`;
}

export function transactionPath(
  args: GetDexTransactionArgs,
  defaultNetwork?: Network,
): string {
  const query = new URLSearchParams({
    network: toProtoNetwork(requiredNetwork(args.network, defaultNetwork)),
  });
  return `${swigPath(args.swigConfigAddress)}/dex/transactions/${encodeURIComponent(args.transactionSignature)}?${query.toString()}`;
}

export function positionsPath(
  args: ListDexPositionsArgs,
  defaultNetwork?: Network,
): string {
  const query = pageQuery(args, defaultNetwork);
  if (args.positionStatus !== undefined) {
    query.set('positionStatus', positionStatusWire(args.positionStatus));
  }
  if (args.protocol !== undefined) {
    query.set('protocol', protocolWire(args.protocol));
  }
  if (args.swigVaultAddress !== undefined) {
    query.set('swigVaultAddress', args.swigVaultAddress);
  }
  return `${swigPath(args.swigConfigAddress)}/dex/positions?${query.toString()}`;
}

function pageQuery(
  args: { network?: Network; pageSize?: number; pageToken?: string },
  defaultNetwork?: Network,
): URLSearchParams {
  const query = new URLSearchParams({
    network: toProtoNetwork(requiredNetwork(args.network, defaultNetwork)),
  });
  if (args.pageSize !== undefined) {
    if (!Number.isSafeInteger(args.pageSize) || args.pageSize < 0) {
      throw new Error('pageSize must be a non-negative integer');
    }
    query.set('pageSize', String(args.pageSize));
  }
  if (args.pageToken) query.set('pageToken', args.pageToken);
  return query;
}

function requiredNetwork(network?: Network, defaultNetwork?: Network): Network {
  const resolved = network ?? defaultNetwork;
  if (!resolved) throw new Error('network is required');
  return resolved;
}

function swigPath(swigConfigAddress: string): string {
  return `/wallet/swig/${encodeURIComponent(swigConfigAddress)}`;
}

/** uint64 crosses the wire as a decimal string, so a bigint keeps its value. */
function slotParam(slot: number | bigint, field: string): string {
  if (
    typeof slot === 'bigint'
      ? slot < 0n
      : !Number.isSafeInteger(slot) || slot < 0
  ) {
    throw new Error(`${field} must be a non-negative integer`);
  }
  return slot.toString();
}

function protocolWire(protocol: DexProtocol): string {
  switch (protocol) {
    case 'pump-swap':
      return 'DEX_PROTOCOL_PUMP_SWAP';
    case 'meteora-dlmm':
      return 'DEX_PROTOCOL_METEORA_DLMM';
    case 'raydium-amm-v4':
      return 'DEX_PROTOCOL_RAYDIUM_AMM_V4';
    case 'raydium-cpmm':
      return 'DEX_PROTOCOL_RAYDIUM_CPMM';
    case 'raydium-clmm':
      return 'DEX_PROTOCOL_RAYDIUM_CLMM';
    case 'orca-whirlpool':
      return 'DEX_PROTOCOL_ORCA_WHIRLPOOL';
    default:
      throw new Error('protocol must be a supported DEX protocol');
  }
}

function actionTypeWire(actionType: DexActionType): string {
  switch (actionType) {
    case 'swap':
      return 'DEX_ACTION_TYPE_SWAP';
    case 'position-open':
      return 'DEX_ACTION_TYPE_POSITION_OPEN';
    case 'liquidity-add':
      return 'DEX_ACTION_TYPE_LIQUIDITY_ADD';
    case 'liquidity-remove':
      return 'DEX_ACTION_TYPE_LIQUIDITY_REMOVE';
    case 'position-rebalance':
      return 'DEX_ACTION_TYPE_POSITION_REBALANCE';
    case 'fee-claim':
      return 'DEX_ACTION_TYPE_FEE_CLAIM';
    case 'reward-claim':
      return 'DEX_ACTION_TYPE_REWARD_CLAIM';
    case 'position-close':
      return 'DEX_ACTION_TYPE_POSITION_CLOSE';
    default:
      throw new Error('actionType must be a supported DEX action type');
  }
}

function positionStatusWire(positionStatus: DexPositionStatus): string {
  switch (positionStatus) {
    case 'open':
      return 'POSITION_STATUS_OPEN';
    case 'closed':
      return 'POSITION_STATUS_CLOSED';
    default:
      throw new Error('positionStatus must be "open" or "closed"');
  }
}
