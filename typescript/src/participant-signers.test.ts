import { afterEach, describe, expect, test } from 'bun:test';

import {
  createParticipantEd25519Signer,
  createParticipantPasskeySigner,
  createParticipantPersonalSignSigner,
  signParticipantSetApproval,
} from './participant-signers.js';

const SECP256K1_ORDER = BigInt(
  '0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141',
);
const originalNavigator = Object.getOwnPropertyDescriptor(
  globalThis,
  'navigator',
);

afterEach(() => {
  if (originalNavigator) {
    Object.defineProperty(globalThis, 'navigator', originalNavigator);
  } else {
    delete (globalThis as { navigator?: Navigator }).navigator;
  }
});

describe('ParticipantSet signers', () => {
  test('signs the lowercase ASCII challenge through personal_sign callback semantics', async () => {
    const publicKey = `02${'11'.repeat(32)}`;
    const challenge = 'AB'.repeat(32);
    const messages: string[] = [];
    const signature = new Uint8Array(65);
    signature[31] = 1;
    signature.set(bigIntToFixedBytes(SECP256K1_ORDER - 2n, 32), 32);
    signature[64] = 27;
    const signer = createParticipantPersonalSignSigner({
      publicKey,
      signMessage: (message) => {
        messages.push(message);
        return signature;
      },
    });

    const approval = await signParticipantSetApproval(
      {
        memberIndex: 3,
        authority: {
          secp256k1: { publicKey: publicKey.toUpperCase() },
        },
        challenge,
      },
      signer,
    );

    expect(messages).toEqual([challenge.toLowerCase()]);
    expect(approval).toEqual({
      memberIndex: 3,
      secp256k1: {
        signature: Uint8Array.from([
          ...signature.slice(0, 32),
          ...bigIntToFixedBytes(2n, 32),
          28,
        ]),
      },
    });
  });

  test('signs decoded challenge bytes with an Ed25519 callback', async () => {
    const challenge = '44'.repeat(32);
    const messages: Uint8Array[] = [];
    const signature = Uint8Array.from({ length: 64 }, (_, index) => index);

    const approval = await signParticipantSetApproval(
      {
        memberIndex: 2,
        authority: { ed25519: { publicKey: 'ed25519_public_key' } },
        challenge,
      },
      createParticipantEd25519Signer({
        publicKey: 'ed25519_public_key',
        signMessage: (message) => {
          messages.push(message);
          return signature;
        },
      }),
    );

    expect(messages).toEqual([hexToBytes(challenge)]);
    expect(approval).toEqual({
      memberIndex: 2,
      ed25519: { signature },
    });
  });

  test('returns the raw passkey assertion bound to the requested challenge', async () => {
    const publicKey = `03${'22'.repeat(32)}`;
    const challenge = '33'.repeat(32);
    const credentialId = Uint8Array.from([8, 9]);
    const authenticatorData = Uint8Array.from([1, 2, 3]);
    const clientDataJson = new TextEncoder().encode(
      '{"type":"webauthn.get","challenge":"bound"}',
    );
    const r = Uint8Array.from({ length: 32 }, (_, index) => index + 1);
    const s = Uint8Array.from({ length: 32 }, (_, index) => index + 33);

    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: {
        credentials: {
          get: async (request: CredentialRequestOptions) => {
            expect([
              ...new Uint8Array(request.publicKey?.challenge as ArrayBuffer),
            ]).toEqual([...hexToBytes(challenge)]);
            expect(request.publicKey?.allowCredentials?.[0]?.id).toEqual(
              credentialId,
            );
            return {
              response: {
                authenticatorData: toArrayBuffer(authenticatorData),
                clientDataJSON: toArrayBuffer(clientDataJson),
                signature: toArrayBuffer(derSignatureBytes(r, s)),
              },
            };
          },
        },
      },
    });

    const approval = await signParticipantSetApproval(
      {
        memberIndex: 1,
        authority: { secp256r1: { publicKey } },
        challenge,
      },
      createParticipantPasskeySigner({ publicKey, credentialId }),
    );

    expect(approval).toEqual({
      memberIndex: 1,
      webauthnP256: {
        authenticatorData,
        clientDataJson,
        signature: Uint8Array.from([...r, ...s]),
      },
    });
  });

  test('rejects a signer for another member key before prompting', async () => {
    const signer = createParticipantPersonalSignSigner({
      publicKey: `02${'11'.repeat(32)}`,
      signMessage: () => {
        throw new Error('must not sign');
      },
    });

    await expect(
      signParticipantSetApproval(
        {
          memberIndex: 0,
          authority: {
            secp256k1: { publicKey: `02${'22'.repeat(32)}` },
          },
          challenge: '33'.repeat(32),
        },
        signer,
      ),
    ).rejects.toThrow(
      'Participant signer public key does not match approval request',
    );
  });
});

function derSignatureBytes(r: Uint8Array, s: Uint8Array): Uint8Array {
  const encodedR = derIntegerBytes(r);
  const encodedS = derIntegerBytes(s);
  return Uint8Array.from([
    0x30,
    encodedR.length + encodedS.length,
    ...encodedR,
    ...encodedS,
  ]);
}

function derIntegerBytes(bytes: Uint8Array): Uint8Array {
  const needsLeadingZero = (bytes[0] & 0x80) !== 0;
  return Uint8Array.from([
    0x02,
    bytes.length + (needsLeadingZero ? 1 : 0),
    ...(needsLeadingZero ? [0] : []),
    ...bytes,
  ]);
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

function hexToBytes(value: string): Uint8Array {
  return Uint8Array.from({ length: value.length / 2 }, (_, index) =>
    Number.parseInt(value.slice(index * 2, index * 2 + 2), 16),
  );
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
}
