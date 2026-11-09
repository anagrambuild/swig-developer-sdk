import { PublicKey } from '@solana/web3.js';
import type { Network } from '../types/common.js';
import type { NetworkWire } from '../types/transaction.js';
import type { DirectWalletAuthority } from '../types/wallet-actions.js';
import type { WalletAddressInfo } from '../types/wallet.js';
import { normalizeNetwork, toProtoNetwork } from './normalizers.js';

export type ReservationAddressOptions =
  | { accountIndex: number; salt?: never }
  | { salt: Uint8Array; accountIndex?: never };

export interface ReserveWalletAddressArgs {
  rootAuthority: DirectWalletAuthority;
  network?: Network;
  /** Defaults to account index zero. Reuse the same options when creating. */
  addressOptions?: ReservationAddressOptions;
}

export interface CreateReservedWalletArgs extends ReserveWalletAddressArgs {
  feePayer: string;
}

export interface WalletReservation {
  wallet: WalletAddressInfo & { network: Network };
  network: Network;
}

interface ReservationRequest {
  network: NetworkWire;
  rootAuthority: DirectWalletAuthority;
  addressOptions?: { accountIndex: number } | { salt: string };
}

export function reservationRequest(
  args: ReserveWalletAddressArgs,
  defaultNetwork?: Network,
): ReservationRequest {
  const network = args.network ?? defaultNetwork;
  if (!network) throw new Error('network is required');
  const authority = args.rootAuthority;
  if (!authority || typeof authority !== 'object' || Array.isArray(authority)) {
    throw new Error('rootAuthority must be a direct signing-key authority');
  }
  const keys = Object.keys(authority);
  if (
    keys.length !== 1 ||
    !keys.some((key) => ['ed25519', 'secp256k1', 'secp256r1'].includes(key))
  ) {
    throw new Error('rootAuthority must be a direct signing-key authority');
  }
  const key =
    'ed25519' in authority
      ? authority.ed25519
      : 'secp256k1' in authority
        ? authority.secp256k1
        : authority.secp256r1;
  if (!key || typeof key.publicKey !== 'string' || !key.publicKey) {
    throw new Error('rootAuthority publicKey is required');
  }
  const request: ReservationRequest = {
    network: toProtoNetwork(network),
    rootAuthority: authority,
  };
  const options = args.addressOptions;
  if (options !== undefined) {
    if (!options || typeof options !== 'object' || Array.isArray(options)) {
      throw new Error('Invalid reservation address options');
    }
    if ('salt' in options) {
      if (
        'accountIndex' in options ||
        !(options.salt instanceof Uint8Array) ||
        options.salt.length !== 32
      ) {
        throw new Error(
          'Reservation requires either a u32 accountIndex or a 32-byte salt',
        );
      }
      request.addressOptions = {
        salt: btoa(String.fromCharCode(...options.salt)),
      };
    } else {
      if (
        !Number.isInteger(options.accountIndex) ||
        options.accountIndex < 0 ||
        options.accountIndex > 0xffffffff
      ) {
        throw new Error('Reservation accountIndex must be a u32');
      }
      request.addressOptions = { accountIndex: options.accountIndex };
    }
  }
  return request;
}

export function normalizeWalletReservation(
  response: unknown,
  network: NetworkWire,
): WalletReservation {
  if (
    !response ||
    typeof response !== 'object' ||
    !('wallet' in response) ||
    !('network' in response)
  ) {
    throw new Error('Invalid wallet reservation response');
  }
  const wallet = response.wallet;
  const resolvedNetwork = normalizeNetwork(network);
  if (
    !resolvedNetwork ||
    (response.network !== network && response.network !== resolvedNetwork)
  ) {
    throw new Error('Wallet reservation response has mismatched network');
  }
  if (
    !wallet ||
    typeof wallet !== 'object' ||
    !('swigConfigAddress' in wallet) ||
    !('walletAddress' in wallet) ||
    typeof wallet.swigConfigAddress !== 'string' ||
    typeof wallet.walletAddress !== 'string'
  ) {
    throw new Error('Wallet reservation response is missing addresses');
  }
  const swigConfigAddress = new PublicKey(wallet.swigConfigAddress).toBase58();
  const walletAddress = new PublicKey(wallet.walletAddress).toBase58();
  return {
    wallet: { swigConfigAddress, walletAddress, network: resolvedNetwork },
    network: resolvedNetwork,
  };
}
