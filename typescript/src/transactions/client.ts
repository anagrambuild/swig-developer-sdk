import bs58 from 'bs58';
import type { HttpClient } from '../core/index.js';
import type {
  CompileParticipantSetApprovalsArgs,
  CompileParticipantSetApprovalsResponseWire,
  CompileParticipantSetApprovalsResult,
  Network,
  ParticipantSetApproval,
  PreparedTransaction,
  SponsorSignedTransactionArgs,
  SponsorSignedTransactionBundleArgs,
  SubmittedTransaction,
  SubmittedTransactionBundle,
  SubmittedTransactionBundleWire,
  SubmittedTransactionWire,
} from '../types/index.js';
import {
  normalizePreparedTransaction,
  normalizeSubmittedTransaction,
} from '../wallets/normalizers.js';

export class TransactionsClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  sponsor = async (
    args: SponsorSignedTransactionArgs,
  ): Promise<SubmittedTransaction> => {
    const response = await this.http.post<SubmittedTransactionWire>(
      '/paymaster/sponsor',
      {
        base58_encoded_transaction: bs58.encode(
          base64ToBytes(args.transaction),
        ),
        network: args.network ?? this.defaultNetwork,
        idempotencyKey: args.idempotencyKey,
      },
      { retry: Boolean(args.idempotencyKey) },
    );

    return normalizeSubmittedTransaction(response);
  };

  sponsorBundle = async (
    args: SponsorSignedTransactionBundleArgs,
  ): Promise<SubmittedTransactionBundle> => {
    if (args.transactions.length === 0 || args.transactions.length > 5) {
      throw new Error('transactions must contain between 1 and 5 items');
    }
    const network = args.network ?? this.defaultNetwork;
    if (network !== 'mainnet') {
      throw new Error('sponsorBundle only supports mainnet');
    }
    const response = await this.http.post<SubmittedTransactionBundleWire>(
      '/paymaster/sponsor/bundle',
      {
        base58_encoded_transactions: args.transactions.map((transaction) =>
          bs58.encode(base64ToBytes(transaction)),
        ),
        network,
        idempotencyKey: args.idempotencyKey,
      },
      { retry: Boolean(args.idempotencyKey) },
    );
    return {
      requestId: requiredString(
        response.requestId ?? response.request_id,
        'requestId',
      ),
      bundleId: requiredString(
        response.bundleId ?? response.bundle_id,
        'bundleId',
      ),
      signatures: response.signatures ?? [],
      estimatedSpentByPaymaster: String(
        response.estimatedSpentByPaymaster ??
          response.estimated_spent_by_paymaster ??
          '0',
      ),
    };
  };

  compileParticipantSetApprovals = async (
    args: CompileParticipantSetApprovalsArgs,
  ): Promise<CompileParticipantSetApprovalsResult> => {
    const response =
      await this.http.post<CompileParticipantSetApprovalsResponseWire>(
        '/transaction/participant-set/compile',
        {
          preparedTransaction: preparedTransactionToWire(
            args.preparedTransaction,
          ),
          approvals: args.approvals.map(participantSetApprovalToWire),
        },
      );
    if (!response.transaction) {
      throw new Error('Compile ParticipantSet response is missing transaction');
    }
    return {
      preparedTransaction: normalizePreparedTransaction(response.transaction),
    };
  };
}

function preparedTransactionToWire(transaction: PreparedTransaction) {
  const plan = transaction.participantSetApprovalPlan;
  return {
    transaction: transaction.transaction,
    transactionEncoding: transaction.transactionEncoding
      ? 'TRANSACTION_ENCODING_BASE64'
      : undefined,
    wallet: transaction.wallet,
    expiresAt: transaction.expiresAt,
    network: transaction.network
      ? toProtoNetwork(transaction.network)
      : undefined,
    recentBlockhash: transaction.recentBlockhash,
    kind: transaction.kind
      ? preparedTransactionKindToWire(transaction.kind)
      : undefined,
    signatureRequests: transaction.signatureRequests.map((request) => ({
      scheme:
        request.scheme === 'secp256r1'
          ? 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1'
          : 'AUTHORITY_SIGNATURE_SCHEME_SECP256K1',
      signer: request.signer,
      messageHash: request.messageHash,
      slot: String(request.slot),
      counter: request.counter,
    })),
    participantSetApprovalPlan: plan
      ? {
          participantSetAddress: plan.participantSetAddress,
          roleId: plan.roleId,
          expirationSlot: plan.expirationSlot,
          transactionDigest: plan.transactionDigest,
          compilationEnvelope: plan.compilationEnvelope,
          threshold: plan.threshold,
          members: plan.members.map((member) => ({
            memberIndex: member.memberIndex,
            signerType:
              member.signerType === 'secp256k1'
                ? 'PARTICIPANT_SET_SIGNER_TYPE_SECP256K1'
                : 'PARTICIPANT_SET_SIGNER_TYPE_WEBAUTHN_P256',
            publicKey: member.publicKey,
            counter: member.counter,
            challenge: member.challenge,
          })),
        }
      : undefined,
  };
}

function participantSetApprovalToWire(approval: ParticipantSetApproval) {
  return 'secp256k1' in approval
    ? {
        memberIndex: approval.memberIndex,
        counter: approval.counter,
        secp256k1: {
          signature: bytesToBase64(approval.secp256k1.signature),
        },
      }
    : {
        memberIndex: approval.memberIndex,
        counter: approval.counter,
        webauthnP256: {
          authenticatorData: bytesToBase64(
            approval.webauthnP256.authenticatorData,
          ),
          clientDataJson: bytesToBase64(approval.webauthnP256.clientDataJson),
          signature: bytesToBase64(approval.webauthnP256.signature),
        },
      };
}

function preparedTransactionKindToWire(kind: PreparedTransaction['kind']) {
  switch (kind) {
    case 'create-swig-wallet':
      return 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET';
    case 'add-authority':
      return 'PREPARED_TRANSACTION_KIND_ADD_AUTHORITY';
    case 'configure-recovery':
      return 'PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY';
    case 'create-participant-set':
      return 'PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET';
    case undefined:
      return undefined;
  }
}

function toProtoNetwork(network: Network): string {
  return network === 'mainnet' ? 'NETWORK_MAINNET' : 'NETWORK_DEVNET';
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`Sponsor bundle response is missing ${field}`);
  }
  return value;
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);

  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  return bytes;
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}
