import type { HttpClient } from '../core/index.js';
import type {
  CreateParticipantSetArgs,
  CreateParticipantSetResponseWire,
  CreateParticipantSetResult,
  Network,
  ParticipantSetMember,
} from '../types/index.js';
import { normalizePreparedTransaction } from '../wallets/normalizers.js';

export class ParticipantSetsClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  create = async (
    args: CreateParticipantSetArgs,
  ): Promise<CreateParticipantSetResult> => {
    const network = args.network ?? this.defaultNetwork;
    if (!network) {
      throw new Error('network is required');
    }

    const response = await this.http.post<CreateParticipantSetResponseWire>(
      '/transaction/participant-set/create',
      {
        network: toProtoNetwork(network),
        feePayer: args.feePayer,
        swigAddress: args.swigConfigAddress,
        setId: args.setId,
        threshold: args.threshold,
        members: args.members.map(participantSetMemberToWire),
      },
    );
    const transaction = response.transaction;
    if (!transaction) {
      throw new Error('Create ParticipantSet response is missing transaction');
    }

    return {
      participantSetAddress: requiredString(
        response.participantSetAddress ?? response.participant_set_address,
        'participantSetAddress',
      ),
      setId: requiredString(response.setId ?? response.set_id, 'setId'),
      transaction: normalizePreparedTransaction(transaction),
    };
  };
}

function participantSetMemberToWire(member: ParticipantSetMember) {
  if ('secp256k1' in member) {
    return { secp256k1PublicKey: member.secp256k1.publicKey };
  }
  return { webauthnP256PublicKey: member.webauthnP256.publicKey };
}

function toProtoNetwork(network: Network): string {
  return network === 'mainnet' ? 'NETWORK_MAINNET' : 'NETWORK_DEVNET';
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`Create ParticipantSet response is missing ${field}`);
  }
  return value;
}
