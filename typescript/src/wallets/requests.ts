import type {
  AddRecoveryAuthorityArgs,
  AddRoleAction,
  AddRoleArgs,
  BuildTransactionArgs,
  CancelRecoveryArgs,
  ConfigureRecoveryArgs,
  CreateWalletArgs,
  ExecuteRecoveryArgs,
  Network,
  PrepareArgs,
  PrepareOperation,
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
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
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
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
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
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
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
  return {
    network: toProtoNetwork(
      resolveNetwork(args.network, wallet.network, defaultNetwork),
    ),
    feePayer: args.feePayer,
    swigAddress: wallet.swigConfigAddress,
    requesterAuthority: walletAuthorityRequest(
      resolveRequesterAuthority(wallet, args),
    ),
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
    | AddRoleArgs,
): NonNullable<
  | TransferArgs['requesterAuthority']
  | SwapArgs['requesterAuthority']
  | PrepareArgs['requesterAuthority']
  | BuildTransactionArgs['requesterAuthority']
  | CancelRecoveryArgs['requesterAuthority']
  | AddRecoveryAuthorityArgs['requesterAuthority']
  | AddRoleArgs['requesterAuthority']
> {
  const requesterAuthority =
    args.requesterAuthority ?? wallet.requesterAuthority;
  if (!requesterAuthority) {
    throw new Error('requesterAuthority is required');
  }
  return requesterAuthority;
}
