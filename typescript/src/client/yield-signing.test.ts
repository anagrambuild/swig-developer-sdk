import { p256 } from '@noble/curves/p256';
import { sha256 } from '@noble/hashes/sha256';
import {
  PublicKey,
  Transaction,
  TransactionInstruction,
  VersionedTransaction,
} from '@solana/web3.js';
import {
  ActionsBuilder,
  createEd25519AuthorityInfo,
  Secp256r1Authority,
} from '@swig-wallet/lib';
import { expect, test } from 'bun:test';
import fixture from '../../../test-fixtures/yield-role.json';
import { SwigClient } from '../server/typescript/index.js';
import type { PreparedTransaction } from '../types/index.js';
import { signPreparedSwigTransaction } from './swig-signing.js';

const signingResult = {
  signature: Buffer.from(fixture.signature, 'hex'),
  message: Buffer.from(fixture.message, 'hex'),
  prefix: Buffer.from(fixture.prefix, 'hex'),
};

function prepared(transaction: string): PreparedTransaction {
  return {
    transaction,
    transactionEncoding: 'base64',
    network: 'devnet',
    signatureRequests: [
      {
        scheme: 'secp256r1',
        signer: fixture.publicKey,
        messageHash: fixture.challenge,
        slot: 12,
        counter: 34,
      },
    ],
  };
}

test('AddAuthority fixture matches the production Swig builder', async () => {
  const authorityData = new Uint8Array(40);
  authorityData.set(Buffer.from(fixture.publicKey, 'hex'));
  const authority = new Secp256r1Authority(authorityData);
  const feePayer = new PublicKey(
    '2bDoQTvcRuAL8bA7igx6cjSZHH3wJqVg1vsYrUjeTWyP',
  );
  const context = await authority.addAuthority({
    swigAddress: new PublicKey(new Uint8Array(32).fill(7)),
    payer: feePayer,
    actingRoleId: 3,
    actions: ActionsBuilder.new()
      .stakeAll()
      .solLimit({ amount: 1000000000n })
      .get(),
    newAuthorityInfo: createEd25519AuthorityInfo(
      new PublicKey(new Uint8Array(32).fill(9)),
    ),
    options: {
      currentSlot: 12n,
      odometer: 34,
      signingFn: async (challenge) => {
        expect(Buffer.from(challenge).toString('hex')).toBe(fixture.challenge);
        return { signature: new Uint8Array(64) };
      },
    },
  });
  const instructions = context.getWeb3Instructions().map(
    (ix) =>
      new TransactionInstruction({
        programId: new PublicKey(ix.programId.toBase58()),
        keys: ix.keys.map((meta) => ({
          ...meta,
          pubkey: new PublicKey(meta.pubkey.toBase58()),
        })),
        data: Buffer.from(ix.data),
      }),
  );
  const tx = new Transaction({
    feePayer,
    recentBlockhash: '11111111111111111111111111111111',
  }).add(...instructions);
  expect(
    tx
      .serialize({ requireAllSignatures: false, verifySignatures: false })
      .toString('base64'),
  ).toBe(fixture.legacy.unsigned);
});

test.each(['legacy', 'v0'] as const)(
  'signs a %s yield role exactly like the protocol builder',
  async (version) => {
    const input = prepared(fixture[version].unsigned);
    const swig = new SwigClient({
      apiKey: 'test',
      network: 'devnet',
      fetch: Object.assign(
        async () =>
          Response.json({
            create: {
              transaction: {
                ...input,
                kind: 'PREPARED_TRANSACTION_KIND_ADD_AUTHORITY',
              },
              yieldAuthority: { ed25519: { publicKey: 'new-authority' } },
              actions: [{ stakeAll: {} }],
            },
          }),
        { preconnect: () => undefined },
      ),
    });
    const role = await swig.yield.prepareRole({
      swigConfigAddress: 'config',
      feePayer: 'payer',
      yieldAuthority: { ed25519: { publicKey: 'new-authority' } },
      roleManagerAuthority: { secp256r1: { publicKey: fixture.publicKey } },
      maxDeposit: 1,
      validatorVoteAddress: 'vote',
    });
    if (role.type !== 'create') throw new Error('Expected role creation');
    const signed = await signPreparedSwigTransaction(role.transaction, {
      secp256r1: async (challenge) => {
        expect(Buffer.from(challenge).toString('hex')).toBe(fixture.challenge);
        return signingResult;
      },
    });
    expect(signed.transaction).toBe(fixture[version].signed);
    const tx = VersionedTransaction.deserialize(
      Buffer.from(signed.transaction, 'base64'),
    );
    const secp = Buffer.from(tx.message.compiledInstructions[0].data);
    const signatureOffset = secp.readUInt16LE(2);
    const messageOffset = secp.readUInt16LE(10);
    const messageSize = secp.readUInt16LE(12);
    expect(
      p256.verify(
        secp.subarray(signatureOffset, signatureOffset + 64),
        sha256(secp.subarray(messageOffset, messageOffset + messageSize)),
        Buffer.from(fixture.publicKey, 'hex'),
        { lowS: true },
      ),
    ).toBe(true);
    // Replacing a previous prefix must not append it twice or alter signed role data.
    const again = prepared(fixture[version].signed);
    if (version === 'v0') {
      const existing = VersionedTransaction.deserialize(
        Buffer.from(again.transaction, 'base64'),
      );
      const original = VersionedTransaction.deserialize(
        Buffer.from(input.transaction, 'base64'),
      );
      existing.message.compiledInstructions[0].data =
        original.message.compiledInstructions[0].data;
      again.transaction = Buffer.from(existing.serialize()).toString('base64');
    } else {
      const existing = Transaction.from(
        Buffer.from(again.transaction, 'base64'),
      );
      const original = Transaction.from(
        Buffer.from(input.transaction, 'base64'),
      );
      existing.instructions[0].data = original.instructions[0].data;
      again.transaction = existing
        .serialize({ requireAllSignatures: false, verifySignatures: false })
        .toString('base64');
    }
    expect(
      (
        await signPreparedSwigTransaction(again, {
          secp256r1: async () => signingResult,
        })
      ).transaction,
    ).toBe(signed.transaction);
  },
);

test.each([
  'header',
  'authority-length',
  'actions-length',
  'payload',
  'unsupported',
  'wrong-program',
] as const)(
  'rejects malformed AddAuthority %s without changing input',
  async (fault) => {
    const tx = VersionedTransaction.deserialize(
      Buffer.from(fixture.v0.unsigned, 'base64'),
    );
    const instruction = tx.message.compiledInstructions[1];
    const data = Buffer.from(instruction.data);
    if (fault === 'header') instruction.data = data.subarray(0, 15);
    if (fault === 'authority-length') {
      data.writeUInt16LE(65535, 2);
      instruction.data = data;
    }
    if (fault === 'actions-length') {
      data.writeUInt16LE(65535, 4);
      instruction.data = data;
    }
    if (fault === 'payload')
      instruction.data = data.subarray(0, data.length - 1);
    if (fault === 'unsupported') {
      data.writeUInt16LE(2, 0);
      instruction.data = data;
    }
    if (fault === 'wrong-program') instruction.programIdIndex = 0;
    const input = prepared(Buffer.from(tx.serialize()).toString('base64'));
    const before = input.transaction;
    const error =
      fault === 'header'
        ? 'Truncated AddAuthorityV1 header'
        : fault === 'unsupported'
          ? 'Unsupported Swig instruction'
          : fault === 'wrong-program'
            ? 'Swig instruction following'
            : 'missing secp256r1 authority payload';
    await expect(
      signPreparedSwigTransaction(input, {
        secp256r1: async () => signingResult,
      }),
    ).rejects.toThrow(error);
    expect(input.transaction).toBe(before);
  },
);
