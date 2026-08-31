import { describe, expect, test } from 'bun:test';

import {
  normalizeAmount,
  normalizeCreateWalletResponse,
  normalizeInstruction,
  normalizePreparedTransaction,
  normalizePrepareTransactionsResponse,
  normalizeSubmittedTransaction,
  normalizeSwigTokenBalances,
  normalizeSwigTokenTransactions,
} from './normalizers.js';

describe('wallet normalizers', () => {
  test('normalizes the x402 prepared-transaction kind string', () => {
    expect(
      normalizePreparedTransaction({
        transaction: 'base64-x402-tx',
        kind: 'PREPARED_TRANSACTION_KIND_X402_PAYMENT',
      }).kind,
    ).toBe('x402-payment');
  });

  test('normalizes the numeric x402 prepared-transaction kind', () => {
    expect(
      normalizePreparedTransaction({
        transaction: 'base64-x402-tx',
        kind: 5,
      }).kind,
    ).toBe('x402-payment');
  });

  test('preserves the numeric ParticipantSet prepared-transaction kind', () => {
    expect(
      normalizePreparedTransaction({
        transaction: 'base64-participant-set-tx',
        kind: 4,
      }).kind,
    ).toBe('create-participant-set');
  });

  test('accepts already-normalized signature requests', () => {
    expect(
      normalizePreparedTransaction({
        transaction: 'base64-x402-tx',
        signatureRequests: [
          {
            scheme: 'secp256r1',
            signer: 'requester_123',
            messageHash: 'hash_123',
            slot: 42,
            counter: 7,
          },
        ],
      }).signatureRequests,
    ).toEqual([
      {
        scheme: 'secp256r1',
        signer: 'requester_123',
        messageHash: 'hash_123',
        slot: 42,
        counter: 7,
      },
    ]);
  });

  test('normalizes snake_case prepared transaction responses', () => {
    expect(
      normalizePreparedTransaction({
        unsigned_transaction: 'base64-tx',
        transaction_encoding: 'TRANSACTION_ENCODING_BASE64',
        network: 'NETWORK_DEVNET',
        recent_blockhash: 'blockhash_123',
      }),
    ).toEqual({
      transaction: 'base64-tx',
      transactionEncoding: 'base64',
      network: 'devnet',
      recentBlockhash: 'blockhash_123',
      signatureRequests: [],
    });
  });

  test('normalizes a ParticipantSet approval plan from ProtoJSON', () => {
    expect(
      normalizePreparedTransaction({
        transaction: 'base64-tx',
        participantSetApprovalPlan: {
          participantSetAddress: 'participant-set-123',
          roleId: 4,
          expirationSlot: '12345',
          nonce: 9,
          transactionDigest: '11'.repeat(32),
          compilationEnvelope: 'envelope-123',
          threshold: 2,
          members: [
            {
              memberIndex: 1,
              authority: {
                secp256r1: { publicKey: `03${'22'.repeat(32)}` },
              },
              challenge: '33'.repeat(32),
            },
          ],
        },
      }),
    ).toMatchObject({
      participantSetApprovalPlan: {
        type: 'participantSet',
        participantSetAddress: 'participant-set-123',
        roleId: 4,
        expirationSlot: '12345',
        nonce: 9,
        threshold: 2,
        members: [
          {
            memberIndex: 1,
            authority: {
              secp256r1: { publicKey: `03${'22'.repeat(32)}` },
            },
          },
        ],
      },
    });
  });

  test('normalizes create wallet responses with multiple prepared transactions', () => {
    expect(
      normalizeCreateWalletResponse({
        wallet: {
          swigConfigAddress: 'swig_config_123',
          walletAddress: 'wallet_123',
        },
        transactions: [
          {
            transaction: 'base64-create-tx',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            kind: 'PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET',
          },
          {
            transaction: 'base64-add-authority-tx',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            kind: 'PREPARED_TRANSACTION_KIND_ADD_AUTHORITY',
            signatureRequests: [
              {
                scheme: 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1',
                signer: 'compressed-passkey',
                messageHash: 'message-hash',
                slot: '42',
                counter: 1,
              },
            ],
          },
          {
            transaction: 'base64-configure-recovery-tx',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            kind: 'PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY',
          },
        ],
        network: 'NETWORK_DEVNET',
      }),
    ).toEqual({
      wallet: {
        swigConfigAddress: 'swig_config_123',
        walletAddress: 'wallet_123',
      },
      creationTransaction: {
        transaction: 'base64-create-tx',
        transactionEncoding: 'base64',
        kind: 'create-swig-wallet',
        network: 'devnet',
        signatureRequests: [],
      },
      addAuthorityTransaction: {
        transaction: 'base64-add-authority-tx',
        transactionEncoding: 'base64',
        kind: 'add-authority',
        network: 'devnet',
        signatureRequests: [
          {
            scheme: 'secp256r1',
            signer: 'compressed-passkey',
            messageHash: 'message-hash',
            slot: 42,
            counter: 1,
          },
        ],
      },
      configureRecoveryTransaction: {
        transaction: 'base64-configure-recovery-tx',
        transactionEncoding: 'base64',
        kind: 'configure-recovery',
        network: 'devnet',
        signatureRequests: [],
      },
      transactions: [
        {
          transaction: 'base64-create-tx',
          transactionEncoding: 'base64',
          kind: 'create-swig-wallet',
          network: 'devnet',
          signatureRequests: [],
        },
        {
          transaction: 'base64-add-authority-tx',
          transactionEncoding: 'base64',
          kind: 'add-authority',
          network: 'devnet',
          signatureRequests: [
            {
              scheme: 'secp256r1',
              signer: 'compressed-passkey',
              messageHash: 'message-hash',
              slot: 42,
              counter: 1,
            },
          ],
        },
        {
          transaction: 'base64-configure-recovery-tx',
          transactionEncoding: 'base64',
          kind: 'configure-recovery',
          network: 'devnet',
          signatureRequests: [],
        },
      ],
      clientAuthorityTransactions: [
        {
          transaction: 'base64-add-authority-tx',
          transactionEncoding: 'base64',
          kind: 'add-authority',
          network: 'devnet',
          signatureRequests: [
            {
              scheme: 'secp256r1',
              signer: 'compressed-passkey',
              messageHash: 'message-hash',
              slot: 42,
              counter: 1,
            },
          ],
        },
      ],
      operatorSignedTransactions: [
        {
          transaction: 'base64-configure-recovery-tx',
          transactionEncoding: 'base64',
          kind: 'configure-recovery',
          network: 'devnet',
          signatureRequests: [],
        },
      ],
      feePayerOnlyTransactions: [
        {
          transaction: 'base64-create-tx',
          transactionEncoding: 'base64',
          kind: 'create-swig-wallet',
          network: 'devnet',
          signatureRequests: [],
        },
      ],
      network: 'devnet',
    });
  });

  test('normalizes grouped prepare responses', () => {
    expect(
      normalizePrepareTransactionsResponse({
        wallet: {
          swigConfigAddress: 'swig_config_123',
          walletAddress: 'wallet_123',
        },
        transactions: [
          {
            transaction: 'base64-grouped-tx',
            transactionEncoding: 'TRANSACTION_ENCODING_BASE64',
            network: 'NETWORK_DEVNET',
            signatureRequests: [
              {
                scheme: 'AUTHORITY_SIGNATURE_SCHEME_SECP256R1',
                signer: 'compressed-passkey',
                messageHash: 'message-hash',
                slot: '42',
                counter: 1,
              },
            ],
          },
        ],
        network: 'NETWORK_DEVNET',
      }),
    ).toEqual({
      wallet: {
        swigConfigAddress: 'swig_config_123',
        walletAddress: 'wallet_123',
        network: 'devnet',
      },
      transactions: [
        {
          transaction: 'base64-grouped-tx',
          transactionEncoding: 'base64',
          network: 'devnet',
          signatureRequests: [
            {
              scheme: 'secp256r1',
              signer: 'compressed-passkey',
              messageHash: 'message-hash',
              slot: 42,
              counter: 1,
            },
          ],
        },
      ],
      clientAuthorityTransactions: [
        {
          transaction: 'base64-grouped-tx',
          transactionEncoding: 'base64',
          network: 'devnet',
          signatureRequests: [
            {
              scheme: 'secp256r1',
              signer: 'compressed-passkey',
              messageHash: 'message-hash',
              slot: 42,
              counter: 1,
            },
          ],
        },
      ],
      feePayerOnlyTransactions: [],
      network: 'devnet',
    });
  });

  test('normalizes instruction account defaults and byte data', () => {
    expect(
      normalizeInstruction({
        programId: 'program_123',
        accounts: [{ pubkey: 'account_123' }],
        data: new Uint8Array([1, 2, 3]),
      }),
    ).toEqual({
      programId: 'program_123',
      accounts: [
        {
          pubkey: 'account_123',
          isSigner: false,
          isWritable: false,
        },
      ],
      data: 'AQID',
    });
  });

  test('serializes numeric amounts as strings', () => {
    expect(normalizeAmount(1_000_000n)).toBe('1000000');
    expect(normalizeAmount('2500')).toBe('2500');
  });

  test('normalizes sponsored submission responses', () => {
    expect(
      normalizeSubmittedTransaction({
        request_id: 'request_123',
        signature: 'signature_123',
        spent_by_paymaster: '5000',
      }),
    ).toEqual({
      requestId: 'request_123',
      signature: 'signature_123',
      spentByPaymaster: '5000',
    });
  });

  test('preserves asset kinds on wallet balances and transactions', () => {
    expect(
      normalizeSwigTokenBalances({
        swig_config_address: 'swig_123',
        wallet_address: 'wallet_123',
        total_usd_value: 1,
        balances: [
          {
            mint_address: 'mint_123',
            token_program: 1,
            token_symbol: 'USDC',
            token_name: 'USD Coin',
            decimals: 6,
            amount_raw: '1000000',
            ui_amount: 1,
            usd_price: 1,
            usd_value: 1,
            asset_kind: 'ASSET_KIND_TOKEN',
          },
        ],
      }).balances[0]?.assetKind,
    ).toBe('token');

    expect(
      normalizeSwigTokenTransactions({
        swig_config_address: 'swig_123',
        wallet_address: 'wallet_123',
        transactions: [
          {
            transaction_signature: 'signature_123',
            slot: '42',
            owner_address: 'owner_123',
            wallet_address: 'wallet_123',
            is_subaccount: false,
            token_account_address: 'wallet_123',
            mint_address: '11111111111111111111111111111111',
            token_program: 0,
            direction: 1,
            amount_raw: '1000000000',
            decimals: 9,
            ui_amount: 1,
            usd_price: 100,
            usd_value: 100,
            token_symbol: 'SOL',
            token_name: 'Solana',
            asset_kind: 2,
          },
        ],
      }).transactions[0]?.assetKind,
    ).toBe('native-sol');
  });
});
