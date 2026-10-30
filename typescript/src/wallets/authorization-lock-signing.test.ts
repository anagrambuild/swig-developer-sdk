import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import { describe, expect, test } from 'bun:test';

import { signPreparedSwigTransaction } from '../client/index.js';
import {
  SwigClient,
  type DirectWalletAuthority,
  type PreparedTransaction,
} from '../index.js';

const SWIG_PROGRAM_ID = new PublicKey(
  'swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB',
);
const SECP256R1_PROGRAM_ID = new PublicKey(
  'Secp256r1SigVerify1111111111111111111111111',
);
const ADD_ARGS_SIZE = 88;
const REMOVE_ARGS_SIZE = 16;
const SLOT = 345_678_901n;
const COUNTER = 8;

const config = Keypair.generate().publicKey;
const feePayer = Keypair.generate().publicKey;
const walletAddress = Keypair.generate().publicKey;
const publicKey = Uint8Array.from({ length: 33 }, (_, index) =>
  index === 0 ? 3 : index + 40,
);
const publicKeyHex = bytesToHex(publicKey);
const messageHash = Uint8Array.from({ length: 32 }, (_, index) => 90 + index);
const messageHashHex = bytesToHex(messageHash);

type Scheme = 'secp256k1' | 'secp256r1';
type Route = 'add' | 'remove';

// AddAuthorizationLockV1 / RemoveAuthorizationLockV1 as the dev portal builds
// them: fixed-size args, then the authority payload with a zeroed signature.
function lockArgs(route: Route): Uint8Array<ArrayBuffer> {
  const args = new Uint8Array(
    route === 'add' ? ADD_ARGS_SIZE : REMOVE_ARGS_SIZE,
  );
  const view = new DataView(args.buffer);
  view.setUint16(0, route === 'add' ? 25 : 26, true);
  view.setUint32(4, 1, true);
  if (route === 'add') {
    args.set(walletAddress.toBytes(), 8);
    view.setBigUint64(72, 1_000_000n, true);
    view.setBigUint64(80, 9_000_000n, true);
  } else {
    view.setUint32(8, 3, true);
  }
  return args;
}

function authorityPayload(
  scheme: Scheme,
  sysvarIndex: number,
): Uint8Array<ArrayBuffer> {
  const payload = new Uint8Array(scheme === 'secp256k1' ? 77 : 17);
  const view = new DataView(payload.buffer);
  view.setBigUint64(0, SLOT, true);
  view.setUint32(8, COUNTER, true);
  if (scheme === 'secp256r1') {
    payload[12] = sysvarIndex;
  }
  return payload;
}

function backendTransaction(scheme: Scheme, route: Route): string {
  const keys = [
    { pubkey: config, isSigner: false, isWritable: true },
    { pubkey: feePayer, isSigner: true, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ...(route === 'add'
      ? [
          { pubkey: walletAddress, isSigner: false, isWritable: false },
          { pubkey: walletAddress, isSigner: false, isWritable: false },
        ]
      : []),
  ];
  const sysvarIndex = keys.length;
  if (scheme === 'secp256r1') {
    keys.push({
      pubkey: SYSVAR_INSTRUCTIONS_PUBKEY,
      isSigner: false,
      isWritable: false,
    });
  }
  const args = lockArgs(route);
  const payload = authorityPayload(scheme, sysvarIndex);
  const data = new Uint8Array(args.length + payload.length);
  data.set(args);
  data.set(payload, args.length);

  const transaction = new Transaction({
    feePayer,
    recentBlockhash: '11111111111111111111111111111111',
  });
  if (scheme === 'secp256r1') {
    transaction.add(
      new TransactionInstruction({
        programId: SECP256R1_PROGRAM_ID,
        keys: [],
        data: Buffer.from(
          secp256r1InstructionData(publicKey, new Uint8Array(64), messageHash),
        ),
      }),
    );
  }
  transaction.add(
    new TransactionInstruction({
      programId: SWIG_PROGRAM_ID,
      keys,
      data: Buffer.from(data),
    }),
  );
  return Buffer.from(
    transaction.serialize({
      requireAllSignatures: false,
      verifySignatures: false,
    }),
  ).toString('base64');
}

// ProtoJSON as the dev portal returns it: enum names, uint64 slot as a string.
function backendResponse(scheme: Scheme, route: Route) {
  return {
    transaction: {
      transaction: backendTransaction(scheme, route),
      transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
      wallet: {
        swigConfigAddress: config.toBase58(),
        walletAddress: walletAddress.toBase58(),
      },
      expiresAt: '2026-10-30T12:00:00Z',
      network: 'NETWORK_DEVNET',
      recentBlockhash: '11111111111111111111111111111111',
      kind:
        route === 'add'
          ? 'PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK'
          : 'PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK',
      signatureRequests: [
        {
          scheme:
            scheme === 'secp256k1'
              ? 'AUTHORITY_SIGNATURE_SCHEME_SECP256K1'
              : 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1',
          signer: publicKeyHex,
          messageHash: messageHashHex,
          slot: SLOT.toString(),
          counter: COUNTER,
        },
      ],
    },
  };
}

async function prepare(
  scheme: Scheme,
  route: Route,
): Promise<{ prepared: PreparedTransaction; body: unknown }> {
  let body: unknown;
  const client = new SwigClient({
    apiKey: 'test-key',
    baseUrl: 'https://api.example.test',
    network: 'devnet',
    fetch: (async (
      input: Parameters<typeof fetch>[0],
      init: Parameters<typeof fetch>[1],
    ) => {
      body = await new Request(input, init).json();
      return Response.json(backendResponse(scheme, route));
    }) as unknown as typeof fetch,
  });
  const requesterAuthority = {
    [scheme]: { publicKey: publicKeyHex },
  } as DirectWalletAuthority;
  const wallet = client.wallets.use(config.toBase58(), { requesterAuthority });
  const prepared =
    route === 'add'
      ? await wallet.addAuthorizationLock({
          feePayer: feePayer.toBase58(),
          amount: 1_000_000n,
          expirySlot: 9_000_000n,
        })
      : await wallet.removeAuthorizationLock({
          feePayer: feePayer.toBase58(),
          lockId: 3,
        });
  return { prepared, body };
}

function signedInstructions(signed: { transaction: string }) {
  const transaction = Transaction.from(
    Buffer.from(signed.transaction, 'base64'),
  );
  const lock = transaction.instructions.find((instruction) =>
    instruction.programId.equals(SWIG_PROGRAM_ID),
  );
  const precompile = transaction.instructions.find((instruction) =>
    instruction.programId.equals(SECP256R1_PROGRAM_ID),
  );
  if (!lock) throw new Error('lock instruction missing');
  return {
    lock: new Uint8Array(lock.data),
    precompile: precompile ? new Uint8Array(precompile.data) : undefined,
  };
}

describe.each(['add', 'remove'] as const)(
  'authorization lock %s signing',
  (route) => {
    const offset = route === 'add' ? ADD_ARGS_SIZE : REMOVE_ARGS_SIZE;

    test('writes a secp256k1 signature after the slot and counter', async () => {
      const { prepared, body } = await prepare('secp256k1', route);
      expect(body).toMatchObject({
        requesterAuthority: { secp256k1: { publicKey: publicKeyHex } },
      });
      expect(prepared.kind).toBe(
        route === 'add'
          ? 'add-authorization-lock'
          : 'remove-authorization-lock',
      );
      expect(prepared.signatureRequests).toEqual([
        {
          scheme: 'secp256k1',
          signer: publicKeyHex,
          messageHash: messageHashHex,
          slot: Number(SLOT),
          counter: COUNTER,
        },
      ]);
      const signature = Uint8Array.from(
        { length: 65 },
        (_, index) => 200 - index,
      );
      signature[64] = 28;

      const signed = await signPreparedSwigTransaction(prepared, {
        secp256k1: {
          [publicKeyHex]: async (message) => {
            expect(message).toEqual(new TextEncoder().encode(messageHashHex));
            return { signature };
          },
        },
      });

      const { lock } = signedInstructions(signed);
      expect(lock.length).toBe(offset + 77);
      expect(lock.slice(0, offset)).toEqual(lockArgs(route));
      const view = new DataView(lock.buffer, lock.byteOffset);
      expect(view.getBigUint64(offset, true)).toBe(SLOT);
      expect(view.getUint32(offset + 8, true)).toBe(COUNTER);
      expect(lock.slice(offset + 12, offset + 77)).toEqual(signature);
    });

    test('appends a secp256k1 signing prefix after the fixed payload', async () => {
      const { prepared } = await prepare('secp256k1', route);
      const signature = new Uint8Array(65).fill(7);
      signature[64] = 27;
      const prefix = new TextEncoder().encode(
        '\x19Ethereum Signed Message:\n64',
      );

      const signed = await signPreparedSwigTransaction(prepared, {
        secp256k1: async () => ({ signature, prefix }),
      });

      const { lock } = signedInstructions(signed);
      expect(lock.slice(offset + 12, offset + 77)).toEqual(signature);
      expect(lock.slice(offset + 77)).toEqual(prefix);
    });

    test('writes a secp256r1 signature into the preceding precompile', async () => {
      const { prepared, body } = await prepare('secp256r1', route);
      expect(body).toMatchObject({
        requesterAuthority: { secp256r1: { publicKey: publicKeyHex } },
      });
      const signature = Uint8Array.from({ length: 64 }, (_, index) => index);

      const signed = await signPreparedSwigTransaction(prepared, {
        secp256r1: async (challenge) => {
          expect(challenge).toEqual(messageHash);
          return { signature };
        },
      });

      const { lock, precompile } = signedInstructions(signed);
      if (!precompile) throw new Error('precompile missing');
      const view = new DataView(precompile.buffer, precompile.byteOffset);
      const signatureOffset = view.getUint16(2, true);
      const publicKeyOffset = view.getUint16(6, true);
      const messageOffset = view.getUint16(10, true);
      expect(precompile.slice(signatureOffset, signatureOffset + 64)).toEqual(
        signature,
      );
      expect(precompile.slice(publicKeyOffset, publicKeyOffset + 33)).toEqual(
        publicKey,
      );
      expect(precompile.slice(messageOffset)).toEqual(messageHash);
      expect(lock.length).toBe(offset + 17);
      expect(lock.slice(offset + 17)).toEqual(new Uint8Array(0));
    });

    test('appends a secp256r1 passkey prefix to the lock authority payload', async () => {
      const { prepared } = await prepare('secp256r1', route);
      const signature = new Uint8Array(64).fill(5);
      const message = Uint8Array.from([9, 8, 7, 6]);
      const prefix = Uint8Array.from([1, 3, 5, 7]);

      const signed = await signPreparedSwigTransaction(prepared, {
        secp256r1: async () => ({ signature, message, prefix }),
      });

      const { lock, precompile } = signedInstructions(signed);
      if (!precompile) throw new Error('precompile missing');
      const view = new DataView(precompile.buffer, precompile.byteOffset);
      expect(precompile.slice(view.getUint16(10, true))).toEqual(message);
      expect(lock.slice(0, offset)).toEqual(lockArgs(route));
      expect(lock.slice(offset, offset + 17)).toEqual(
        authorityPayload('secp256r1', route === 'add' ? 5 : 3),
      );
      expect(lock.slice(offset + 17)).toEqual(prefix);
    });
  },
);

describe('authorization lock requesters', () => {
  test('rejects ProgramExec and ParticipantSet requesters before sending', async () => {
    let calls = 0;
    const client = new SwigClient({
      apiKey: 'test-key',
      baseUrl: 'https://api.example.test',
      network: 'devnet',
      fetch: (async () => {
        calls += 1;
        return Response.json({});
      }) as unknown as typeof fetch,
    });
    for (const requesterAuthority of [
      { programExecProof: { roleId: 1, zkProof: 'proof' } },
      { participantSet: { address: 'set' } },
    ] as const) {
      const wallet = client.wallets.use('config', { requesterAuthority });
      await expect(
        wallet.addAuthorizationLock({
          feePayer: 'payer',
          amount: 1,
          durationSlots: 10,
        }),
      ).rejects.toThrow(
        'Authorization lock requesterAuthority must use ed25519, secp256k1, or secp256r1',
      );
      await expect(
        wallet.removeAuthorizationLock({ feePayer: 'payer', lockId: 0 }),
      ).rejects.toThrow(
        'Authorization lock requesterAuthority must use ed25519, secp256k1, or secp256r1',
      );
    }
    expect(calls).toBe(0);
  });
});

function secp256r1InstructionData(
  key: Uint8Array,
  signature: Uint8Array,
  message: Uint8Array,
): Uint8Array {
  const messageOffset = 16 + key.length + signature.length;
  const data = new Uint8Array(messageOffset + message.length);
  const view = new DataView(data.buffer);
  data[0] = 1;
  view.setUint16(2, 16 + key.length, true);
  view.setUint16(4, 65535, true);
  view.setUint16(6, 16, true);
  view.setUint16(8, 65535, true);
  view.setUint16(10, messageOffset, true);
  view.setUint16(12, message.length, true);
  view.setUint16(14, 65535, true);
  data.set(key, 16);
  data.set(signature, 16 + key.length);
  data.set(message, messageOffset);
  return data;
}

function bytesToHex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}
