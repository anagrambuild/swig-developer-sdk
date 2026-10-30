import bs58 from 'bs58';

import type {
  AddAuthorizationLockArgs,
  AddRecoveryAuthorityArgs,
  AddRoleAction,
  AddRoleArgs,
  Amount,
  BuildTransactionArgs,
  CancelRecoveryArgs,
  ConfigureRecoveryArgs,
  CreateWalletArgs,
  ExecuteRecoveryArgs,
  Network,
  NetworkWire,
  PrepareArgs,
  PrepareOperation,
  RemoveAuthorizationLockArgs,
  SetRentClaimerArgs,
  StartRecoveryArgs,
  SwapArgs,
  TransferArgs,
  TransferSolArgs,
  TransferTokenArgs,
  WalletAuthority,
  WalletAuthorityKind,
} from '../types/index.js';
import type { WalletHandle } from './handle.js';
import {
  normalizeAmount,
  normalizeInstruction,
  toProtoNetwork,
} from './normalizers.js';

export function createWalletRequest(
  args: CreateWalletArgs,
  defaultNetwork?: Network,
) {
  if (args.initialUser) {
    assertNonParticipantAuthority(args.initialUser, 'initialUser');
  }
  return {
    network: toProtoNetwork(resolveNetwork(args.network, defaultNetwork)),
    feePayer: args.feePayer,
    ...(args.policyId ? { policyId: args.policyId } : {}),
    initialUser: args.initialUser
      ? walletAuthorityRequest(args.initialUser)
      : undefined,
  };
}

export function transferSolRequest(
  wallet: WalletHandle,
  args: TransferSolArgs,
  defaultNetwork?: Network,
) {
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
    destination: args.destination,
    lamports: normalizeAmount(args.amount),
  };
}

export function transferTokenRequest(
  wallet: WalletHandle,
  args: TransferTokenArgs,
  defaultNetwork?: Network,
) {
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
    mint: args.mint,
    destinationOwner: args.destinationOwner,
    amount: normalizeAmount(args.amount),
  };
}

export function prepareRequest(
  wallet: WalletHandle,
  args: PrepareArgs,
  defaultNetwork?: Network,
) {
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
    operations: args.operations.map(prepareOperationRequest),
  };
}

export function swapRequest(
  wallet: WalletHandle,
  args: SwapArgs,
  defaultNetwork?: Network,
) {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  assertNonParticipantAuthority(requesterAuthority, 'requesterAuthority');
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
    inputMint: args.inputMint,
    outputMint: args.outputMint,
    amount: normalizeAmount(args.amount),
    slippageBps: args.slippageBps,
    destinationAccount: args.destinationAccount,
    wrapAndUnwrapSol: args.wrapAndUnwrapSol,
    tipAmountLamports:
      args.tipAmountLamports === undefined
        ? undefined
        : normalizeAmount(args.tipAmountLamports),
    computeUnitPricePercentile: args.computeUnitPricePercentile,
    maxAccounts: args.maxAccounts,
    mode: args.mode,
    blockhashSlotsToExpiry: args.blockhashSlotsToExpiry,
  };
}

export function startRecoveryRequest(
  wallet: WalletHandle,
  args: StartRecoveryArgs,
  defaultNetwork?: Network,
) {
  validateGuardianSource(args);
  if ('guardianRequesterAuthority' in args) {
    assertNonParticipantAuthority(
      args.guardianRequesterAuthority!,
      'guardianRequesterAuthority',
    );
  }
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    ...('guardianPubkey' in args
      ? { guardianPubkey: args.guardianPubkey }
      : {}),
    newAuthority: args.newAuthority,
    newAuthorityKind: toProtoWalletAuthorityKind(args.newAuthorityKind),
    ...('guardianSwigAddress' in args
      ? {
          guardianSwigAddress: args.guardianSwigAddress,
          guardianRequesterAuthority: walletAuthorityRequest(
            args.guardianRequesterAuthority!,
          ),
        }
      : {}),
  };
}

export function addRecoveryAuthorityRequest(
  wallet: WalletHandle,
  args: AddRecoveryAuthorityArgs,
  defaultNetwork?: Network,
) {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  assertNonParticipantAuthority(requesterAuthority, 'requesterAuthority');
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
  };
}

export function configureRecoveryRequest(
  wallet: WalletHandle,
  args: ConfigureRecoveryArgs,
  defaultNetwork?: Network,
) {
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    guardianPubkey: args.guardianPubkey,
    delaySeconds: args.delaySeconds,
    targetRoleId: args.targetRoleId,
  };
}

export function cancelRecoveryRequest(
  wallet: WalletHandle,
  args: CancelRecoveryArgs,
  defaultNetwork?: Network,
) {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  assertNonParticipantAuthority(requesterAuthority, 'requesterAuthority');
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
  };
}

export function executeRecoveryRequest(
  wallet: WalletHandle,
  args: ExecuteRecoveryArgs,
  defaultNetwork?: Network,
) {
  if (
    (args.guardianSwigAddress === undefined) !==
    (args.guardianRequesterAuthority === undefined)
  ) {
    throw new Error(
      'guardianSwigAddress and guardianRequesterAuthority must be provided together',
    );
  }
  if (args.guardianRequesterAuthority) {
    assertNonParticipantAuthority(
      args.guardianRequesterAuthority,
      'guardianRequesterAuthority',
    );
  }
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    newAuthority: args.newAuthority,
    newAuthorityKind: toProtoWalletAuthorityKind(args.newAuthorityKind),
    ...(args.guardianSwigAddress
      ? {
          guardianSwigAddress: args.guardianSwigAddress,
          guardianRequesterAuthority: walletAuthorityRequest(
            args.guardianRequesterAuthority,
          ),
        }
      : {}),
  };
}

export function buildTransactionRequest(
  wallet: WalletHandle,
  args: BuildTransactionArgs,
  defaultNetwork?: Network,
) {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  if (
    'participantSet' in requesterAuthority &&
    args.addressLookupTableAccounts?.length
  ) {
    throw new Error(
      'ParticipantSet requesterAuthority does not support addressLookupTableAccounts',
    );
  }
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
    instructions: args.instructions.map(normalizeInstruction),
    addressLookupTableAccounts: args.addressLookupTableAccounts,
  };
}

export function isTokenTransfer(args: TransferArgs): args is TransferTokenArgs {
  return (
    'mint' in args && typeof args.mint === 'string' && args.mint.length > 0
  );
}

export function addRoleRequest(
  wallet: WalletHandle,
  args: AddRoleArgs,
  defaultNetwork?: Network,
) {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  if (
    !('ed25519' in requesterAuthority) &&
    !('secp256r1' in requesterAuthority)
  ) {
    throw new Error(
      'Add role requesterAuthority must use ed25519 or secp256r1',
    );
  }
  if ('programExecProof' in args.authority) {
    throw new Error('Add role authority does not support programExecProof');
  }
  if (
    'participantSet' in args.authority &&
    'roleId' in args.authority.participantSet
  ) {
    throw new Error('Add role ParticipantSet authority must omit roleId');
  }
  if (args.actions.length === 0) {
    throw new Error('Add role actions must not be empty');
  }
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
    authority: walletAuthorityRequest(args.authority),
    actions: args.actions.map(addRoleActionRequest),
  };
}

function addRoleActionRequest(action: AddRoleAction) {
  switch (action.type) {
    case 'all':
      return { all: {} };
    case 'allButManageAuthority':
      return { allButManageAuthority: {} };
    case 'manageAuthority':
      return { manageAuthority: {} };
    case 'solLimit':
      return { solLimit: { amount: normalizeAmount(action.amount) } };
    case 'solRecurringLimit':
      return {
        solRecurringLimit: {
          recurringAmount: normalizeAmount(action.recurringAmount),
          window: normalizeAmount(action.window),
        },
      };
    case 'solDestinationLimit':
      return {
        solDestinationLimit: {
          amount: normalizeAmount(action.amount),
          destination: action.destination,
        },
      };
    case 'solRecurringDestinationLimit':
      return {
        solRecurringDestinationLimit: {
          recurringAmount: normalizeAmount(action.recurringAmount),
          window: normalizeAmount(action.window),
          destination: action.destination,
        },
      };
    case 'tokenLimit':
      return {
        tokenLimit: {
          mint: action.mint,
          amount: normalizeAmount(action.amount),
        },
      };
    case 'tokenRecurringLimit':
      return {
        tokenRecurringLimit: {
          mint: action.mint,
          recurringAmount: normalizeAmount(action.recurringAmount),
          window: normalizeAmount(action.window),
        },
      };
    case 'tokenDestinationLimit':
      return {
        tokenDestinationLimit: {
          mint: action.mint,
          amount: normalizeAmount(action.amount),
          destination: action.destination,
        },
      };
    case 'tokenRecurringDestinationLimit':
      return {
        tokenRecurringDestinationLimit: {
          mint: action.mint,
          recurringAmount: normalizeAmount(action.recurringAmount),
          window: normalizeAmount(action.window),
          destination: action.destination,
        },
      };
    case 'program':
      return { program: { programId: action.programId } };
    case 'programAll':
      return { programAll: {} };
    case 'programCurated':
      return { programCurated: {} };
    case 'stakeLimit':
      return { stakeLimit: { amount: normalizeAmount(action.amount) } };
    case 'stakeRecurringLimit':
      return {
        stakeRecurringLimit: {
          recurringAmount: normalizeAmount(action.recurringAmount),
          window: normalizeAmount(action.window),
        },
      };
    case 'stakeAll':
      return { stakeAll: {} };
    case 'subAccount':
      return { subAccount: {} };
  }
}

export function walletAuthorityRequest(authority: WalletAuthority) {
  if ('participantSet' in authority) {
    return {
      participantSet: {
        participantSetAddress: authority.participantSet.address,
        roleId: authority.participantSet.roleId,
      },
    };
  }
  return authority;
}

function assertNonParticipantAuthority(
  authority: WalletAuthority,
  field: string,
): void {
  if ('participantSet' in authority) {
    throw new Error(`${field} does not support ParticipantSet authority`);
  }
}

function prepareOperationRequest(operation: PrepareOperation) {
  switch (operation.type) {
    case 'transferSol':
      return {
        transferSol: {
          destination: operation.destination,
          lamports: normalizeAmount(operation.amount),
        },
      };
    case 'transferToken':
      return {
        transferToken: {
          mint: operation.mint,
          destinationOwner: operation.destinationOwner,
          amount: normalizeAmount(operation.amount),
        },
      };
  }
}

function resolveNetwork(...networks: Array<Network | undefined>): Network {
  const network = networks.find((candidate) => candidate !== undefined);
  if (!network) {
    throw new Error('network is required');
  }
  return network;
}

function toProtoWalletAuthorityKind(kind: WalletAuthorityKind): string {
  switch (kind) {
    case 'ed25519':
      return 'WALLET_AUTHORITY_KIND_ED25519';
    case 'secp256k1':
      return 'WALLET_AUTHORITY_KIND_SECP256K1';
    case 'secp256r1':
      return 'WALLET_AUTHORITY_KIND_SECP256R1';
  }
}

function validateGuardianSource(args: StartRecoveryArgs): void {
  const hasDirectGuardian = args.guardianPubkey !== undefined;
  const hasSwigAddress = args.guardianSwigAddress !== undefined;
  const hasSwigAuthority = args.guardianRequesterAuthority !== undefined;
  if (hasSwigAddress !== hasSwigAuthority) {
    throw new Error(
      'guardianSwigAddress and guardianRequesterAuthority must be provided together',
    );
  }
  if (hasDirectGuardian === hasSwigAddress) {
    throw new Error(
      'provide exactly one guardian source: guardianPubkey or guardianSwigAddress',
    );
  }
}

function resolveRequesterAuthority(
  wallet: WalletHandle,
  args:
    | TransferArgs
    | SwapArgs
    | PrepareArgs
    | BuildTransactionArgs
    | CancelRecoveryArgs
    | AddRecoveryAuthorityArgs
    | AddRoleArgs
    | AddAuthorizationLockArgs
    | RemoveAuthorizationLockArgs,
): NonNullable<
  | TransferArgs['requesterAuthority']
  | SwapArgs['requesterAuthority']
  | PrepareArgs['requesterAuthority']
  | BuildTransactionArgs['requesterAuthority']
  | CancelRecoveryArgs['requesterAuthority']
  | AddRecoveryAuthorityArgs['requesterAuthority']
  | AddRoleArgs['requesterAuthority']
  | AddAuthorizationLockArgs['requesterAuthority']
  | RemoveAuthorizationLockArgs['requesterAuthority']
> {
  const requesterAuthority =
    args.requesterAuthority ?? wallet.requesterAuthority;
  if (!requesterAuthority) {
    throw new Error('requesterAuthority is required');
  }
  return requesterAuthority;
}

interface SetRentClaimerRequest {
  network: NetworkWire;
  swigAddress: string;
  feePayer: string;
  requesterAuthority: Extract<WalletAuthority, { ed25519: unknown }>;
  rentClaimer: string;
}

export function setRentClaimerRequest(
  wallet: WalletHandle,
  args: SetRentClaimerArgs,
  defaultNetwork?: Network,
): SetRentClaimerRequest {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  if (!('ed25519' in requesterAuthority)) {
    throw new Error(
      'rent claimer requires a direct Ed25519 requesterAuthority',
    );
  }
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    swigAddress: wallet.swigConfigAddress,
    feePayer: args.feePayer,
    requesterAuthority,
    rentClaimer: args.rentClaimer,
  };
}

const MAX_UINT32 = 4_294_967_295;
const MAX_UINT64 = 18_446_744_073_709_551_615n;

export function addAuthorizationLockRequest(
  wallet: WalletHandle,
  args: AddAuthorizationLockArgs,
  defaultNetwork?: Network,
) {
  const hasExpirySlot = args.expirySlot !== undefined;
  const hasDurationSlots = args.durationSlots !== undefined;
  if (hasExpirySlot === hasDurationSlots) {
    throw new Error(
      'provide exactly one authorization lock expiry: expirySlot or durationSlots',
    );
  }
  if (args.mint !== undefined && !isPublicKey(args.mint)) {
    throw new Error('mint must be a base58-encoded 32-byte public key');
  }
  const amount = normalizeUint64(args.amount, 'amount');
  if (BigInt(amount) === 0n) {
    throw new Error('amount must be greater than zero');
  }
  const requesterAuthority = resolveAuthorizationLockRequester(wallet, args);
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
    ...(args.mint === undefined ? {} : { mint: args.mint }),
    amount,
    ...(args.expirySlot === undefined
      ? {}
      : { expirySlot: normalizeUint64(args.expirySlot, 'expirySlot') }),
    ...(args.durationSlots === undefined
      ? {}
      : {
          durationSlots: normalizeUint64(args.durationSlots, 'durationSlots'),
        }),
  };
}

export function removeAuthorizationLockRequest(
  wallet: WalletHandle,
  args: RemoveAuthorizationLockArgs,
  defaultNetwork?: Network,
) {
  if (
    !Number.isInteger(args.lockId) ||
    args.lockId < 0 ||
    args.lockId > MAX_UINT32
  ) {
    throw new Error('lockId must be an unsigned 32-bit integer');
  }
  const requesterAuthority = resolveAuthorizationLockRequester(wallet, args);
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(requesterAuthority),
    lockId: args.lockId,
  };
}

function resolveAuthorizationLockRequester(
  wallet: WalletHandle,
  args: AddAuthorizationLockArgs | RemoveAuthorizationLockArgs,
) {
  const requesterAuthority = resolveRequesterAuthority(wallet, args);
  if (
    !('ed25519' in requesterAuthority) &&
    !('secp256k1' in requesterAuthority) &&
    !('secp256r1' in requesterAuthority)
  ) {
    throw new Error(
      'Authorization lock requesterAuthority must use ed25519, secp256k1, or secp256r1',
    );
  }
  return requesterAuthority;
}

/** uint64 crosses the wire as a decimal string so values above 2^53 survive. */
function normalizeUint64(value: Amount, field: string): string {
  if (typeof value === 'number' && !Number.isSafeInteger(value)) {
    throw new Error(
      `${field} numbers must be safe integers; use a bigint or decimal string for larger values`,
    );
  }
  const normalized = normalizeAmount(value);
  if (!/^\d+$/.test(normalized)) {
    throw new Error(`${field} must be a non-negative decimal integer`);
  }
  if (BigInt(normalized) > MAX_UINT64) {
    throw new Error(`${field} must fit in an unsigned 64-bit integer`);
  }
  return normalized;
}

function isPublicKey(value: string): boolean {
  try {
    return bs58.decode(value).length === 32;
  } catch {
    return false;
  }
}
