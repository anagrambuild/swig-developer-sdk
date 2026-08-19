import type { Network } from './common.js';
import type {
  ParticipantSetApprovalPlan,
  ParticipantSetApprovalPlanWire,
} from './participant-set.js';
import type { DirectWalletAuthority } from './wallet-actions.js';
import type { WalletAddressInfo } from './wallet.js';

export type TransactionEncoding = 'base64';
export type ProtoTransactionEncoding =
  'TRANSACTION_ENCODING_UNSPECIFIED' | 'TRANSACTION_ENCODING_BASE64';
export type TransactionEncodingWire =
  TransactionEncoding | ProtoTransactionEncoding | number;

export type ProtoNetwork =
  'NETWORK_UNSPECIFIED' | 'NETWORK_DEVNET' | 'NETWORK_MAINNET';
export type NetworkWire = Network | ProtoNetwork | number;

export type PreparedTransactionKind =
  | 'create-swig-wallet'
  | 'add-authority'
  | 'configure-recovery'
  | 'create-participant-set'
  | 'x402-payment';
export type ProtoPreparedTransactionKind =
  | 'PREPARED_TRANSACTION_KIND_UNSPECIFIED'
  | 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET'
  | 'PREPARED_TRANSACTION_KIND_ADD_AUTHORITY'
  | 'PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY'
  | 'PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET'
  | 'PREPARED_TRANSACTION_KIND_X402_PAYMENT';
export type PreparedTransactionKindWire =
  PreparedTransactionKind | ProtoPreparedTransactionKind | number;

export interface PreparedTransaction {
  transaction: string;
  transactionEncoding?: TransactionEncoding;
  wallet?: WalletAddressInfo;
  expiresAt?: string;
  network?: Network;
  recentBlockhash?: string;
  kind?: PreparedTransactionKind;
  signatureRequests: ClientSignatureRequest[];
  participantSetApprovalPlan?: ParticipantSetApprovalPlan;
}

export interface PreparedTransactionWire {
  transaction?: string;
  unsigned_transaction?: string;
  unsignedTransaction?: string;
  transaction_encoding?: TransactionEncodingWire;
  transactionEncoding?: TransactionEncodingWire;
  wallet?: WalletAddressInfo;
  expires_at?: string;
  expiresAt?: string;
  network?: NetworkWire;
  recent_blockhash?: string;
  recentBlockhash?: string;
  kind?: PreparedTransactionKindWire;
  signature_requests?: ClientSignatureRequestWire[];
  signatureRequests?: ClientSignatureRequestWire[];
  participant_set_approval_plan?: ParticipantSetApprovalPlanWire;
  participantSetApprovalPlan?: ParticipantSetApprovalPlanWire;
}

export interface ClientSignatureRequest {
  scheme: 'secp256r1' | 'secp256k1';
  signer: string;
  messageHash: string;
  slot: number;
  counter: number;
}

export interface ClientSignatureRequestWire {
  scheme?:
    | ClientSignatureRequest['scheme']
    | 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1'
    | 'AUTHORITY_SIGNATURE_SCHEME_SECP256K1'
    | number;
  signer?: string;
  message_hash?: string;
  messageHash?: string;
  slot?: number | string;
  counter?: number;
}

export interface CreateWalletResponseWire extends PreparedTransactionWire {
  wallet?: WalletAddressInfo;
  transactions?: PreparedTransactionWire[];
}

export interface PrepareTransactionsResponseWire {
  wallet?: WalletAddressInfo;
  transactions?: PreparedTransactionWire[];
  network?: NetworkWire;
}

export interface PreparedTransactionsResult {
  wallet?: WalletAddressInfo & { network?: Network };
  transactions: PreparedTransaction[];
  clientAuthorityTransactions: PreparedTransaction[];
  feePayerOnlyTransactions: PreparedTransaction[];
  network?: Network;
}

export interface RecoverySetupPlan {
  requesterAuthority: DirectWalletAuthority;
  guardianPubkey: string;
  delaySeconds: number;
  targetRoleId?: number;
}

export interface CreateWalletResult {
  wallet: WalletAddressInfo & { network?: Network };
  transactions: PreparedTransaction[];
  clientAuthorityTransactions: PreparedTransaction[];
  operatorSignedTransactions: PreparedTransaction[];
  feePayerOnlyTransactions: PreparedTransaction[];
  creationTransaction?: PreparedTransaction;
  addAuthorityTransaction?: PreparedTransaction;
  configureRecoveryTransaction?: PreparedTransaction;
  recoverySetup?: RecoverySetupPlan;
  network?: Network;
}

export interface PreparedRecoverySetupResult {
  wallet?: WalletAddressInfo & { network?: Network };
  transactions: PreparedTransaction[];
  clientAuthorityTransactions: PreparedTransaction[];
  operatorSignedTransactions: PreparedTransaction[];
  feePayerOnlyTransactions: PreparedTransaction[];
  addAuthorityTransaction?: PreparedTransaction;
  configureRecoveryTransaction?: PreparedTransaction;
  network?: Network;
}

export interface SponsorSignedTransactionArgs {
  transaction: string;
  transactionEncoding?: TransactionEncoding;
  network?: Network;
  idempotencyKey?: string;
}

export interface SubmittedTransaction {
  requestId: string;
  signature: string;
  spentByPaymaster: string;
}

export interface SubmittedTransactionWire {
  request_id?: string;
  requestId?: string;
  signature?: string;
  spent_by_paymaster?: number | string;
  spentByPaymaster?: number | string;
}

export interface SponsorSignedTransactionBundleArgs {
  transactions: string[];
  network?: Network;
  idempotencyKey?: string;
}

export interface SubmittedTransactionBundle {
  requestId: string;
  bundleId: string;
  signatures: string[];
  estimatedSpentByPaymaster: string;
}

export interface SubmittedTransactionBundleWire {
  request_id?: string;
  requestId?: string;
  bundle_id?: string;
  bundleId?: string;
  signatures?: string[];
  estimated_spent_by_paymaster?: number | string;
  estimatedSpentByPaymaster?: number | string;
}
