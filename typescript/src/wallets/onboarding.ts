import {
  type Connection,
  PublicKey,
  VersionedTransaction,
} from '@solana/web3.js';
import { createPublicKey, randomUUID, verify } from 'node:crypto';
import type { SwigClient } from '../server/typescript/index.js';
import type {
  Network,
  PreparedTransaction,
  SwigRoleAction,
} from '../types/index.js';
import type { WalletTemplate } from '../types/policy.js';
import type { AddRoleAction } from '../types/wallet-actions.js';

export type AgentPermission =
  | { type: 'programAll' }
  | {
      type: 'tokenRecurringLimit';
      mint: string;
      recurringAmount: string;
      window: string;
    }
  | { type: 'solRecurringLimit'; recurringAmount: string; window: string };

export function decodeAgentPermission(
  action: SwigRoleAction,
): AgentPermission | undefined {
  const data = action.actionData;
  const amount = action.actionCode === 2 ? data.recurring_amount : data.limit;
  if (action.actionCode === 13) return { type: 'programAll' };
  if (
    typeof amount !== 'string' ||
    !/^\d+$/.test(amount) ||
    typeof data.window !== 'string' ||
    !/^\d+$/.test(data.window)
  )
    return undefined;
  if (action.actionCode === 2)
    return {
      type: 'solRecurringLimit',
      recurringAmount: amount,
      window: data.window,
    };
  if (action.actionCode === 6 && typeof data.token_mint === 'string')
    return {
      type: 'tokenRecurringLimit',
      mint: data.token_mint,
      recurringAmount: amount,
      window: data.window,
    };
  return undefined;
}

function permission(action: AddRoleAction): AgentPermission {
  if (action.type === 'programAll') return { type: 'programAll' };
  if (
    action.type === 'solRecurringLimit' ||
    action.type === 'tokenRecurringLimit'
  ) {
    for (const value of [action.window, action.recurringAmount]) {
      if (
        !/^[1-9]\d*$/.test(String(value)) ||
        (typeof value === 'number' && !Number.isSafeInteger(value)) ||
        BigInt(value) > 18446744073709551615n
      )
        throw new Error('Invalid recurring limit');
    }
    if (
      action.type === 'tokenRecurringLimit' &&
      new PublicKey(action.mint).toBase58() !== action.mint
    )
      throw new Error('Invalid token mint');
  }
  if (action.type === 'solRecurringLimit')
    return {
      type: action.type,
      recurringAmount: String(action.recurringAmount),
      window: String(action.window),
    };
  if (action.type === 'tokenRecurringLimit')
    return {
      type: action.type,
      mint: action.mint,
      recurringAmount: String(action.recurringAmount),
      window: String(action.window),
    };
  throw new Error('Unsupported agent permission in reserved wallet setup');
}

type Operation = 'create' | 'agent';
type Step =
  | { kind: 'prepare'; operation: Operation }
  | { kind: 'sign'; transaction: string; payer: string }
  | {
      kind: 'submit';
      operation: Operation;
      transaction: string;
      idempotencyKey: string;
    }
  | { kind: 'confirm'; operation: Operation; signature: string }
  | { kind: 'verify' }
  | { kind: 'ready' };

export interface ReservedWalletSetup {
  network: Network;
  owner: string;
  agent: string;
  policyId: string;
  permissions: AgentPermission[];
  walletAddress: string;
  swigConfigAddress: string;
  salt: string;
  step: Step;
}

/** Start from the server-fetched, user-approved template. Only a direct owner and a scoped agent are supported. */
export async function startReservedWalletSetup(
  client: SwigClient,
  args: {
    network: Network;
    owner: string;
    agent: string;
    template: WalletTemplate;
    salt: Uint8Array;
  },
): Promise<ReservedWalletSetup> {
  const { owner, agent, template, network, salt } = args;
  const role = template.additionalRoles[0];
  if (
    !role ||
    template.additionalRoles.length !== 1 ||
    template.rentClaimer ||
    template.recovery ||
    template.initialRole.keySource !== 'atCreation' ||
    template.initialRole.permissions.length !== 1 ||
    template.initialRole.permissions[0]?.type !== 'all' ||
    role.keySource !== 'atCreation' ||
    !role.permissions.length
  )
    throw new Error(
      'Expected an at-creation full-control owner and one scoped at-creation agent',
    );
  if (
    owner === agent ||
    ![owner, agent].every(
      (key) =>
        new PublicKey(key).toBase58() === key &&
        PublicKey.isOnCurve(new PublicKey(key).toBytes()),
    )
  )
    throw new Error('Expected distinct Ed25519 signing keys');
  const permissions = role.permissions.map(permission);
  const reserved = await client.wallets.reserveAddress({
    network,
    rootAuthority: { ed25519: { publicKey: owner } },
    addressOptions: { salt },
  });
  return {
    network,
    owner,
    agent,
    policyId: template.id,
    permissions,
    salt: Buffer.from(salt).toString('hex'),
    walletAddress: reserved.wallet.walletAddress,
    swigConfigAddress: reserved.wallet.swigConfigAddress,
    step: { kind: 'prepare', operation: 'create' },
  };
}

/** Pass only trusted server-side state. Serialize advances per owner. Persist each returned checkpoint
 * before advancing again. In particular, submit checkpoints retain the exact payload/key for replay. */
export async function advanceReservedWalletSetup(
  client: SwigClient,
  rpc: Pick<Connection, 'isBlockhashValid' | 'getSignatureStatuses'>,
  setup: ReservedWalletSetup,
  signedTransaction?: string,
): Promise<ReservedWalletSetup> {
  const { network, owner, agent, step } = setup;
  const wallet = client.wallets.use(setup.swigConfigAddress, {
    network,
    requesterAuthority: { ed25519: { publicKey: owner } },
  });
  if (signedTransaction && step.kind !== 'sign')
    throw new Error('No signature expected at this step');
  switch (step.kind) {
    case 'prepare': {
      const balance = await client.paymaster.getBalance({
        network,
        kind: 'api',
      });
      if (!balance.configured || BigInt(balance.balanceLamports) <= 0n)
        throw new Error('Fund the project API paymaster before continuing');
      const payer = balance.address;
      let prepared: PreparedTransaction | undefined;
      if (step.operation === 'create') {
        const created = await client.wallets.createReserved({
          network,
          rootAuthority: { ed25519: { publicKey: owner } },
          addressOptions: { salt: Buffer.from(setup.salt, 'hex') },
          feePayer: payer,
        });
        if (
          created.wallet.walletAddress !== setup.walletAddress ||
          created.wallet.swigConfigAddress !== setup.swigConfigAddress
        )
          throw new Error('Reservation address mismatch');
        prepared = created.creationTransaction;
      } else
        prepared = await wallet.roles.add({
          network,
          feePayer: payer,
          authority: { ed25519: { publicKey: agent } },
          actions: setup.permissions,
        });
      if (
        !prepared ||
        (prepared.wallet &&
          (prepared.wallet.swigConfigAddress !== setup.swigConfigAddress ||
            prepared.wallet.walletAddress !== setup.walletAddress)) ||
        (prepared.network !== undefined && prepared.network !== network) ||
        prepared.signatureRequests.length ||
        prepared.participantSetApprovalPlan
      )
        throw new Error('Unexpected signing plan');
      const tx = VersionedTransaction.deserialize(
        Buffer.from(prepared.transaction, 'base64'),
      );
      const keys = tx.message.staticAccountKeys.slice(
        0,
        tx.message.header.numRequiredSignatures,
      );
      if (
        keys[0]?.toBase58() !== payer ||
        tx.signatures.some((signature) =>
          signature.some((byte) => byte !== 0),
        ) ||
        (step.operation === 'create'
          ? keys.length !== 1
          : keys.length !== 2 || keys[1]?.toBase58() !== owner)
      )
        throw new Error('Unexpected transaction signers');
      return {
        ...setup,
        step:
          step.operation === 'create'
            ? {
                kind: 'submit',
                operation: 'create',
                transaction: prepared.transaction,
                idempotencyKey: randomUUID(),
              }
            : { kind: 'sign', transaction: prepared.transaction, payer },
      };
    }
    case 'sign': {
      const original = VersionedTransaction.deserialize(
        Buffer.from(step.transaction, 'base64'),
      );
      if (
        !(
          await rpc.isBlockhashValid(original.message.recentBlockhash, {
            commitment: 'confirmed',
          })
        ).value
      )
        return { ...setup, step: { kind: 'prepare', operation: 'agent' } };
      if (!signedTransaction) return setup;
      const signed = VersionedTransaction.deserialize(
        Buffer.from(signedTransaction, 'base64'),
      );
      const bytes = Buffer.from(original.message.serialize());
      const key = createPublicKey({
        key: {
          kty: 'OKP',
          crv: 'Ed25519',
          x: new PublicKey(owner).toBuffer().toString('base64url'),
        },
        format: 'jwk',
      });
      const signature = signed.signatures[1];
      if (
        !Buffer.from(signed.message.serialize()).equals(bytes) ||
        !signature ||
        signed.signatures[0]?.some((byte) => byte !== 0) ||
        !verify(null, bytes, key, signature)
      )
        throw new Error('Owner signature does not match the prepared message');
      return {
        ...setup,
        step: {
          kind: 'submit',
          operation: 'agent',
          transaction: signedTransaction,
          idempotencyKey: randomUUID(),
        },
      };
    }
    case 'submit': {
      const receipt = await client.transactions.sponsor({
        network,
        transaction: step.transaction,
        idempotencyKey: step.idempotencyKey,
      });
      return {
        ...setup,
        step: {
          kind: 'confirm',
          operation: step.operation,
          signature: receipt.signature,
        },
      };
    }
    case 'confirm': {
      const response = await rpc.getSignatureStatuses([step.signature], {
        searchTransactionHistory: true,
      });
      const status = response.value[0];
      if (
        !status ||
        (status.confirmationStatus !== 'confirmed' &&
          status.confirmationStatus !== 'finalized')
      )
        return setup;
      if (status.err)
        throw new Error('Confirmed transaction failed: ' + step.signature);
      return {
        ...setup,
        step:
          step.operation === 'create'
            ? { kind: 'prepare', operation: 'agent' }
            : { kind: 'verify' },
      };
    }
    case 'ready':
    case 'verify': {
      const layout = await wallet.listRoles({ network });
      if (
        layout.swigConfigAddress !== setup.swigConfigAddress ||
        layout.walletAddress !== setup.walletAddress
      )
        throw new Error('Indexed wallet address mismatch');
      if (step.kind === 'verify' && layout.roles.length === 0) return setup;
      const root = layout.roles.find((role) => role.roleId === 0);
      const delegate = layout.roles.find(
        (role) =>
          role.roleId !== 0 &&
          role.authorityValue === agent &&
          role.authorityType === 1,
      );
      if (
        root?.authorityType !== 1 ||
        root.authorityValue !== owner ||
        root.actions.length !== 1 ||
        root.actions[0]?.actionCode !== 7
      )
        throw new Error('Wallet roles differ from the approved policy');
      if (step.kind === 'verify' && layout.roles.length === 1) return setup;
      if (layout.roles.length !== 2 || !delegate)
        throw new Error('Agent role differs from the approved policy');
      const actual = delegate.actions
        .map((action) => JSON.stringify(decodeAgentPermission(action)))
        .sort();
      const expected = setup.permissions
        .map((action) => JSON.stringify(permission(action)))
        .sort();
      if (
        actual.length !== expected.length ||
        actual.some((value, i) => value !== expected[i])
      )
        throw new Error('Agent permissions differ from the approved policy');
      return { ...setup, step: { kind: 'ready' } };
    }
  }
}
