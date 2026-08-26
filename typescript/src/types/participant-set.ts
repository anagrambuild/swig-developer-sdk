import type { Network } from './common.js';
import type {
  PreparedTransaction,
  PreparedTransactionWire,
} from './transaction.js';
import type { WalletAuthority } from './wallet-actions.js';

export type ParticipantSetMember = Extract<
  WalletAuthority,
  { ed25519: unknown } | { secp256k1: unknown } | { secp256r1: unknown }
>;

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

export interface ParticipantApprovalRequest {
  memberIndex: number;
  authority: ParticipantSetMember;
  challenge: string;
}

export interface ParticipantSetApprovalPlan {
  type: 'participantSet';
  participantSetAddress: string;
  roleId: number;
  expirationSlot: string;
  nonce: number;
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
  nonce?: number;
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
  authority?: ParticipantSetMemberWire;
  challenge?: string;
}

export interface ParticipantSetMemberWire {
  ed25519?: { publicKey?: string; public_key?: string };
  secp256k1?: { publicKey?: string; public_key?: string };
  secp256r1?: { publicKey?: string; public_key?: string };
}

export type ParticipantSetApproval =
  | {
      memberIndex: number;
      ed25519: { signature: Uint8Array };
    }
  | {
      memberIndex: number;
      secp256k1: { signature: Uint8Array };
    }
  | {
      memberIndex: number;
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
  transaction: PreparedTransaction;
  authorizationExpirationSlot: string;
}

export interface CompileParticipantSetApprovalsResponseWire {
  transaction?: PreparedTransactionWire;
  authorizationExpirationSlot?: string | number;
  authorization_expiration_slot?: string | number;
}
