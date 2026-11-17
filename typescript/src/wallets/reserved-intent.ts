import {
  ComputeBudgetProgram,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import {
  Actions,
  createEd25519AuthorityInfo,
  Ed25519Authority,
  SWIG_PROGRAM_ADDRESS_STRING,
} from '@swig-wallet/lib';
import { createHash } from 'node:crypto';
import type { ReservedWalletSetup } from './onboarding.js';

const programId = new PublicKey(SWIG_PROGRAM_ADDRESS_STRING);

// ReservationV1's canonical Ed25519 package is version, program, salt, authority type and owner.
export function reservedOwnerAddresses(owner: string, salt: Uint8Array) {
  if (salt.length !== 32)
    throw new Error('Expected a 32-byte reservation salt');
  const bytes = Buffer.concat([
    Buffer.from([1]),
    programId.toBuffer(),
    salt,
    Buffer.from([1, 0]),
    new PublicKey(owner).toBuffer(),
  ]);
  const domain = Buffer.from('rsv');
  const commitment = createHash('sha256').update(domain).update(bytes).digest();
  const [config] = PublicKey.findProgramAddressSync(
    [domain, commitment],
    programId,
  );
  const [funds] = PublicKey.findProgramAddressSync(
    [Buffer.from('swig-wallet-address'), config.toBuffer()],
    programId,
  );
  return { bytes, config, funds };
}

export async function verifyReservedIntent(
  transaction: VersionedTransaction,
  setup: ReservedWalletSetup,
  payer: string,
  operation: 'create' | 'agent',
): Promise<void> {
  const { message } = transaction;
  if (message.version !== 'legacy')
    throw new Error('Unexpected reserved wallet transaction format');
  const payerKey = new PublicKey(payer);
  const { bytes, config, funds } = reservedOwnerAddresses(
    setup.owner,
    Buffer.from(setup.salt, 'hex'),
  );
  if (
    config.toBase58() !== setup.swigConfigAddress ||
    funds.toBase58() !== setup.walletAddress
  )
    throw new Error('Reservation address mismatch');
  let instructions: TransactionInstruction[];
  if (operation === 'create') {
    instructions = [
      ComputeBudgetProgram.setComputeUnitLimit({ units: 600000 }),
      new TransactionInstruction({
        programId,
        keys: [
          { pubkey: config, isWritable: true, isSigner: false },
          { pubkey: payerKey, isWritable: true, isSigner: true },
          { pubkey: funds, isWritable: true, isSigner: false },
          {
            pubkey: SystemProgram.programId,
            isWritable: false,
            isSigner: false,
          },
        ],
        data: Buffer.concat([Buffer.from([24, 0]), bytes]),
      }),
    ];
  } else {
    const actions = Actions.set();
    for (const limit of setup.permissions) {
      if (limit.type === 'programAll') {
        actions.programAll();
        continue;
      }
      const amounts = {
        recurringAmount: BigInt(limit.recurringAmount),
        window: BigInt(limit.window),
      };
      if (limit.type === 'solRecurringLimit')
        actions.solRecurringLimit(amounts);
      else actions.tokenRecurringLimit({ ...amounts, mint: limit.mint });
    }
    const ix = await new Ed25519Authority(
      new PublicKey(setup.owner).toBytes(),
    ).addAuthority({
      swigAddress: config.toBytes(),
      payer: payerKey.toBytes(),
      actingRoleId: 0,
      actions: actions.get(),
      newAuthorityInfo: createEd25519AuthorityInfo(setup.agent),
    });
    instructions = ix.getWeb3Instructions().map(
      (raw) =>
        new TransactionInstruction({
          programId: new PublicKey(raw.programId.toBytes()),
          keys: raw.keys.map((meta) => ({
            ...meta,
            pubkey: new PublicKey(meta.pubkey.toBytes()),
          })),
          data: Buffer.from(raw.data),
        }),
    );
  }
  // Rust and JS may order static keys differently. Recompile the decoded message
  // with the same compiler to compare program, accounts, privileges, data and order.
  const expected = new TransactionMessage({
    payerKey,
    recentBlockhash: message.recentBlockhash,
    instructions,
  }).compileToLegacyMessage();
  const actual = TransactionMessage.decompile(message).compileToLegacyMessage();
  if (
    message.staticAccountKeys.length !== expected.staticAccountKeys.length ||
    !Buffer.from(actual.serialize()).equals(Buffer.from(expected.serialize()))
  )
    throw new Error(
      'Transaction instructions differ from the approved wallet operation',
    );
}
