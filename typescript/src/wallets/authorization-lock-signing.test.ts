import { secp256k1 } from '@noble/curves/secp256k1';
import { sha256 } from '@noble/hashes/sha2';
import { keccak_256 } from '@noble/hashes/sha3';
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
  TransactionInstruction,
  type AccountMeta,
} from '@solana/web3.js';
import { describe, expect, test } from 'bun:test';

import {
  signPreparedSwigTransaction,
  type Secp256k1SigningFn,
} from '../client/index.js';
import {
  SwigClient,
  type AuthorizationLockRequesterAuthority,
} from '../index.js';
import {
  createParticipantEd25519Signer,
  signParticipantSetApproval,
} from '../participant-signers.js';

const SWIG_PROGRAM_ID = new PublicKey(
  'swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB',
);
const SECP256R1_PROGRAM_ID = new PublicKey(
  'Secp256r1SigVerify1111111111111111111111111',
);
const ARGS_SIZE = { add: 88, remove: 16 } as const;
const PERSONAL_SIGN_PREFIX = new TextEncoder().encode(
  '\x19Ethereum Signed Message:\n64',
);
const SLOT = 345_678_901n;
const COUNTER = 8;

const config = Keypair.generate().publicKey;
const walletAddress = Keypair.generate().publicKey;
const feePayer = Keypair.generate().publicKey;
const rentClaimer = Keypair.generate().publicKey;
const participantSet = Keypair.generate().publicKey;

const k1PrivateKey = Uint8Array.from({ length: 32 }, (_, index) => index + 7);
const k1PublicKey = secp256k1.getPublicKey(k1PrivateKey, true);
const r1PublicKey = Uint8Array.from({ length: 33 }, (_, index) =>
  index === 0 ? 3 : index + 40,
);

type Route = keyof typeof ARGS_SIZE;
type Requester = 'secp256k1' | 'secp256r1' | 'participantSet';

// The lock instructions as the dev portal builds them. Add: swig, wallet
// address, payer, system program, asset account, then authority accounts.
// Remove: swig, wallet address, payer, system program, rent claimer, then
// authority accounts.
function lockArgs(route: Route): Uint8Array<ArrayBuffer> {
  const args = new Uint8Array(ARGS_SIZE[route]);
  const view = new DataView(args.buffer);
  view.setUint16(0, route === 'add' ? 25 : 26, true);
  view.setUint32(4, 4, true);
  if (route === 'add') {
    args.set(walletAddress.toBytes(), 8);
    view.setBigUint64(72, 1_000_000n, true);
    view.setBigUint64(80, 9_000_000n, true);
  } else {
    view.setUint32(8, 3, true);
  }
  return args;
}

function lockAccounts(route: Route, claimer: PublicKey): AccountMeta[] {
  return mergeTransactionFlags([
    { pubkey: config, isSigner: false, isWritable: true },
    { pubkey: walletAddress, isSigner: false, isWritable: false },
    { pubkey: feePayer, isSigner: true, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    route === 'add'
      ? { pubkey: walletAddress, isSigner: false, isWritable: false }
      : { pubkey: claimer, isSigner: false, isWritable: true },
  ]);
}

/** The dev portal hashes transaction-level flags, merged over repeated keys. */
function mergeTransactionFlags(accounts: AccountMeta[]): AccountMeta[] {
  return accounts.map((account) => {
    const same = accounts.filter((other) =>
      other.pubkey.equals(account.pubkey),
    );
    return {
      pubkey: account.pubkey,
      isSigner: same.some((other) => other.isSigner),
      isWritable: same.some((other) => other.isWritable),
    };
  });
}

/** args || 40-byte AccountsPayload per account || slot || counter. */
function signedPayload(
  args: Uint8Array,
  accounts: AccountMeta[],
  slot: bigint,
  counter: number,
): Uint8Array {
  const payload = new Uint8Array(args.length + accounts.length * 40 + 12);
  const view = new DataView(payload.buffer);
  payload.set(args);
  accounts.forEach((account, index) => {
    const offset = args.length + index * 40;
    payload.set(account.pubkey.toBytes(), offset);
    payload[offset + 32] = account.isWritable ? 1 : 0;
    payload[offset + 33] = account.isSigner ? 1 : 0;
  });
  const tail = args.length + accounts.length * 40;
  view.setBigUint64(tail, slot, true);
  view.setUint32(tail + 8, counter, true);
  return payload;
}

function slotCounter(): Uint8Array {
  const bytes = new Uint8Array(12);
  const view = new DataView(bytes.buffer);
  view.setBigUint64(0, SLOT, true);
  view.setUint32(8, COUNTER, true);
  return bytes;
}

function backendPrepared(
  requester: Requester,
  route: Route,
  claimer: PublicKey,
) {
  const args = lockArgs(route);
  const keys = lockAccounts(route, claimer);
  const transaction = new Transaction({
    feePayer,
    recentBlockhash: '11111111111111111111111111111111',
  });
  let data: Uint8Array;
  let messageHash: Uint8Array | undefined;
  let publicKey: Uint8Array | undefined;
  if (requester === 'secp256k1') {
    publicKey = k1PublicKey;
    messageHash = sha256(signedPayload(args, keys, SLOT, COUNTER));
    data = concat(
      args,
      slotCounter(),
      new Uint8Array(65),
      PERSONAL_SIGN_PREFIX,
    );
  } else {
    const sysvarIndex = keys.length;
    keys.push({
      pubkey: SYSVAR_INSTRUCTIONS_PUBKEY,
      isSigner: false,
      isWritable: false,
    });
    if (requester === 'participantSet') {
      data = concat(args, Uint8Array.of(sysvarIndex));
    } else {
      publicKey = r1PublicKey;
      messageHash = keccak_256(signedPayload(args, keys, SLOT, COUNTER));
      data = concat(
        args,
        slotCounter(),
        Uint8Array.of(sysvarIndex),
        new Uint8Array(4),
      );
      transaction.add(
        new TransactionInstruction({
          programId: SECP256R1_PROGRAM_ID,
          keys: [],
          data: Buffer.from(
            secp256r1InstructionData(
              r1PublicKey,
              new Uint8Array(64),
              messageHash,
            ),
          ),
        }),
      );
    }
  }
  transaction.add(
    new TransactionInstruction({
      programId: SWIG_PROGRAM_ID,
      keys,
      data: Buffer.from(data),
    }),
  );
  const kind =
    route === 'add'
      ? 'PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK'
      : 'PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK';
  // ProtoJSON as the dev portal returns it: enum names, uint64 as strings.
  return {
    transaction: Buffer.from(
      transaction.serialize({
        requireAllSignatures: false,
        verifySignatures: false,
      }),
    ).toString('base64'),
    transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
    wallet: {
      swigConfigAddress: config.toBase58(),
      walletAddress: walletAddress.toBase58(),
    },
    expiresAt: '2026-10-30T12:00:00Z',
    network: 'NETWORK_DEVNET',
    recentBlockhash: '11111111111111111111111111111111',
    kind,
    ...(publicKey && messageHash
      ? {
          signatureRequests: [
            {
              scheme:
                requester === 'secp256k1'
                  ? 'AUTHORITY_SIGNATURE_SCHEME_SECP256K1'
                  : 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1',
              signer: bytesToHex(publicKey),
              messageHash: bytesToHex(messageHash),
              slot: SLOT.toString(),
              counter: COUNTER,
            },
          ],
        }
      : {
          participantSetApprovalPlan: {
            participantSetAddress: participantSet.toBase58(),
            roleId: 4,
            expirationSlot: '345679000',
            nonce: 12,
            transactionDigest: '11'.repeat(32),
            compilationEnvelope: 'sealed-envelope',
            threshold: 1,
            members: [
              {
                memberIndex: 0,
                authority: { ed25519: { publicKey: 'member-ed25519' } },
                challenge: '22'.repeat(32),
              },
            ],
          },
        }),
  };
}

function requesterAuthority(
  requester: Requester,
): AuthorizationLockRequesterAuthority {
  switch (requester) {
    case 'secp256k1':
      return { secp256k1: { publicKey: bytesToHex(k1PublicKey) } };
    case 'secp256r1':
      return { secp256r1: { publicKey: bytesToHex(r1PublicKey) } };
    case 'participantSet':
      return {
        participantSet: { address: participantSet.toBase58(), roleId: 4 },
      };
  }
}

async function prepare(
  requester: Requester,
  route: Route,
  claimer: PublicKey = rentClaimer,
) {
  const calls: { url: string; body: Record<string, unknown> }[] = [];
  const client = new SwigClient({
    apiKey: 'test-key',
    baseUrl: 'https://api.example.test',
    network: 'devnet',
    fetch: (async (
      input: Parameters<typeof fetch>[0],
      init: Parameters<typeof fetch>[1],
    ) => {
      const request = new Request(input, init);
      const body = (await request.json()) as Record<string, unknown>;
      calls.push({ url: request.url, body });
      if (request.url.endsWith('/transaction/wallet/participant-set/compile')) {
        const prepared = body.preparedTransaction as Record<string, unknown>;
        return Response.json({
          transaction: {
            transaction: 'compiled-base64',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            wallet: prepared.wallet,
            network: 'NETWORK_DEVNET',
            recentBlockhash: prepared.recentBlockhash,
            kind: prepared.kind,
          },
          authorizationExpirationSlot: '345679000',
        });
      }
      return Response.json({
        transaction: backendPrepared(requester, route, claimer),
      });
    }) as unknown as typeof fetch,
  });
  const wallet = client.wallets.use(config.toBase58(), {
    requesterAuthority: requesterAuthority(requester),
  });
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
  return { client, prepared, calls };
}

function signedLock(signed: { transaction: string }) {
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
    lock,
    data: new Uint8Array(lock.data),
    precompile: precompile ? new Uint8Array(precompile.data) : undefined,
  };
}

/** A personal_sign wallet: keccak256(EIP-191 prefix || message), low-s. */
const personalSign: Secp256k1SigningFn = async (message) => {
  const prefix = new TextEncoder().encode(
    `\x19Ethereum Signed Message:\n${message.length}`,
  );
  return { signature: signK1(concat(prefix, message)), prefix };
};

/** A raw signer that declares no prefix: keccak256(message), low-s. */
const rawSign: Secp256k1SigningFn = async (message) => ({
  signature: signK1(message),
});

function signK1(message: Uint8Array): Uint8Array {
  const signature = secp256k1.sign(keccak_256(message), k1PrivateKey);
  return concat(
    signature.toCompactRawBytes(),
    Uint8Array.of(27 + signature.recovery),
  );
}

/**
 * state/src/authority/secp256k1.rs: recover the signer from
 * keccak256(prefix || hex(sha256(args || accounts || slot || counter))), using
 * the transaction-level account flags, and reject high-s signatures.
 */
function recoverProgramSigner(
  signed: { transaction: string },
  route: Route,
): Uint8Array {
  const { lock, data } = signedLock(signed);
  const offset = ARGS_SIZE[route];
  const view = new DataView(data.buffer, data.byteOffset);
  const slot = view.getBigUint64(offset, true);
  const counter = view.getUint32(offset + 8, true);
  const signature = data.slice(offset + 12, offset + 77);
  const prefix = data.slice(offset + 77);
  const digestHex = new TextEncoder().encode(
    bytesToHex(
      sha256(signedPayload(data.slice(0, offset), lock.keys, slot, counter)),
    ),
  );
  const parsed = secp256k1.Signature.fromCompact(signature.slice(0, 64));
  if (parsed.hasHighS()) throw new Error('program rejects high-s signatures');
  return parsed
    .addRecoveryBit(signature[64] === 27 ? 0 : 1)
    .recoverPublicKey(keccak_256(concat(prefix, digestHex)))
    .toRawBytes(true);
}

describe.each(['add', 'remove'] as const)('authorization lock %s', (route) => {
  const offset = ARGS_SIZE[route];

  test('writes a secp256k1 signature over the zeroed bytes and keeps the server prefix', async () => {
    const { prepared, calls } = await prepare('secp256k1', route);
    expect(calls[0]?.body.requesterAuthority).toEqual({
      secp256k1: { publicKey: bytesToHex(k1PublicKey) },
    });
    expect(prepared.kind).toBe(`${route}-authorization-lock`);
    expect(prepared.signatureRequests).toMatchObject([
      { scheme: 'secp256k1', slot: Number(SLOT), counter: COUNTER },
    ]);
    const before = Buffer.from(prepared.transaction, 'base64');
    const original = new Uint8Array(
      Transaction.from(before).instructions[0].data,
    );
    expect(original.slice(offset + 77)).toEqual(PERSONAL_SIGN_PREFIX);

    const signed = await signPreparedSwigTransaction(prepared, {
      secp256k1: personalSign,
    });

    const { data } = signedLock(signed);
    expect(data.length).toBe(original.length);
    expect(data.slice(0, offset + 12)).toEqual(original.slice(0, offset + 12));
    expect(data.slice(offset + 12, offset + 77)).not.toEqual(
      new Uint8Array(65),
    );
    expect([27, 28]).toContain(data[offset + 76]);
    // The prefix is written exactly once, matching the server's bytes.
    expect(data.slice(offset + 77)).toEqual(PERSONAL_SIGN_PREFIX);
  });

  test.each([
    ['personal_sign', personalSign],
    ['raw', rawSign],
  ] as const)(
    'a %s secp256k1 signature recovers the requester under the program algorithm',
    async (_, signer) => {
      const { prepared } = await prepare('secp256k1', route);
      let signedMessage: Uint8Array | undefined;
      const signed = await signPreparedSwigTransaction(prepared, {
        secp256k1: {
          [bytesToHex(k1PublicKey)]: async (message) => {
            signedMessage = message;
            return signer(message);
          },
        },
      });

      expect(signedMessage).toEqual(
        new TextEncoder().encode(prepared.signatureRequests[0]?.messageHash),
      );
      expect(recoverProgramSigner(signed, route)).toEqual(k1PublicKey);
    },
  );

  test.if(route === 'remove')(
    'hashes a rent claimer that is also the fee payer as a signer',
    async () => {
      const { prepared } = await prepare('secp256k1', route, feePayer);
      const signed = await signPreparedSwigTransaction(prepared, {
        secp256k1: personalSign,
      });
      expect(recoverProgramSigner(signed, route)).toEqual(k1PublicKey);
    },
  );

  test('writes a secp256r1 signature into the preceding precompile', async () => {
    const { prepared, calls } = await prepare('secp256r1', route);
    expect(calls[0]?.body.requesterAuthority).toEqual({
      secp256r1: { publicKey: bytesToHex(r1PublicKey) },
    });
    const messageHash = hexToBytes(prepared.signatureRequests[0]!.messageHash);
    const signature = Uint8Array.from({ length: 64 }, (_, index) => index);

    const signed = await signPreparedSwigTransaction(prepared, {
      secp256r1: async (challenge) => {
        expect(challenge).toEqual(messageHash);
        return { signature };
      },
    });

    const { data, precompile } = signedLock(signed);
    if (!precompile) throw new Error('precompile missing');
    const view = new DataView(precompile.buffer, precompile.byteOffset);
    const signatureOffset = view.getUint16(2, true);
    const publicKeyOffset = view.getUint16(6, true);
    expect(precompile.slice(signatureOffset, signatureOffset + 64)).toEqual(
      signature,
    );
    expect(precompile.slice(publicKeyOffset, publicKeyOffset + 33)).toEqual(
      r1PublicKey,
    );
    expect(precompile.slice(view.getUint16(10, true))).toEqual(messageHash);
    expect(data.length).toBe(offset + 17);
  });

  test('appends a secp256r1 passkey prefix to the lock authority payload', async () => {
    const { prepared } = await prepare('secp256r1', route);
    const message = Uint8Array.of(9, 8, 7, 6);
    const prefix = Uint8Array.of(1, 3, 5, 7);

    const signed = await signPreparedSwigTransaction(prepared, {
      secp256r1: async () => ({
        signature: new Uint8Array(64).fill(5),
        message,
        prefix,
      }),
    });

    const { data, precompile } = signedLock(signed);
    if (!precompile) throw new Error('precompile missing');
    const view = new DataView(precompile.buffer, precompile.byteOffset);
    expect(precompile.slice(view.getUint16(10, true))).toEqual(message);
    expect(data.slice(0, offset)).toEqual(lockArgs(route));
    expect(data.slice(offset + 17)).toEqual(prefix);
  });

  test('compiles ParticipantSet approvals for a ProgramExec lock requester', async () => {
    const { client, prepared, calls } = await prepare('participantSet', route);
    expect(calls[0]?.body.requesterAuthority).toEqual({
      participantSet: {
        participantSetAddress: participantSet.toBase58(),
        roleId: 4,
      },
    });
    expect(prepared.signatureRequests).toEqual([]);
    const plan = prepared.participantSetApprovalPlan;
    if (!plan) throw new Error('approval plan missing');
    expect(plan).toMatchObject({
      type: 'participantSet',
      participantSetAddress: participantSet.toBase58(),
      roleId: 4,
      expirationSlot: '345679000',
      nonce: 12,
      threshold: 1,
    });

    const approvals = await Promise.all(
      plan.members.map((member) =>
        signParticipantSetApproval(
          member,
          createParticipantEd25519Signer({
            publicKey: 'member-ed25519',
            signMessage: () => new Uint8Array(64).fill(member.memberIndex + 1),
          }),
        ),
      ),
    );
    const compiled = await client.transactions.compileParticipantSetApprovals({
      preparedTransaction: prepared,
      approvals,
    });

    expect(calls[1]?.url).toBe(
      'https://api.example.test/transaction/wallet/participant-set/compile',
    );
    expect(calls[1]?.body).toMatchObject({
      preparedTransaction: {
        transaction: prepared.transaction,
        transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
        wallet: {
          swigConfigAddress: config.toBase58(),
          walletAddress: walletAddress.toBase58(),
        },
        network: 'NETWORK_DEVNET',
        kind:
          route === 'add'
            ? 'PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK'
            : 'PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK',
        signatureRequests: [],
        participantSetApprovalPlan: {
          participantSetAddress: participantSet.toBase58(),
          roleId: 4,
          expirationSlot: '345679000',
          nonce: 12,
          transactionDigest: '11'.repeat(32),
          compilationEnvelope: 'sealed-envelope',
          threshold: 1,
          members: [
            {
              memberIndex: 0,
              authority: { ed25519: { publicKey: 'member-ed25519' } },
              challenge: '22'.repeat(32),
            },
          ],
        },
      },
      approvals: [
        {
          memberIndex: 0,
          ed25519: {
            signature: Buffer.from(new Uint8Array(64).fill(1)).toString(
              'base64',
            ),
          },
        },
      ],
    });
    expect(compiled).toMatchObject({
      transaction: {
        transaction: 'compiled-base64',
        kind: `${route}-authorization-lock`,
        signatureRequests: [],
      },
      authorizationExpirationSlot: '345679000',
    });
  });
});

describe('authorization lock requesters', () => {
  test('rejects programExecProof before sending', async () => {
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
    const wallet = client.wallets.use('config', {
      requesterAuthority: { programExecProof: { roleId: 1, zkProof: 'proof' } },
    });
    const message =
      'Authorization lock requesterAuthority does not support programExecProof';
    await expect(
      wallet.addAuthorizationLock({
        feePayer: 'payer',
        amount: 1,
        durationSlots: 10,
      }),
    ).rejects.toThrow(message);
    await expect(
      wallet.removeAuthorizationLock({ feePayer: 'payer', lockId: 0 }),
    ).rejects.toThrow(message);
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

function concat(...parts: Uint8Array[]): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(
    parts.reduce((size, part) => size + part.length, 0),
  );
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

function bytesToHex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function hexToBytes(hex: string): Uint8Array<ArrayBuffer> {
  return new Uint8Array(Buffer.from(hex, 'hex'));
}
