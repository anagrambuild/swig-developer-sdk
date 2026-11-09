import {
  createAssociatedTokenAccountIdempotentInstruction,
  createMint,
  getAccount,
  getAssociatedTokenAddress,
  getMint,
  mintTo,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
  VersionedTransaction,
} from '@solana/web3.js';
import {
  DEFAULT_BACKEND_URL,
  SwigClient,
  type PreparedTransaction,
} from '@swig-wallet/developer-sdk';
import { SWIG_PROGRAM_ADDRESS_STRING } from '@swig-wallet/lib';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';

const X402_DEVNET = 'solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1' as const;

const DEFAULT_FACILITATOR_PORT = 4022;
const DEFAULT_RESOURCE_SERVER_PORT = 4021;
const MINIMUM_FACILITATOR_BALANCE = LAMPORTS_PER_SOL;
const MAX_U64 = 18_446_744_073_709_551_615n;
const SWIG_PROGRAM_ID = new PublicKey(SWIG_PROGRAM_ADDRESS_STRING);

const STATE_DIRECTORY = new URL('./.local/', import.meta.url);
const STATE_FILE = new URL('./.local/state.json', import.meta.url);

export class X402SetupError extends Error {}

interface SetupState {
  facilitatorSecretKey?: number[];
  developerSecretKey?: number[];
  resourceProviderSecretKey?: number[];
  mintSecretKey?: number[];
  swigConfigAddress?: string;
  swigWalletAddress?: string;
  developerPublicKey?: string;
}

export interface ReadyX402Fixture {
  rpcUrl: string;
  backendUrl: string;
  apiKey: string;
  network: 'devnet';
  x402Network: typeof X402_DEVNET;
  facilitator: Keypair;
  developer: Keypair;
  resourceProvider: Keypair;
  mint: PublicKey;
  tokenProgram: PublicKey;
  swigConfigAddress: string;
  swigWalletAddress: string;
  paymentAmount: bigint;
  facilitatorPort: number;
  resourceServerPort: number;
}

export async function ensureX402Fixture(): Promise<ReadyX402Fixture> {
  if (
    process.env.SWIG_NETWORK !== undefined &&
    process.env.SWIG_NETWORK !== 'devnet'
  ) {
    throw new X402SetupError(
      'The x402 fixture supports SWIG_NETWORK=devnet only',
    );
  }
  const configuredRpc = aliasedEnvironment('SWIG_RPC_URL', 'SOLANA_RPC_URL');
  if (configuredRpc === undefined) {
    throw new X402SetupError('Set SWIG_RPC_URL (or SOLANA_RPC_URL)');
  }
  const rpcUrl = requireHttpUrl('SWIG_RPC_URL', configuredRpc);
  const backendUrl = requireHttpUrl(
    'SWIG_API_URL',
    aliasedEnvironment('SWIG_API_URL', 'SWIG_BACKEND_URL') ??
      DEFAULT_BACKEND_URL,
  );
  const apiKey = requiredEnvironment('SWIG_API_KEY');
  const sharedSecret = process.env.SWIG_ED25519_PRIVATE_KEY;
  const legacySecret = process.env.X402_DEVELOPER_KEYPAIR;
  let suppliedDeveloper: Keypair | undefined;
  if (sharedSecret !== undefined) {
    const encoded = sharedSecret.trim();
    if (!/^[0-9a-fA-F]{128}$/.test(encoded)) {
      throw new X402SetupError(
        'SWIG_ED25519_PRIVATE_KEY must be a hex-encoded 64-byte keypair',
      );
    }
    try {
      suppliedDeveloper = Keypair.fromSecretKey(Buffer.from(encoded, 'hex'));
    } catch {
      throw new X402SetupError(
        'SWIG_ED25519_PRIVATE_KEY must contain a valid Ed25519 keypair',
      );
    }
    if (legacySecret !== undefined) {
      const legacy = keypairFromEnvironmentOrState(
        'X402_DEVELOPER_KEYPAIR',
        undefined,
      );
      if (!Buffer.from(suppliedDeveloper.secretKey).equals(legacy.secretKey)) {
        throw new X402SetupError(
          'SWIG_ED25519_PRIVATE_KEY conflicts with X402_DEVELOPER_KEYPAIR',
        );
      }
    }
  }
  const paymentAmount = atomicAmountFromEnvironment(
    'X402_PAYMENT_AMOUNT',
    '1000',
  );
  const facilitatorPort = portFromEnvironment(
    'X402_FACILITATOR_PORT',
    DEFAULT_FACILITATOR_PORT,
  );
  const resourceServerPort = portFromEnvironment(
    'X402_RESOURCE_SERVER_PORT',
    DEFAULT_RESOURCE_SERVER_PORT,
  );
  if (facilitatorPort === resourceServerPort) {
    throw new X402SetupError(
      'The facilitator and resource server must use different ports',
    );
  }
  const state = loadState();
  const facilitator = keypairFromEnvironmentOrState(
    'X402_FACILITATOR_KEYPAIR',
    state.facilitatorSecretKey,
  );
  const developer =
    suppliedDeveloper ??
    keypairFromEnvironmentOrState(
      'X402_DEVELOPER_KEYPAIR',
      state.developerSecretKey,
    );
  const resourceProvider = keypairFromEnvironmentOrState(
    'X402_RESOURCE_PROVIDER_KEYPAIR',
    state.resourceProviderSecretKey,
  );
  if (process.env.X402_FACILITATOR_KEYPAIR === undefined) {
    state.facilitatorSecretKey = [...facilitator.secretKey];
  } else {
    delete state.facilitatorSecretKey;
  }
  if (sharedSecret === undefined && legacySecret === undefined) {
    state.developerSecretKey = [...developer.secretKey];
  } else {
    delete state.developerSecretKey;
  }
  if (process.env.X402_RESOURCE_PROVIDER_KEYPAIR === undefined) {
    state.resourceProviderSecretKey = [...resourceProvider.secretKey];
  } else {
    delete state.resourceProviderSecretKey;
  }
  saveState(state);

  const connection = new Connection(rpcUrl, 'confirmed');

  await ensureFacilitatorBalance(connection, facilitator);

  const swig = await ensureSwig({
    connection,
    state,
    apiKey,
    backendUrl,
    facilitator,
    developer,
  });
  const mint = await ensureMint({
    connection,
    state,
    facilitator,
    paymentAmount,
  });

  await ensureTokenAccounts({
    connection,
    facilitator,
    resourceProvider,
    mint: mint.address,
    tokenProgram: mint.tokenProgram,
    swigWalletAddress: new PublicKey(swig.walletAddress),
    requiredBalance: paymentAmount,
  });

  saveState(state);

  return {
    rpcUrl,
    backendUrl,
    apiKey,
    network: 'devnet',
    x402Network: X402_DEVNET,
    facilitator,
    developer,
    resourceProvider,
    mint: mint.address,
    tokenProgram: mint.tokenProgram,
    swigConfigAddress: swig.configAddress,
    swigWalletAddress: swig.walletAddress,
    paymentAmount,
    facilitatorPort,
    resourceServerPort,
  };
}

async function ensureSwig(args: {
  connection: Connection;
  state: SetupState;
  apiKey: string;
  backendUrl: string;
  facilitator: Keypair;
  developer: Keypair;
}): Promise<{ configAddress: string; walletAddress: string }> {
  const configuredConfig = process.env.SWIG_CONFIG_ADDRESS;
  const configuredWallet = process.env.SWIG_WALLET_ADDRESS;
  if (configuredWallet !== undefined && configuredConfig === undefined) {
    throw new X402SetupError(
      'SWIG_WALLET_ADDRESS requires SWIG_CONFIG_ADDRESS',
    );
  }
  let explicitSwig:
    { configAddress: string; walletAddress: string } | undefined;
  if (configuredConfig !== undefined) {
    let config: PublicKey;
    try {
      config = new PublicKey(configuredConfig);
    } catch {
      throw new X402SetupError('SWIG_CONFIG_ADDRESS must be a public key');
    }
    const [wallet] = PublicKey.findProgramAddressSync(
      [Buffer.from('swig-wallet-address'), config.toBuffer()],
      SWIG_PROGRAM_ID,
    );
    if (
      configuredWallet !== undefined &&
      configuredWallet !== wallet.toBase58()
    ) {
      throw new X402SetupError(
        'SWIG_WALLET_ADDRESS does not match SWIG_CONFIG_ADDRESS',
      );
    }
    explicitSwig = {
      configAddress: config.toBase58(),
      walletAddress: wallet.toBase58(),
    };
  }
  const rememberedSwig =
    args.state.swigConfigAddress &&
    args.state.swigWalletAddress &&
    args.state.developerPublicKey === args.developer.publicKey.toBase58()
      ? {
          configAddress: args.state.swigConfigAddress,
          walletAddress: args.state.swigWalletAddress,
        }
      : undefined;
  const candidate = explicitSwig ?? rememberedSwig;

  if (candidate) {
    const [configAccount, walletAccount] = await Promise.all([
      args.connection.getAccountInfo(
        new PublicKey(candidate.configAddress),
        'confirmed',
      ),
      args.connection.getAccountInfo(
        new PublicKey(candidate.walletAddress),
        'confirmed',
      ),
    ]);

    if (configAccount && !configAccount.owner.equals(SWIG_PROGRAM_ID)) {
      throw new X402SetupError(
        'The configured account is not owned by the Swig program',
      );
    }
    if (configAccount && walletAccount) {
      return candidate;
    }

    if (explicitSwig) {
      throw new X402SetupError(
        'The Swig supplied through the environment does not exist',
      );
    }
  }

  const swig = new SwigClient({
    apiKey: args.apiKey,
    baseUrl: args.backendUrl,
    network: 'devnet',
  });
  const created = await swig.wallets.create({
    feePayer: args.facilitator.publicKey.toBase58(),
    initialUser: {
      ed25519: { publicKey: args.developer.publicKey.toBase58() },
    },
  });

  if (!created.creationTransaction) {
    throw new X402SetupError(
      'Wallet creation response is missing its transaction',
    );
  }
  if (!created.wallet.walletAddress) {
    throw new X402SetupError(
      'Wallet creation response is missing walletAddress',
    );
  }

  await signAndSendPreparedTransaction(
    args.connection,
    created.creationTransaction,
    args.facilitator,
  );
  await waitForAccount(
    args.connection,
    new PublicKey(created.wallet.swigConfigAddress),
  );

  args.state.swigConfigAddress = created.wallet.swigConfigAddress;
  args.state.swigWalletAddress = created.wallet.walletAddress;
  args.state.developerPublicKey = args.developer.publicKey.toBase58();
  saveState(args.state);

  return {
    configAddress: created.wallet.swigConfigAddress,
    walletAddress: created.wallet.walletAddress,
  };
}

async function ensureMint(args: {
  connection: Connection;
  state: SetupState;
  facilitator: Keypair;
  paymentAmount: bigint;
}): Promise<{ address: PublicKey; tokenProgram: PublicKey }> {
  const configuredMint = process.env.X402_MINT;

  if (configuredMint) {
    const address = new PublicKey(configuredMint);
    const account = await args.connection.getAccountInfo(address, 'confirmed');

    if (!account) {
      throw new X402SetupError('X402_MINT does not exist');
    }

    const tokenProgram = requireTokenProgram(account.owner);
    await getMint(args.connection, address, 'confirmed', tokenProgram);
    return { address, tokenProgram };
  }

  const decimals = decimalsForPaymentAmount(args.paymentAmount);
  let mint = args.state.mintSecretKey
    ? Keypair.fromSecretKey(Uint8Array.from(args.state.mintSecretKey))
    : Keypair.generate();
  let existing = await args.connection.getAccountInfo(
    mint.publicKey,
    'confirmed',
  );

  if (existing) {
    const mintState = await getMint(
      args.connection,
      mint.publicKey,
      'confirmed',
      TOKEN_PROGRAM_ID,
    );

    if (
      mintState.decimals === decimals &&
      mintState.mintAuthority?.equals(args.facilitator.publicKey)
    ) {
      return { address: mint.publicKey, tokenProgram: TOKEN_PROGRAM_ID };
    }

    mint = Keypair.generate();
    existing = null;
  }

  args.state.mintSecretKey = Array.from(mint.secretKey);
  saveState(args.state);

  if (!existing) {
    await createMint(
      args.connection,
      args.facilitator,
      args.facilitator.publicKey,
      null,
      decimals,
      mint,
      { commitment: 'confirmed' },
      TOKEN_PROGRAM_ID,
    );
  }

  return { address: mint.publicKey, tokenProgram: TOKEN_PROGRAM_ID };
}

async function ensureTokenAccounts(args: {
  connection: Connection;
  facilitator: Keypair;
  resourceProvider: Keypair;
  mint: PublicKey;
  tokenProgram: PublicKey;
  swigWalletAddress: PublicKey;
  requiredBalance: bigint;
}): Promise<void> {
  const sourceTokenAccount = await getAssociatedTokenAddress(
    args.mint,
    args.swigWalletAddress,
    true,
    args.tokenProgram,
  );
  const resourceProviderTokenAccount = await getAssociatedTokenAddress(
    args.mint,
    args.resourceProvider.publicKey,
    false,
    args.tokenProgram,
  );
  const transaction = new Transaction().add(
    createAssociatedTokenAccountIdempotentInstruction(
      args.facilitator.publicKey,
      sourceTokenAccount,
      args.swigWalletAddress,
      args.mint,
      args.tokenProgram,
    ),
    createAssociatedTokenAccountIdempotentInstruction(
      args.facilitator.publicKey,
      resourceProviderTokenAccount,
      args.resourceProvider.publicKey,
      args.mint,
      args.tokenProgram,
    ),
  );

  await signAndSendTransaction(args.connection, transaction, args.facilitator);

  const source = await getAccount(
    args.connection,
    sourceTokenAccount,
    'confirmed',
    args.tokenProgram,
  );
  if (source.amount >= args.requiredBalance) {
    return;
  }

  const mint = await getMint(
    args.connection,
    args.mint,
    'confirmed',
    args.tokenProgram,
  );
  if (!mint.mintAuthority?.equals(args.facilitator.publicKey)) {
    throw new X402SetupError(
      'The Swig source account has insufficient tokens and the facilitator is not the mint authority',
    );
  }

  await mintTo(
    args.connection,
    args.facilitator,
    args.mint,
    sourceTokenAccount,
    args.facilitator,
    args.requiredBalance - source.amount,
    [],
    { commitment: 'confirmed' },
    args.tokenProgram,
  );
}

async function ensureFacilitatorBalance(
  connection: Connection,
  facilitator: Keypair,
): Promise<void> {
  const balance = await connection.getBalance(
    facilitator.publicKey,
    'confirmed',
  );
  if (balance >= MINIMUM_FACILITATOR_BALANCE) {
    return;
  }

  const signature = await connection.requestAirdrop(
    facilitator.publicKey,
    MINIMUM_FACILITATOR_BALANCE,
  );
  await confirmSignature(connection, signature);
}

async function signAndSendPreparedTransaction(
  connection: Connection,
  prepared: PreparedTransaction,
  signer: Keypair,
): Promise<void> {
  const bytes = Buffer.from(prepared.transaction, 'base64');
  let transaction: Transaction | VersionedTransaction;

  try {
    transaction = VersionedTransaction.deserialize(bytes);
  } catch {
    transaction = Transaction.from(bytes);
  }

  if (transaction instanceof VersionedTransaction) {
    transaction.sign([signer]);
  } else {
    transaction.partialSign(signer);
  }

  const signature = await connection.sendRawTransaction(
    transaction.serialize(),
  );
  await confirmSignature(connection, signature);
}

async function signAndSendTransaction(
  connection: Connection,
  transaction: Transaction,
  signer: Keypair,
): Promise<void> {
  const latest = await connection.getLatestBlockhash('confirmed');
  transaction.feePayer = signer.publicKey;
  transaction.recentBlockhash = latest.blockhash;
  transaction.sign(signer);

  const signature = await connection.sendRawTransaction(
    transaction.serialize(),
  );
  const result = await connection.confirmTransaction(
    { signature, ...latest },
    'confirmed',
  );

  if (result.value.err) {
    throw new X402SetupError('The transaction did not confirm successfully');
  }
}

async function confirmSignature(
  connection: Connection,
  signature: string,
): Promise<void> {
  const result = await connection.confirmTransaction(signature, 'confirmed');
  if (result.value.err) {
    throw new X402SetupError('The transaction did not confirm successfully');
  }
}

async function waitForAccount(
  connection: Connection,
  address: PublicKey,
): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (await connection.getAccountInfo(address, 'confirmed')) {
      return;
    }
    await Bun.sleep(250);
  }

  throw new X402SetupError(`Timed out waiting for ${address.toBase58()}`);
}

function decimalsForPaymentAmount(amount: bigint): number {
  return amount.toString().length - 1;
}

function atomicAmountFromEnvironment(name: string, fallback: string): bigint {
  const raw = process.env[name] ?? fallback;
  if (!/^[1-9][0-9]*$/.test(raw)) {
    throw new X402SetupError(`${name} must be a canonical positive integer`);
  }

  const amount = BigInt(raw);
  if (amount > MAX_U64) {
    throw new X402SetupError(`${name} exceeds u64`);
  }
  return amount;
}

function requireTokenProgram(owner: PublicKey): PublicKey {
  if (owner.equals(TOKEN_PROGRAM_ID)) {
    return TOKEN_PROGRAM_ID;
  }
  if (owner.equals(TOKEN_2022_PROGRAM_ID)) {
    return TOKEN_2022_PROGRAM_ID;
  }
  throw new X402SetupError(
    'X402_MINT is not owned by a supported token program',
  );
}

function loadState(): SetupState {
  if (!existsSync(STATE_FILE)) {
    return {};
  }
  const state = JSON.parse(readFileSync(STATE_FILE, 'utf8')) as SetupState & {
    requesterSecretKey?: number[];
    merchantSecretKey?: number[];
  };

  state.developerSecretKey ??= state.requesterSecretKey;
  state.resourceProviderSecretKey ??= state.merchantSecretKey;
  delete state.requesterSecretKey;
  delete state.merchantSecretKey;

  return state;
}

function saveState(state: SetupState): void {
  mkdirSync(STATE_DIRECTORY, { recursive: true });
  writeFileSync(STATE_FILE, `${JSON.stringify(state, null, 2)}\n`, {
    mode: 0o600,
  });
}

function requiredEnvironment(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new X402SetupError(`${name} is required`);
  }
  return value;
}

function aliasedEnvironment(name: string, alias: string): string | undefined {
  const canonical = process.env[name];
  const legacy = process.env[alias];
  if (canonical !== undefined && legacy !== undefined && canonical !== legacy) {
    throw new X402SetupError(`${name} conflicts with ${alias}`);
  }
  const value = canonical ?? legacy;
  if (value === '') {
    throw new X402SetupError(`${name} (or ${alias}) must not be empty`);
  }
  return value;
}

function requireHttpUrl(name: string, value: string): string {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new X402SetupError(`${name} must be a valid HTTP(S) URL`);
  }
  if (
    (url.protocol !== 'http:' && url.protocol !== 'https:') ||
    !url.hostname
  ) {
    throw new X402SetupError(`${name} must be a valid HTTP(S) URL`);
  }
  return value;
}

function keypairFromEnvironmentOrState(
  name: string,
  rememberedSecretKey: number[] | undefined,
): Keypair {
  const raw = process.env[name];
  if (raw === undefined) {
    return rememberedSecretKey === undefined
      ? Keypair.generate()
      : keypairFromSecretKey(name, rememberedSecretKey);
  }

  let parsed: unknown;

  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new X402SetupError(`${name} must be a JSON secret-key array`);
  }

  return keypairFromSecretKey(name, parsed);
}

function keypairFromSecretKey(name: string, value: unknown): Keypair {
  if (
    !Array.isArray(value) ||
    value.length !== 64 ||
    !value.every(
      (byte) =>
        Number.isInteger(byte) && Number(byte) >= 0 && Number(byte) <= 255,
    )
  ) {
    throw new X402SetupError(`${name} must contain 64 byte values`);
  }

  try {
    return Keypair.fromSecretKey(Uint8Array.from(value));
  } catch {
    throw new X402SetupError(`${name} must contain a valid Ed25519 keypair`);
  }
}

function portFromEnvironment(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw === '') {
    return fallback;
  }

  const port = Number(raw);
  if (!Number.isSafeInteger(port) || port < 1 || port > 65_535) {
    throw new X402SetupError(`${name} must be a valid TCP port`);
  }
  return port;
}

if (import.meta.main) {
  try {
    const fixture = await ensureX402Fixture();
    console.log('x402 fixture is ready');
    console.log(`Swig config: ${fixture.swigConfigAddress}`);
    console.log(`Swig wallet: ${fixture.swigWalletAddress}`);
    console.log(`Developer: ${fixture.developer.publicKey.toBase58()}`);
    console.log(
      `Resource provider: ${fixture.resourceProvider.publicKey.toBase58()}`,
    );
    console.log(`Facilitator: ${fixture.facilitator.publicKey.toBase58()}`);
    console.log(`Mint: ${fixture.mint.toBase58()}`);
    console.log(
      `Payment amount: ${fixture.paymentAmount.toString()} atomic units`,
    );
  } catch (error) {
    console.error(
      error instanceof X402SetupError
        ? error.message
        : 'Could not prepare the x402 fixture. Check the local services and Devnet funding.',
    );
    process.exitCode = 1;
  }
}
