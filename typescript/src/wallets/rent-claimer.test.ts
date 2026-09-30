import { Connection, PublicKey, type AccountInfo } from '@solana/web3.js';
import { SWIG_PROGRAM_ADDRESS_STRING } from '@swig-wallet/lib';
import { describe, expect, spyOn, test } from 'bun:test';

import { fetchRentClaimer } from '../index.js';

const program = new PublicKey(SWIG_PROGRAM_ADDRESS_STRING);
const claimer = new PublicKey(new Uint8Array(32).fill(7));
// Protocol fixtures are explicit bytes, independent of the reader's decoder.
const rentEntry = Buffer.concat([
  Buffer.from('0101200000000000', 'hex'),
  claimer.toBuffer(),
]);
const countEntry = Buffer.from('02010800000000000500000000000000', 'hex');

function fixture(tail: Buffer = Buffer.alloc(0)) {
  const id = Buffer.alloc(32, 6);
  const [config, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from('swig'), id],
    program,
  );
  const [, walletBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('swig-wallet-address'), config.toBuffer()],
    program,
  );
  const data = Buffer.alloc(48 + 56 + 88 + tail.length);
  data[0] = 1;
  data[1] = bump;
  id.copy(data, 2);
  data.writeUInt16LE(2, 34);
  data.writeUInt32LE(2, 36);
  data[40] = walletBump;
  data.writeUInt32LE(9, 44);
  // One Ed25519 role and one session role, with different authority lengths.
  for (const [offset, type, length, end, roleId] of [
    [48, 1, 32, 56, 0],
    [104, 2, 64, 144, 1],
  ]) {
    data.writeUInt16LE(type, offset);
    data.writeUInt16LE(length, offset + 2);
    data.writeUInt16LE(1, offset + 4);
    data.writeUInt32LE(roleId, offset + 8);
    data.writeUInt32LE(end, offset + 12);
    data.fill(1, offset + 16, offset + 16 + length);
    data.writeUInt16LE(7, offset + 16 + length); // All, no value bytes.
    data.writeUInt32LE(8, offset + 20 + length);
  }
  tail.copy(data, 192);
  const account: AccountInfo<Buffer> = {
    data,
    owner: program,
    executable: false,
    lamports: 10_000_000,
  };
  const connection = new Connection('http://localhost:8899');
  const read = spyOn(connection, 'getAccountInfo').mockResolvedValue(account);
  return { config, data, account, connection, read };
}

describe('fetchRentClaimer', () => {
  test('returns null for an absent claimer and defaults to finalized', async () => {
    const f = fixture();
    expect(await fetchRentClaimer(f.connection, f.config)).toBeNull();
    expect(f.read).toHaveBeenCalledWith(f.config, 'finalized');
  });

  test('reads a claimer through the public entrypoint and honors commitment', async () => {
    const f = fixture(rentEntry);
    expect(
      (await fetchRentClaimer(f.connection, f.config, 'confirmed'))?.equals(
        claimer,
      ),
    ).toBe(true);
    expect(f.read).toHaveBeenCalledWith(f.config, 'confirmed');
  });

  test('supports either tail order and an active-count-only tail', async () => {
    for (const tail of [
      Buffer.concat([countEntry, rentEntry]),
      Buffer.concat([rentEntry, countEntry]),
    ]) {
      const f = fixture(tail);
      expect(
        (await fetchRentClaimer(f.connection, f.config))?.equals(claimer),
      ).toBe(true);
    }
    const f = fixture(countEntry);
    expect(await fetchRentClaimer(f.connection, f.config)).toBeNull();
  });

  test('missing, wrong-owner, executable and RPC failures never mean unset', async () => {
    const missing = fixture();
    missing.read.mockResolvedValue(null);
    await expect(
      fetchRentClaimer(missing.connection, missing.config),
    ).rejects.toThrow('Missing or invalid');
    const wrongOwner = fixture();
    wrongOwner.account.owner = claimer;
    await expect(
      fetchRentClaimer(wrongOwner.connection, wrongOwner.config),
    ).rejects.toThrow('Missing or invalid');
    const executable = fixture();
    executable.account.executable = true;
    await expect(
      fetchRentClaimer(executable.connection, executable.config),
    ).rejects.toThrow('Missing or invalid');
    const rpc = fixture();
    const failure = new Error('RPC unavailable');
    rpc.read.mockRejectedValue(failure);
    await expect(fetchRentClaimer(rpc.connection, rpc.config)).rejects.toBe(
      failure,
    );
  });

  test('rejects short, closed, legacy, wrong-PDA and noncanonical headers', async () => {
    const short = fixture();
    short.account.data = Buffer.alloc(47);
    await expect(
      fetchRentClaimer(short.connection, short.config),
    ).rejects.toThrow('header');
    for (const [offset, value] of [
      [0, 0],
      [1, 0],
      [2, 0],
      [40, 0],
      [41, 1],
    ]) {
      const f = fixture();
      f.data[offset] = value;
      await expect(fetchRentClaimer(f.connection, f.config)).rejects.toThrow();
    }
    const legacy = fixture();
    legacy.data.writeBigUInt64LE(2_000_000n, 40);
    await expect(
      fetchRentClaimer(legacy.connection, legacy.config),
    ).rejects.toThrow('canonical V2');
    const other = fixture();
    await expect(fetchRentClaimer(other.connection, claimer)).rejects.toThrow(
      'canonical V2',
    );
  });

  test('rejects truncated and invalid role or action boundaries', async () => {
    for (const boundary of [0, 15, 47, 10_000]) {
      const f = fixture();
      f.data.writeUInt32LE(boundary, 60);
      await expect(fetchRentClaimer(f.connection, f.config)).rejects.toThrow(
        'role boundary',
      );
    }
    const truncated = fixture();
    truncated.data.writeUInt16LE(3, 34);
    await expect(
      fetchRentClaimer(truncated.connection, truncated.config),
    ).rejects.toThrow('Truncated Swig role');
    const action = fixture();
    action.data.writeUInt32LE(9, 100);
    await expect(
      fetchRentClaimer(action.connection, action.config),
    ).rejects.toThrow('action boundary');
  });

  test('validates the entire tail, even after finding a claimer', async () => {
    const malformed = [
      Buffer.alloc(1),
      Buffer.alloc(8),
      rentEntry.subarray(0, 39),
      Buffer.concat([rentEntry, rentEntry]),
      Buffer.concat([countEntry, countEntry]),
      Buffer.from('0301000000000000', 'hex'),
      Buffer.from('0102200000000000' + '07'.repeat(32), 'hex'),
      Buffer.from('0101200001000000' + '07'.repeat(32), 'hex'),
      Buffer.from('02010800000000000500000001000000', 'hex'),
    ];
    for (const tail of malformed) {
      for (const prefix of [Buffer.alloc(0), rentEntry]) {
        const f = fixture(Buffer.concat([prefix, tail]));
        await expect(
          fetchRentClaimer(f.connection, f.config),
        ).rejects.toThrow();
      }
    }
  });
});
