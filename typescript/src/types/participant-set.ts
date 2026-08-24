import type { JsonObject, Network } from './common.js';
import type {
  PreparedTransaction,
  PreparedTransactionWire,
} from './transaction.js';
import type { WalletAuthority } from './wallet-actions.js';

export type ParticipantSetSignerType = 'secp256k1' | 'webauthnP256';

export type ParticipantSetMember =
  | { secp256k1: { publicKey: string } }
  | { webauthnP256: { publicKey: string } };

export interface CreateParticipantSetArgs {
  swigConfigAddress: string;
  feePayer: string;
  threshold: number;
  members: ParticipantSetMember[];
  setId?: string;
  network?: Network;
}

export interface CreateParticipantSetResult {
  participantSetAddress: string;
  setId: string;
  transaction: PreparedTransaction;
}

export interface CreateParticipantSetResponseWire {
  participantSetAddress?: string;
  participant_set_address?: string;
  setId?: string;
  set_id?: string;
  transaction?: PreparedTransactionWire;
}

export interface AddParticipantSetRoleArgs {
  feePayer: string;
  participantSetAddress: string;
  permissions: JsonObject[];
  requesterAuthority?: WalletAuthority;
  network?: Network;
}

export interface ParticipantApprovalRequest {
  memberIndex: number;
  signerType: ParticipantSetSignerType;
  publicKey: string;
  counter: number;
  challenge: string;
}

export interface ParticipantSetApprovalPlan {
  type: 'participantSet';
  participantSetAddress: string;
  roleId: number;
  expirationSlot: string;
  transactionDigest: string;
  compilationEnvelope: string;
  threshold: number;
  members: ParticipantApprovalRequest[];
}

export interface ParticipantSetApprovalPlanWire {
  participantSetAddress?: string;
  participant_set_address?: string;
  roleId?: number;
  role_id?: number;
  expirationSlot?: string | number;
  expiration_slot?: string | number;
  transactionDigest?: string;
  transaction_digest?: string;
  compilationEnvelope?: string;
  compilation_envelope?: string;
  threshold?: number;
  members?: ParticipantApprovalRequestWire[];
}

export interface ParticipantApprovalRequestWire {
  memberIndex?: number;
  member_index?: number;
  signerType?:
    | 'PARTICIPANT_SET_SIGNER_TYPE_SECP256K1'
    | 'PARTICIPANT_SET_SIGNER_TYPE_WEBAUTHN_P256'
    | number;
  signer_type?:
    | 'PARTICIPANT_SET_SIGNER_TYPE_SECP256K1'
    | 'PARTICIPANT_SET_SIGNER_TYPE_WEBAUTHN_P256'
    | number;
  publicKey?: string;
  public_key?: string;
  counter?: number;
  challenge?: string;
}

export type ParticipantSetApproval =
  | {
      memberIndex: number;
      counter: number;
      secp256k1: { signature: Uint8Array };
    }
  | {
      memberIndex: number;
      counter: number;
      webauthnP256: {
        authenticatorData: Uint8Array;
        clientDataJson: Uint8Array;
        signature: Uint8Array;
      };
    };

export interface CompileParticipantSetApprovalsArgs {
  preparedTransaction: PreparedTransaction;
  approvals: ParticipantSetApproval[];
}

export interface CompileParticipantSetApprovalsResult {
  preparedTransaction: PreparedTransaction;
}

export interface CompileParticipantSetApprovalsResponseWire {
  transaction?: PreparedTransactionWire;
}
