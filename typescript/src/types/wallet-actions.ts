import type { Amount, Network } from './common.js';
import type { SolanaInstructionInput } from './instruction.js';
import type { CreateWalletResult } from './transaction.js';

export type WalletAuthority =
  | { ed25519: { publicKey: string } }
  | { secp256k1: { publicKey: string } }
  | { secp256r1: { publicKey: string } }
  | { programExecProof: { roleId: number; zkProof: string } }
  | {
      participantSet: {
        address: string;
        roleId?: number;
      };
    };

export type WalletAuthorityKind = 'ed25519' | 'secp256k1' | 'secp256r1';

export interface CreateWalletArgs {
  policyId?: string;
  feePayer: string;
  initialUser?: WalletAuthority;
  recovery?: {
    guardianPubkey?: string;
    delaySeconds?: number;
    targetRoleId?: number;
  };
  network?: Network;
}

export type CreateWalletResponse = CreateWalletResult;

export interface BaseTransferArgs {
  feePayer: string;
  requesterAuthority?: WalletAuthority;
  amount: Amount;
  network?: Network;
}

export interface TransferSolArgs extends BaseTransferArgs {
  destination: string;
  mint?: undefined;
}

export interface TransferTokenArgs extends BaseTransferArgs {
  mint: string;
  destinationOwner: string;
}

export type TransferArgs = TransferSolArgs | TransferTokenArgs;

export type PrepareOperation =
  | {
      type: 'transferSol';
      destination: string;
      amount: Amount;
    }
  | {
      type: 'transferToken';
      mint: string;
      destinationOwner: string;
      amount: Amount;
    };

export interface PrepareArgs {
  feePayer: string;
  requesterAuthority?: WalletAuthority;
  operations: PrepareOperation[];
  network?: Network;
}

export interface SwapArgs {
  feePayer: string;
  requesterAuthority?: WalletAuthority;
  inputMint: string;
  outputMint: string;
  amount: Amount;
  slippageBps?: number;
  destinationAccount?: string;
  wrapAndUnwrapSol?: boolean;
  tipAmountLamports?: Amount;
  computeUnitPricePercentile?: string;
  maxAccounts?: number;
  mode?: string;
  blockhashSlotsToExpiry?: number;
  network?: Network;
}

export interface BaseRecoveryArgs {
  feePayer: string;
  network?: Network;
}

export interface AddRecoveryAuthorityArgs extends BaseRecoveryArgs {
  requesterAuthority?: WalletAuthority;
}

export interface ConfigureRecoveryArgs extends BaseRecoveryArgs {
  guardianPubkey: string;
  delaySeconds?: number;
  targetRoleId?: number;
}

export interface PrepareRecoverySetupArgs
  extends AddRecoveryAuthorityArgs, ConfigureRecoveryArgs {}

export type StartRecoveryArgs = BaseRecoveryArgs & {
  newAuthority: string;
  newAuthorityKind: WalletAuthorityKind;
} & (
    | {
        guardianPubkey: string;
        guardianSwigAddress?: never;
        guardianRequesterAuthority?: never;
      }
    | {
        guardianPubkey?: never;
        guardianSwigAddress: string;
        guardianRequesterAuthority: WalletAuthority;
      }
  );

export interface CancelRecoveryArgs extends BaseRecoveryArgs {
  requesterAuthority?: WalletAuthority;
}

export type ExecuteRecoveryArgs = BaseRecoveryArgs & {
  newAuthority: string;
  newAuthorityKind: WalletAuthorityKind;
} & (
    | {
        guardianSwigAddress?: never;
        guardianRequesterAuthority?: never;
      }
    | {
        guardianSwigAddress: string;
        guardianRequesterAuthority: WalletAuthority;
      }
  );

export interface BuildTransactionArgs {
  feePayer: string;
  requesterAuthority?: WalletAuthority;
  instructions: SolanaInstructionInput[];
  addressLookupTableAccounts?: string[];
  network?: Network;
}
