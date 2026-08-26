import { describe, expect, test } from 'bun:test';

import { SwigClient } from '../server/typescript/index.js';
import { WalletsClient } from './client.js';
import {
  addRecoveryAuthorityRequest,
  addRoleRequest,
  buildTransactionRequest,
  cancelRecoveryRequest,
  configureRecoveryRequest,
  createWalletRequest,
  executeRecoveryRequest,
  startRecoveryRequest,
  swapRequest,
  transferSolRequest,
  transferTokenRequest,
} from './requests.js';

type CapturedRequest = {
  url: string;
  method?: string;
  headers: Headers;
  body: unknown;
};

function jsonFetch(
  handler: (request: CapturedRequest) => unknown,
): typeof fetch {
  return (async (input, init) => {
    const request = new Request(input, init);
    const text = await request.text();

    return new Response(
      JSON.stringify(
        handler({
          url: request.url,
          method: request.method,
          headers: request.headers,
          body: text ? JSON.parse(text) : undefined,
        }),
      ),
      {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      },
    );
  }) as typeof fetch;
}

describe('WalletsClient', () => {
  test('creates a wallet handle from an IdP session', () => {
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'mainnet',
    );

    const wallet = wallets.fromIdpSession(
      {
        configAddress: 'swig_config_123',
        walletAddress: 'wallet_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
      },
      { network: 'devnet' },
    );

    expect(wallet.swigConfigAddress).toBe('swig_config_123');
    expect(wallet.walletAddress).toBe('wallet_123');
    expect(wallet.requesterAuthority).toEqual({
      ed25519: { publicKey: 'requester_123' },
    });
    expect(wallet.network).toBe('devnet');
  });

  test('creates a wallet handle from a Swig address string', () => {
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'devnet',
    );

    const wallet = wallets.use('swig_config_123', {
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    expect(wallet.swigConfigAddress).toBe('swig_config_123');
    expect(wallet.requesterAuthority).toEqual({
      ed25519: { publicKey: 'requester_123' },
    });
    expect(wallet.network).toBe('devnet');
  });

  test('builds SOL transfer requests for the transaction API', () => {
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'mainnet',
    );
    const wallet = wallets.fromIdpSession({
      configAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    expect(
      transferSolRequest(
        wallet,
        {
          feePayer: 'payer_123',
          destination: 'destination_123',
          amount: 1_000_000n,
        },
        'mainnet',
      ),
    ).toEqual({
      network: 'NETWORK_MAINNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
      destination: 'destination_123',
      lamports: '1000000',
    });
  });

  test('maps general role authorities and actions to typed protobuf fields', async () => {
    const calls: Array<{ path: string; body: unknown }> = [];
    const wallets = new WalletsClient(
      {
        post: async (path: string, body: unknown) => {
          calls.push({ path, body });
          return {
            transaction: {
              transaction: 'add-role-base64',
              transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            },
          };
        },
      } as never,
      'devnet',
    );
    const wallet = wallets.use('swig_123', {
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    await wallet.roles.add({
      feePayer: 'payer_123',
      authority: {
        participantSet: { address: 'participant_set_new' },
      },
      actions: [
        { type: 'all' },
        { type: 'allButManageAuthority' },
        { type: 'manageAuthority' },
        { type: 'solLimit', amount: 1_000_000n },
        { type: 'solRecurringLimit', recurringAmount: 2, window: 3 },
        {
          type: 'solDestinationLimit',
          amount: 4,
          destination: 'sol_destination',
        },
        {
          type: 'solRecurringDestinationLimit',
          recurringAmount: 5,
          window: 6,
          destination: 'recurring_sol_destination',
        },
        { type: 'tokenLimit', mint: 'mint_1', amount: 7 },
        {
          type: 'tokenRecurringLimit',
          mint: 'mint_2',
          recurringAmount: 8,
          window: 9,
        },
        {
          type: 'tokenDestinationLimit',
          mint: 'mint_3',
          amount: 10,
          destination: 'token_destination',
        },
        {
          type: 'tokenRecurringDestinationLimit',
          mint: 'mint_4',
          recurringAmount: 11,
          window: 12,
          destination: 'recurring_token_destination',
        },
        { type: 'program', programId: 'program_123' },
        { type: 'programAll' },
        { type: 'programCurated' },
        { type: 'stakeLimit', amount: 13 },
        { type: 'stakeRecurringLimit', recurringAmount: 14, window: 15 },
        { type: 'stakeAll' },
        { type: 'subAccount' },
      ],
    });

    expect(calls).toEqual([
      {
        path: '/transaction/wallet/role/add',
        body: {
          network: 'NETWORK_DEVNET',
          feePayer: 'payer_123',
          swigAddress: 'swig_123',
          requesterAuthority: {
            ed25519: { publicKey: 'requester_123' },
          },
          authority: {
            participantSet: {
              participantSetAddress: 'participant_set_new',
            },
          },
          actions: [
            { all: {} },
            { allButManageAuthority: {} },
            { manageAuthority: {} },
            { solLimit: { amount: '1000000' } },
            { solRecurringLimit: { recurringAmount: '2', window: '3' } },
            {
              solDestinationLimit: {
                amount: '4',
                destination: 'sol_destination',
              },
            },
            {
              solRecurringDestinationLimit: {
                recurringAmount: '5',
                window: '6',
                destination: 'recurring_sol_destination',
              },
            },
            { tokenLimit: { mint: 'mint_1', amount: '7' } },
            {
              tokenRecurringLimit: {
                mint: 'mint_2',
                recurringAmount: '8',
                window: '9',
              },
            },
            {
              tokenDestinationLimit: {
                mint: 'mint_3',
                amount: '10',
                destination: 'token_destination',
              },
            },
            {
              tokenRecurringDestinationLimit: {
                mint: 'mint_4',
                recurringAmount: '11',
                window: '12',
                destination: 'recurring_token_destination',
              },
            },
            { program: { programId: 'program_123' } },
            { programAll: {} },
            { programCurated: {} },
            { stakeLimit: { amount: '13' } },
            {
              stakeRecurringLimit: {
                recurringAmount: '14',
                window: '15',
              },
            },
            { stakeAll: {} },
            { subAccount: {} },
          ],
        },
      },
    ]);
  });

  test('rejects ParticipantSet as an add-role requester before transport', async () => {
    const wallet = new WalletsClient(
      {
        post: async () => {
          throw new Error('transport must not be called');
        },
      } as never,
      'devnet',
    ).use('swig_123', {
      requesterAuthority: {
        participantSet: { address: 'participant_set_requester' },
      },
    });

    await expect(
      wallet.roles.add({
        feePayer: 'payer_123',
        authority: { ed25519: { publicKey: 'role_public_key' } },
        actions: [{ type: 'all' }],
      }),
    ).rejects.toThrow(
      'Add role requesterAuthority must use ed25519 or secp256r1',
    );
  });

  test('rejects a ParticipantSet roleId before add-role transport', () => {
    const wallet = new WalletsClient(
      { post: async () => ({}) } as never,
      'devnet',
    ).use('swig_123', {
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    expect(() =>
      addRoleRequest(
        wallet,
        {
          feePayer: 'payer_123',
          authority: {
            participantSet: { address: 'participant_set_123', roleId: 7 },
          },
          actions: [{ type: 'all' }],
        } as never,
        'devnet',
      ),
    ).toThrow('Add role ParticipantSet authority must omit roleId');
  });

  test('rejects ParticipantSet from unsupported endpoint shapes', () => {
    const participantSet = {
      participantSet: { address: 'participant_set_123' },
    } as const;
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'devnet',
    );
    const wallet = wallets.use('swig_123', {
      requesterAuthority: participantSet,
    });

    expect(() =>
      createWalletRequest(
        {
          feePayer: 'payer_123',
          initialUser: participantSet,
        } as never,
        'devnet',
      ),
    ).toThrow('initialUser does not support ParticipantSet authority');
    expect(() =>
      swapRequest(
        wallet,
        {
          feePayer: 'payer_123',
          inputMint: 'input_mint',
          outputMint: 'output_mint',
          amount: 1,
        },
        'devnet',
      ),
    ).toThrow('requesterAuthority does not support ParticipantSet authority');
    expect(() =>
      addRecoveryAuthorityRequest(wallet, { feePayer: 'payer_123' }, 'devnet'),
    ).toThrow('requesterAuthority does not support ParticipantSet authority');
    expect(() =>
      cancelRecoveryRequest(wallet, { feePayer: 'payer_123' }, 'devnet'),
    ).toThrow('requesterAuthority does not support ParticipantSet authority');
    expect(() =>
      buildTransactionRequest(
        wallet,
        {
          feePayer: 'payer_123',
          instructions: [],
          addressLookupTableAccounts: ['lookup_table_123'],
        },
        'devnet',
      ),
    ).toThrow(
      'ParticipantSet requesterAuthority does not support addressLookupTableAccounts',
    );
  });

  test('builds token transfer requests for the transaction API', () => {
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'devnet',
    );
    const wallet = wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    expect(
      transferTokenRequest(
        wallet,
        {
          feePayer: 'payer_123',
          mint: 'mint_123',
          destinationOwner: 'owner_123',
          amount: '2500',
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
      mint: 'mint_123',
      destinationOwner: 'owner_123',
      amount: '2500',
    });
  });

  test('builds Jupiter swap requests for the transaction API', () => {
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'devnet',
    );
    const wallet = wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    expect(
      swapRequest(
        wallet,
        {
          feePayer: 'payer_123',
          inputMint: 'input_mint_123',
          outputMint: 'output_mint_123',
          amount: 1_000n,
          slippageBps: 75,
          destinationAccount: 'destination_account_123',
          tipAmountLamports: '5000',
          computeUnitPricePercentile: 'high',
          maxAccounts: 32,
          mode: 'fast',
          blockhashSlotsToExpiry: 10,
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
      inputMint: 'input_mint_123',
      outputMint: 'output_mint_123',
      amount: '1000',
      slippageBps: 75,
      destinationAccount: 'destination_account_123',
      wrapAndUnwrapSol: undefined,
      tipAmountLamports: '5000',
      computeUnitPricePercentile: 'high',
      maxAccounts: 32,
      mode: 'fast',
      blockhashSlotsToExpiry: 10,
    });
  });

  test('builds recovery requests for the transaction API', () => {
    const wallets = new WalletsClient(
      { post: async () => ({}) } as never,
      'devnet',
    );
    const wallet = wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    expect(
      startRecoveryRequest(
        wallet,
        {
          feePayer: 'payer_123',
          guardianPubkey: 'guardian_123',
          newAuthority: 'new_authority_123',
          newAuthorityKind: 'secp256r1',
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      guardianPubkey: 'guardian_123',
      newAuthority: 'new_authority_123',
      newAuthorityKind: 'WALLET_AUTHORITY_KIND_SECP256R1',
    });
    expect(
      addRecoveryAuthorityRequest(
        wallet,
        {
          feePayer: 'payer_123',
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });
    expect(
      configureRecoveryRequest(
        wallet,
        {
          feePayer: 'payer_123',
          guardianPubkey: 'guardian_123',
          delaySeconds: 86_400,
          targetRoleId: 0,
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      guardianPubkey: 'guardian_123',
      delaySeconds: 86_400,
      targetRoleId: 0,
    });
    expect(
      cancelRecoveryRequest(
        wallet,
        {
          feePayer: 'payer_123',
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });
    expect(
      executeRecoveryRequest(
        wallet,
        {
          feePayer: 'payer_123',
          newAuthority: 'new_authority_123',
          newAuthorityKind: 'secp256r1',
        },
        'devnet',
      ),
    ).toEqual({
      network: 'NETWORK_DEVNET',
      feePayer: 'payer_123',
      swigAddress: 'swig_config_123',
      newAuthority: 'new_authority_123',
      newAuthorityKind: 'WALLET_AUTHORITY_KIND_SECP256R1',
    });
  });

  test('prepares wallet creation through the local transaction endpoint', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        if (request.method === 'GET') {
          return {
            id: 'policy_123',
            name: 'Default policy',
            description: null,
            authority: {
              type: 'Ed25519',
              publicKey: 'initial_user_123',
            },
            actions: [{ type: 'All' }],
            guardianEnabled: false,
            guardianAuthority: null,
            guardianDelaySeconds: 86_400,
          };
        }

        return {
          network: 'NETWORK_DEVNET',
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
          transactions: [
            {
              transaction: 'base64-create-tx',
              transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
              network: 'NETWORK_DEVNET',
              recentBlockhash: 'blockhash_123',
              expiresAt: '2026-05-13T00:00:00Z',
              kind: 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET',
            },
          ],
        };
      }),
    });

    const created = await swig.wallets.create({
      feePayer: 'payer_123',
      policyId: 'policy_123',
    });

    expect(calls).toHaveLength(2);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/wallet/policies/policy_123',
      method: 'GET',
    });
    expect(calls[1]).toMatchObject({
      url: 'http://localhost:8080/transaction/wallet/create',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        policyId: 'policy_123',
      },
    });
    expect(calls[0]?.headers.get('authorization')).toBe('Bearer sk_test');
    expect(calls[1]?.headers.get('authorization')).toBe('Bearer sk_test');
    expect(created.wallet).toEqual({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      network: 'devnet',
    });
    expect(created.transactions).toHaveLength(1);
    expect(created.creationTransaction).toMatchObject({
      transaction: 'base64-create-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
      recentBlockhash: 'blockhash_123',
      kind: 'create-swig-wallet',
    });
    expect(created.feePayerOnlyTransactions).toHaveLength(1);
    expect(created.clientAuthorityTransactions).toEqual([]);
    expect(created.operatorSignedTransactions).toEqual([]);
  });

  test('returns a recovery setup plan from a recovery-enabled policy', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        if (request.method === 'GET') {
          return {
            id: 'policy_recovery_123',
            name: 'Recovery policy',
            description: null,
            authority: {
              type: 'Secp256r1',
              publicKey: 'passkey_public_key_123',
            },
            actions: [{ type: 'All' }],
            guardianEnabled: true,
            guardianAuthority: {
              type: 'Ed25519',
              publicKey: 'guardian_123',
            },
            guardianDelaySeconds: 86_400,
          };
        }

        return {
          network: 'NETWORK_DEVNET',
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
          transactions: [
            {
              transaction: 'base64-create-tx',
              transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
              network: 'NETWORK_DEVNET',
              kind: 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET',
            },
          ],
        };
      }),
    });

    const created = await swig.wallets.create({
      feePayer: 'payer_123',
      policyId: 'policy_recovery_123',
    });

    expect(calls).toHaveLength(2);
    expect(created.transactions).toHaveLength(1);
    expect(created.recoverySetup).toEqual({
      requesterAuthority: {
        secp256r1: {
          publicKey: 'passkey_public_key_123',
        },
      },
      guardianPubkey: 'guardian_123',
      delaySeconds: 86_400,
    });
  });

  test('uses create recovery options when the policy guardian is provided at creation', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        if (request.method === 'GET') {
          return {
            id: 'policy_recovery_at_creation_123',
            name: 'Recovery policy',
            description: null,
            authority: null,
            actions: [{ type: 'All' }],
            guardianEnabled: true,
            guardianAuthority: null,
            guardianDelaySeconds: '1',
          };
        }

        return {
          network: 'NETWORK_DEVNET',
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
          transactions: [
            {
              transaction: 'base64-create-tx',
              transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
              network: 'NETWORK_DEVNET',
              kind: 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET',
            },
          ],
        };
      }),
    });

    const created = await swig.wallets.create({
      feePayer: 'payer_123',
      policyId: 'policy_recovery_at_creation_123',
      initialUser: {
        secp256r1: {
          publicKey: 'passkey_public_key_123',
        },
      },
      recovery: {
        guardianPubkey: 'guardian_123',
      },
    });

    expect(calls).toHaveLength(2);
    expect(created.recoverySetup).toEqual({
      requesterAuthority: {
        secp256r1: {
          publicKey: 'passkey_public_key_123',
        },
      },
      guardianPubkey: 'guardian_123',
      delaySeconds: 1,
    });
  });

  test('prepares wallet creation without a policy id when an initial user is provided', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          network: 'NETWORK_DEVNET',
          wallet: {
            swigConfigAddress: 'swig_config_456',
            walletAddress: 'wallet_456',
          },
          transactions: [
            {
              transaction: 'base64-create-tx',
              transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
              network: 'NETWORK_DEVNET',
              kind: 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET',
            },
          ],
        };
      }),
    });

    await swig.wallets.create({
      feePayer: 'payer_123',
      initialUser: {
        ed25519: {
          publicKey: 'initial_user_123',
        },
      },
    });

    expect(calls[0]).toMatchObject({
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        initialUser: {
          ed25519: {
            publicKey: 'initial_user_123',
          },
        },
      },
    });
    expect(
      (calls[0]?.body as Record<string, unknown>).policyId,
    ).toBeUndefined();
  });

  test('prepares SOL transfers through the local transaction endpoint', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          transaction: 'base64-transfer-tx',
          transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
          network: 'NETWORK_DEVNET',
          recentBlockhash: 'blockhash_456',
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    const prepared = await wallet.transfer.sol({
      feePayer: 'payer_123',
      destination: 'destination_123',
      amount: 42n,
    });

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/transfer/sol',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
        destination: 'destination_123',
        lamports: '42',
      },
    });
    expect(prepared).toMatchObject({
      transaction: 'base64-transfer-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
      recentBlockhash: 'blockhash_456',
    });
  });

  test('prepares token transfers through the opinionated wallet API', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          transaction: 'base64-token-transfer-tx',
          transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
          network: 'NETWORK_DEVNET',
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    const prepared = await wallet.transfer.token({
      feePayer: 'payer_123',
      mint: 'mint_123',
      destinationOwner: 'owner_123',
      amount: 42n,
    });

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/transfer/spl-token',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
        mint: 'mint_123',
        destinationOwner: 'owner_123',
        amount: '42',
      },
    });
    expect(prepared).toMatchObject({
      transaction: 'base64-token-transfer-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
    });
  });

  test('prepares grouped wallet operations through the transaction API', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
          transactions: [
            {
              transaction: 'base64-grouped-tx',
              transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
              network: 'NETWORK_DEVNET',
              recentBlockhash: 'blockhash_grouped',
            },
          ],
          network: 'NETWORK_DEVNET',
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    const prepared = await wallet.prepare({
      feePayer: 'payer_123',
      operations: [
        {
          type: 'transferSol',
          destination: 'destination_123',
          amount: 42n,
        },
        {
          type: 'transferToken',
          mint: 'mint_123',
          destinationOwner: 'owner_123',
          amount: '2500',
        },
      ],
    });

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/prepare/batch',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
        operations: [
          {
            transferSol: {
              destination: 'destination_123',
              lamports: '42',
            },
          },
          {
            transferToken: {
              mint: 'mint_123',
              destinationOwner: 'owner_123',
              amount: '2500',
            },
          },
        ],
      },
    });
    expect(prepared.transactions).toHaveLength(1);
    expect(prepared.transactions[0]).toMatchObject({
      transaction: 'base64-grouped-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
      recentBlockhash: 'blockhash_grouped',
    });
    expect(prepared.wallet).toEqual({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      network: 'devnet',
    });
    expect(prepared.feePayerOnlyTransactions).toHaveLength(1);
    expect(prepared.clientAuthorityTransactions).toEqual([]);
  });

  test('builds custom transactions through the custom preparation endpoint', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          transaction: 'base64-custom-tx',
          transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
          network: 'NETWORK_DEVNET',
          recentBlockhash: 'blockhash_custom',
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    const prepared = await wallet.buildTransaction({
      feePayer: 'payer_123',
      instructions: [
        {
          programId: 'program_123',
          accounts: [
            {
              pubkey: 'account_123',
              isSigner: true,
              isWritable: true,
            },
          ],
          data: new Uint8Array([1, 2, 3]),
        },
      ],
      addressLookupTableAccounts: ['lookup_table_123'],
    });

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/prepare/custom',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
        instructions: [
          {
            programId: 'program_123',
            accounts: [
              {
                pubkey: 'account_123',
                isSigner: true,
                isWritable: true,
              },
            ],
            data: 'AQID',
          },
        ],
        addressLookupTableAccounts: ['lookup_table_123'],
      },
    });
    expect(prepared).toMatchObject({
      transaction: 'base64-custom-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
      recentBlockhash: 'blockhash_custom',
    });
  });

  test('prepares Jupiter swaps through the local transaction endpoint', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          transaction: 'base64-swap-tx',
          transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
          network: 'NETWORK_DEVNET',
          recentBlockhash: 'blockhash_789',
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    const prepared = await wallet.swap.jupiter({
      feePayer: 'payer_123',
      inputMint: 'input_mint_123',
      outputMint: 'output_mint_123',
      amount: 42n,
      slippageBps: 100,
      destinationAccount: 'destination_account_123',
    });

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/swap/jupiter',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
        inputMint: 'input_mint_123',
        outputMint: 'output_mint_123',
        amount: '42',
        slippageBps: 100,
        destinationAccount: 'destination_account_123',
      },
    });
    expect(prepared).toMatchObject({
      transaction: 'base64-swap-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
      recentBlockhash: 'blockhash_789',
    });
  });

  test('prepares recovery setup from a create-time setup plan', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        if (request.url.endsWith('/transaction/recovery/configure')) {
          return {
            transaction: 'base64-configure-recovery-tx',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            network: 'NETWORK_DEVNET',
            kind: 'PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY',
            wallet: {
              swigConfigAddress: 'swig_config_123',
              walletAddress: 'wallet_123',
            },
          };
        }

        return {
          transaction: 'base64-add-authority-tx',
          transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
          network: 'NETWORK_DEVNET',
          kind: 'PREPARED_TRANSACTION_KIND_ADD_AUTHORITY',
          signatureRequests: [
            {
              scheme: 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1',
              signer: 'passkey_public_key_123',
              messageHash: 'hash_123',
              slot: 123,
              counter: 1,
            },
          ],
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
    });
    const recoverySetup = {
      requesterAuthority: { secp256r1: { publicKey: 'passkey_123' } },
      guardianPubkey: 'guardian_123',
      delaySeconds: 86_400,
    } as const;

    const prepared = await wallet.recovery.prepareSetup({
      feePayer: 'payer_123',
      ...recoverySetup,
    });

    expect(calls).toHaveLength(2);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/wallet/recovery-authority/add',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { secp256r1: { publicKey: 'passkey_123' } },
      },
    });
    expect(calls[1]).toMatchObject({
      url: 'http://localhost:8080/transaction/recovery/configure',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        guardianPubkey: 'guardian_123',
        delaySeconds: 86_400,
      },
    });
    expect(prepared.transactions).toHaveLength(2);
    expect(prepared.clientAuthorityTransactions).toHaveLength(1);
    expect(prepared.operatorSignedTransactions).toHaveLength(1);
    expect(prepared.addAuthorityTransaction).toMatchObject({
      transaction: 'base64-add-authority-tx',
      kind: 'add-authority',
    });
    expect(prepared.configureRecoveryTransaction).toMatchObject({
      transaction: 'base64-configure-recovery-tx',
      kind: 'configure-recovery',
    });
  });

  test('prepares recovery flow through the transaction API', async () => {
    const calls: CapturedRequest[] = [];
    const swig = new SwigClient({
      apiKey: 'sk_test',
      baseUrl: 'http://localhost:8080',
      network: 'devnet',
      fetch: jsonFetch((request) => {
        calls.push(request);
        return {
          transaction: `base64-${calls.length}-tx`,
          transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
          network: 'NETWORK_DEVNET',
          recentBlockhash: 'blockhash_recovery',
          wallet: {
            swigConfigAddress: 'swig_config_123',
            walletAddress: 'wallet_123',
          },
        };
      }),
    });
    const wallet = swig.wallets.use({
      swigConfigAddress: 'swig_config_123',
      walletAddress: 'wallet_123',
      requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
    });

    const started = await wallet.recovery.start({
      feePayer: 'payer_123',
      guardianPubkey: 'guardian_123',
      newAuthority: 'new_authority_123',
      newAuthorityKind: 'secp256r1',
    });
    const cancelled = await wallet.recovery.cancel({
      feePayer: 'payer_123',
    });
    const executed = await wallet.recovery.execute({
      feePayer: 'payer_123',
      newAuthority: 'new_authority_123',
      newAuthorityKind: 'secp256r1',
    });

    expect(calls).toHaveLength(3);
    expect(calls[0]).toMatchObject({
      url: 'http://localhost:8080/transaction/recovery/start',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        guardianPubkey: 'guardian_123',
        newAuthority: 'new_authority_123',
        newAuthorityKind: 'WALLET_AUTHORITY_KIND_SECP256R1',
      },
    });
    expect(calls[1]).toMatchObject({
      url: 'http://localhost:8080/transaction/recovery/cancel',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        requesterAuthority: { ed25519: { publicKey: 'requester_123' } },
      },
    });
    expect(calls[2]).toMatchObject({
      url: 'http://localhost:8080/transaction/recovery/execute',
      method: 'POST',
      body: {
        network: 'NETWORK_DEVNET',
        feePayer: 'payer_123',
        swigAddress: 'swig_config_123',
        newAuthority: 'new_authority_123',
        newAuthorityKind: 'WALLET_AUTHORITY_KIND_SECP256R1',
      },
    });
    expect(started).toMatchObject({
      transaction: 'base64-1-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
    });
    expect(cancelled.transaction).toBe('base64-2-tx');
    expect(executed.transaction).toBe('base64-3-tx');
  });
});
