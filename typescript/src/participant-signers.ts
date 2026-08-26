import { secp256r1DerToRawSignature } from './passkeys/index.js';
import type {
  ParticipantApprovalRequest,
  ParticipantSetApproval,
} from './types/index.js';

const SECP256K1_ORDER = BigInt(
  '0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141',
);
const SECP256K1_HALF_ORDER = SECP256K1_ORDER >> 1n;

export type ParticipantSignerType = 'ed25519' | 'secp256k1' | 'secp256r1';

export interface ParticipantSigner {
  readonly type: ParticipantSignerType;
  readonly publicKey: string;
  sign(request: ParticipantApprovalRequest): Promise<ParticipantSetApproval>;
}

export interface CreateParticipantEd25519SignerOptions {
  publicKey: string;
  signMessage(message: Uint8Array): Promise<Uint8Array> | Uint8Array;
}

export interface CreateParticipantPasskeySignerOptions {
  publicKey: string;
  credentialId: BufferSource;
  rpId?: string;
  timeout?: number;
  userVerification?: UserVerificationRequirement;
}

export interface CreateParticipantPersonalSignSignerOptions {
  publicKey: string;
  signMessage(
    message: string,
  ): Promise<string | Uint8Array> | string | Uint8Array;
}

export function createParticipantEd25519Signer(
  options: CreateParticipantEd25519SignerOptions,
): ParticipantSigner {
  return {
    type: 'ed25519',
    publicKey: options.publicKey,
    sign: async (request) => {
      assertMatchingRequest(request, 'ed25519', options.publicKey);
      const signature = await options.signMessage(
        participantChallengeBytes(request.challenge),
      );
      if (signature.length !== 64) {
        throw new Error('Participant ed25519 signature must be 64 bytes');
      }
      return {
        memberIndex: request.memberIndex,
        ed25519: { signature: signature.slice() },
      };
    },
  };
}

export function createParticipantPasskeySigner(
  options: CreateParticipantPasskeySignerOptions,
): ParticipantSigner {
  return {
    type: 'secp256r1',
    publicKey: options.publicKey,
    sign: async (request) => {
      assertMatchingRequest(request, 'secp256r1', options.publicKey);
      const assertion = (await navigator.credentials.get({
        publicKey: {
          challenge: toArrayBuffer(
            participantChallengeBytes(request.challenge),
          ),
          allowCredentials: [
            {
              id: options.credentialId,
              type: 'public-key',
            },
          ],
          rpId: options.rpId,
          timeout: options.timeout,
          userVerification: options.userVerification,
        },
      })) as PublicKeyCredential | null;
      if (!assertion?.response) {
        throw new Error('Failed to get participant passkey assertion');
      }
      const response = assertion.response as AuthenticatorAssertionResponse;
      return {
        memberIndex: request.memberIndex,
        webauthnP256: {
          authenticatorData: new Uint8Array(response.authenticatorData),
          clientDataJson: new Uint8Array(response.clientDataJSON),
          signature: secp256r1DerToRawSignature(
            new Uint8Array(response.signature),
          ),
        },
      };
    },
  };
}

export function createParticipantPersonalSignSigner(
  options: CreateParticipantPersonalSignSignerOptions,
): ParticipantSigner {
  return {
    type: 'secp256k1',
    publicKey: options.publicKey,
    sign: async (request) => {
      assertMatchingRequest(request, 'secp256k1', options.publicKey);
      const challenge = normalizeHex(request.challenge);
      if (challenge.length !== 64) {
        throw new Error('Participant challenge must be 32 bytes');
      }
      const signed = await options.signMessage(challenge);
      return {
        memberIndex: request.memberIndex,
        secp256k1: {
          signature: normalizeSecp256k1Signature(
            typeof signed === 'string' ? hexToBytes(signed) : signed,
          ),
        },
      };
    },
  };
}

export async function signParticipantSetApproval(
  request: ParticipantApprovalRequest,
  signer: ParticipantSigner,
): Promise<ParticipantSetApproval> {
  assertMatchingRequest(request, signer.type, signer.publicKey);
  const approval = await signer.sign(request);
  if (approval.memberIndex !== request.memberIndex) {
    throw new Error('Participant signer returned approval for another request');
  }
  if (
    (signer.type === 'ed25519' && !('ed25519' in approval)) ||
    (signer.type === 'secp256k1' && !('secp256k1' in approval)) ||
    (signer.type === 'secp256r1' && !('webauthnP256' in approval))
  ) {
    throw new Error('Participant signer returned the wrong proof type');
  }
  return approval;
}

function assertMatchingRequest(
  request: ParticipantApprovalRequest,
  signerType: ParticipantSignerType,
  publicKey: string,
): void {
  const authority = participantApprovalAuthority(request);
  if (authority.type !== signerType) {
    throw new Error('Participant signer type does not match approval request');
  }
  const matches =
    signerType === 'ed25519'
      ? authority.publicKey === publicKey
      : normalizeHex(authority.publicKey) === normalizeHex(publicKey);
  if (!matches) {
    throw new Error(
      'Participant signer public key does not match approval request',
    );
  }
}

function participantApprovalAuthority(request: ParticipantApprovalRequest): {
  type: ParticipantSignerType;
  publicKey: string;
} {
  if ('ed25519' in request.authority) {
    return {
      type: 'ed25519',
      publicKey: request.authority.ed25519.publicKey,
    };
  }
  if ('secp256k1' in request.authority) {
    return {
      type: 'secp256k1',
      publicKey: request.authority.secp256k1.publicKey,
    };
  }
  return {
    type: 'secp256r1',
    publicKey: request.authority.secp256r1.publicKey,
  };
}

function normalizeSecp256k1Signature(signature: Uint8Array): Uint8Array {
  if (signature.length !== 65) {
    throw new Error('Participant secp256k1 signature must be 65 bytes');
  }
  const normalized = signature.slice();
  let recovery = normalizeRecoveryByte(normalized[64]);
  const r = bytesToBigInt(normalized.slice(0, 32));
  let s = bytesToBigInt(normalized.slice(32, 64));
  if (r === 0n || r >= SECP256K1_ORDER || s === 0n || s >= SECP256K1_ORDER) {
    throw new Error('Participant secp256k1 signature has invalid scalars');
  }
  if (s > SECP256K1_HALF_ORDER) {
    s = SECP256K1_ORDER - s;
    recovery = recovery === 27 ? 28 : 27;
    normalized.set(bigIntToFixedBytes(s, 32), 32);
  }
  normalized[64] = recovery;
  return normalized;
}

function normalizeRecoveryByte(value: number): 27 | 28 {
  if (value === 0 || value === 27) {
    return 27;
  }
  if (value === 1 || value === 28) {
    return 28;
  }
  throw new Error('Participant secp256k1 signature has invalid recovery byte');
}

function hexToBytes(value: string): Uint8Array {
  const normalized = normalizeHex(value);
  if (normalized.length % 2 !== 0) {
    throw new Error('Hex strings must contain an even number of characters');
  }
  return Uint8Array.from({ length: normalized.length / 2 }, (_, index) =>
    Number.parseInt(normalized.slice(index * 2, index * 2 + 2), 16),
  );
}

function participantChallengeBytes(challenge: string): Uint8Array {
  const bytes = hexToBytes(challenge);
  if (bytes.length !== 32) {
    throw new Error('Participant challenge must be 32 bytes');
  }
  return bytes;
}

function normalizeHex(value: string): string {
  const normalized = value.startsWith('0x') ? value.slice(2) : value;
  if (!normalized || !/^[\da-fA-F]+$/.test(normalized)) {
    throw new Error('Invalid hex string');
  }
  return normalized.toLowerCase();
}

function bytesToBigInt(bytes: Uint8Array): bigint {
  let value = 0n;
  for (const byte of bytes) {
    value = (value << 8n) | BigInt(byte);
  }
  return value;
}

function bigIntToFixedBytes(value: bigint, length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  let remaining = value;
  for (let index = length - 1; index >= 0; index--) {
    bytes[index] = Number(remaining & 0xffn);
    remaining >>= 8n;
  }
  return bytes;
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
}
