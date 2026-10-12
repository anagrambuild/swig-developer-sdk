import type {
  DexActionType,
  DexPositionFilter,
  DexPositionStatus,
  DexProtocol,
  DexTransactionFilter,
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
  return {
    path: `${swigPath(args.swigConfigAddress)}/dex/transactions:search`,
    body: searchBody(
      args,
      defaultNetwork,
      (args.filters ?? []).map(transactionFilterWire),
    ),
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
  return {
    path: `${swigPath(args.swigConfigAddress)}/dex/positions:search`,
    body: searchBody(
      args,
      defaultNetwork,
      (args.filters ?? []).map(positionFilterWire),
    ),
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

function transactionFilterWire(filter: DexTransactionFilter): DexFilterWire {
  switch (filter.kind) {
    case 'startSlot':
      return { startSlot: slotWire(filter.value, 'startSlot') };
    case 'endSlot':
      return { endSlot: slotWire(filter.value, 'endSlot') };
    case 'protocol':
      return { protocol: protocolWire(filter.value) };
    case 'actionType':
      return { actionType: actionTypeWire(filter.value) };
    case 'swigVaultAddress':
      return { swigVaultAddress: filter.value };
    default:
      throw new Error(
        'filter kind must be startSlot, endSlot, protocol, actionType or swigVaultAddress',
      );
  }
}

function positionFilterWire(filter: DexPositionFilter): DexFilterWire {
  switch (filter.kind) {
    case 'positionStatus':
      return { positionStatus: positionStatusWire(filter.value) };
    case 'protocol':
      return { protocol: protocolWire(filter.value) };
    case 'swigVaultAddress':
      return { swigVaultAddress: filter.value };
    default:
      throw new Error(
        'filter kind must be positionStatus, protocol or swigVaultAddress',
      );
  }
}

function requiredNetwork(network?: Network, defaultNetwork?: Network): Network {
  const resolved = network ?? defaultNetwork;
  if (!resolved) throw new Error('network is required');
  return resolved;
}

function swigPath(swigConfigAddress: string): string {
  return `/wallet/api/swig/${encodeURIComponent(swigConfigAddress)}`;
}

/** uint64 crosses the wire as a decimal string, so the bigint keeps its value. */
function slotWire(slot: bigint, kind: string): string {
  if (typeof slot !== 'bigint' || slot < 0n) {
    throw new Error(`${kind} must be a non-negative bigint`);
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
