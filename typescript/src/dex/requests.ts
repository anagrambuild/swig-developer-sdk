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

/** A filter on the wire: one key, named for its kind. */
type DexFilterWire = Record<string, string>;

interface DexSearchBodyWire {
  network: string;
  pageSize?: number;
  pageToken?: string;
  filters?: DexFilterWire[];
}

interface DexSearchRequest {
  path: string;
  body: DexSearchBodyWire;
}

export function transactionsSearch(
  args: ListDexTransactionsArgs,
  defaultNetwork?: Network,
): DexSearchRequest {
  const filters: DexFilterWire[] = [];
  if (args.startSlot !== undefined) {
    filters.push({ startSlot: slotWire(args.startSlot, 'startSlot') });
  }
  if (args.endSlot !== undefined) {
    filters.push({ endSlot: slotWire(args.endSlot, 'endSlot') });
  }
  if (args.protocol !== undefined) {
    filters.push({ protocol: protocolWire(args.protocol) });
  }
  if (args.actionType !== undefined) {
    filters.push({ actionType: actionTypeWire(args.actionType) });
  }
  if (args.swigVaultAddress !== undefined) {
    filters.push({ swigVaultAddress: args.swigVaultAddress });
  }
  return {
    path: `${swigPath(args.swigConfigAddress)}/dex/transactions:search`,
    body: searchBody(args, defaultNetwork, filters),
  };
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

export function positionsSearch(
  args: ListDexPositionsArgs,
  defaultNetwork?: Network,
): DexSearchRequest {
  const filters: DexFilterWire[] = [];
  if (args.positionStatus !== undefined) {
    filters.push({ positionStatus: positionStatusWire(args.positionStatus) });
  }
  if (args.protocol !== undefined) {
    filters.push({ protocol: protocolWire(args.protocol) });
  }
  if (args.swigVaultAddress !== undefined) {
    filters.push({ swigVaultAddress: args.swigVaultAddress });
  }
  return {
    path: `${swigPath(args.swigConfigAddress)}/dex/positions:search`,
    body: searchBody(args, defaultNetwork, filters),
  };
}

function searchBody(
  args: { network?: Network; pageSize?: number; pageToken?: string },
  defaultNetwork: Network | undefined,
  filters: DexFilterWire[],
): DexSearchBodyWire {
  if (
    args.pageSize !== undefined &&
    (!Number.isSafeInteger(args.pageSize) || args.pageSize < 0)
  ) {
    throw new Error('pageSize must be a non-negative integer');
  }
  return {
    network: toProtoNetwork(requiredNetwork(args.network, defaultNetwork)),
    ...(args.pageSize !== undefined ? { pageSize: args.pageSize } : {}),
    ...(args.pageToken ? { pageToken: args.pageToken } : {}),
    ...(filters.length > 0 ? { filters } : {}),
  };
}

function requiredNetwork(network?: Network, defaultNetwork?: Network): Network {
  const resolved = network ?? defaultNetwork;
  if (!resolved) throw new Error('network is required');
  return resolved;
}

function swigPath(swigConfigAddress: string): string {
  return `/wallet/api/swig/${encodeURIComponent(swigConfigAddress)}`;
}

/** uint64 crosses the wire as a decimal string. */
function slotWire(slot: number, field: string): string {
  if (!Number.isSafeInteger(slot) || slot < 0) {
    throw new Error(`${field} must be a non-negative integer`);
  }
  return String(slot);
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
