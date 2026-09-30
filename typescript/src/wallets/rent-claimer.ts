import { PublicKey, type Commitment, type Connection } from '@solana/web3.js';
import {
  getPositionDecoder,
  getSwigCodec,
  SWIG_PROGRAM_ADDRESS_STRING,
} from '@swig-wallet/lib';

const SWIG_PROGRAM_ID = new PublicKey(SWIG_PROGRAM_ADDRESS_STRING);
const SWIG_HEADER_LENGTH = 48;
const ROLE_HEADER_LENGTH = 16;
const TAIL_HEADER_LENGTH = 8;

/**
 * Read a V2 Swig config's immutable rent recipient directly from Solana RPC.
 * Returns null only when the supported account layout has no rent-claimer
 * record. Missing, closed, malformed or unsupported accounts and RPC failures
 * throw. The header check follows the current V2 format; it is not a historical
 * account-generation proof because V1 and V2 share a header discriminator.
 * The connection selects the network; swigConfig is the config PDA, not the
 * wallet-address PDA. The default commitment is finalized.
 *
 * @example
 * const claimer = await fetchRentClaimer(connection, swigConfig);
 * console.log(claimer?.toBase58() ?? 'unset');
 */
export async function fetchRentClaimer(
  connection: Connection,
  swigConfig: PublicKey,
  commitment: Commitment = 'finalized',
): Promise<PublicKey | null> {
  const account = await connection.getAccountInfo(swigConfig, commitment);
  if (
    !account ||
    account.executable ||
    !account.owner.equals(SWIG_PROGRAM_ID)
  ) {
    throw new Error('Missing or invalid Swig config account');
  }
  const data = account.data;
  if (data.length < SWIG_HEADER_LENGTH || data[0] !== 1) {
    throw new Error('Invalid Swig config header');
  }

  const [canonicalConfig, configBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('swig'), data.subarray(2, 34)],
    SWIG_PROGRAM_ID,
  );
  const [, walletBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('swig-wallet-address'), swigConfig.toBuffer()],
    SWIG_PROGRAM_ID,
  );
  // Bytes 44..48 hold the V2 subaccount counter, not part of the wallet bump.
  if (
    !canonicalConfig.equals(swigConfig) ||
    data[1] !== configBump ||
    data.readUInt32LE(40) !== walletBump ||
    walletBump === 0
  ) {
    throw new Error('Expected a canonical V2 Swig config');
  }

  const swig = getSwigCodec().decode(data);
  const body = Buffer.from(swig.roles_buffer);
  let offset = 0;
  for (let i = 0; i < swig.roles; i++) {
    if (offset + ROLE_HEADER_LENGTH > body.length) {
      throw new Error('Truncated Swig role');
    }
    const role = getPositionDecoder().decode(
      body.subarray(offset, offset + ROLE_HEADER_LENGTH),
    );
    const actionsStart = offset + ROLE_HEADER_LENGTH + role.authorityLen;
    if (role.boundary < actionsStart || role.boundary > body.length) {
      throw new Error('Invalid Swig role boundary');
    }
    let actionOffset = actionsStart;
    for (let j = 0; j < role.numActions; j++) {
      if (actionOffset + 8 > role.boundary) {
        throw new Error('Truncated Swig action');
      }
      const next = actionsStart + body.readUInt32LE(actionOffset + 4);
      if (
        next !== actionOffset + 8 + body.readUInt16LE(actionOffset + 2) ||
        next > role.boundary
      ) {
        throw new Error('Invalid Swig action boundary');
      }
      actionOffset = next;
    }
    if (actionOffset !== role.boundary) {
      throw new Error('Unexpected trailing Swig role data');
    }
    offset = role.boundary;
  }

  // Tail ABI: swig-wallet 77a41ef, state/src/tail/{mod,rent_claimer}.rs.
  // Validate every entry, including entries after the claimer, before returning.
  let rentClaimer: PublicKey | null = null;
  const seen = new Set<number>();
  while (offset < body.length) {
    if (offset + TAIL_HEADER_LENGTH > body.length) {
      throw new Error('Truncated Swig tail');
    }
    const kind = body.readUInt8(offset);
    const version = body.readUInt8(offset + 1);
    const length = body.readUInt16LE(offset + 2);
    const end = offset + TAIL_HEADER_LENGTH + length;
    // Current kinds: 1 = rent claimer, 2 = active subaccount count.
    const expectedLength = kind === 1 ? 32 : kind === 2 ? 8 : -1;
    if (
      version !== 1 ||
      seen.has(kind) ||
      length !== expectedLength ||
      end > body.length ||
      body.readUInt32LE(offset + 4) !== 0
    ) {
      throw new Error('Unsupported or malformed Swig tail');
    }
    seen.add(kind);
    if (kind === 1) {
      rentClaimer = new PublicKey(
        body.subarray(offset + TAIL_HEADER_LENGTH, end),
      );
    }
    if (kind === 2 && body.readUInt32LE(offset + 12) !== 0) {
      throw new Error('Invalid Swig active-subaccount reserved bytes');
    }
    offset = end;
  }
  return rentClaimer;
}
