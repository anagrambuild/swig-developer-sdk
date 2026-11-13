import type {
  AddRoleAction,
  CreatePolicyArgs,
  CreatePolicyResult,
  DirectWalletAuthority,
  Policy,
  PolicyUser,
} from '../types/index.js';

export function createPolicyRequest(args: CreatePolicyArgs) {
  if (!args.name.trim()) throw new Error('Policy name is required');
  if (!args.initialActions.length)
    throw new Error('Initial actions are required');
  if (
    args.initialUser.keySource !== 'atCreation' &&
    args.initialUser.keySource !== 'stored'
  )
    throw new Error('Invalid policy key source');
  if (
    args.initialUser.keySource === 'atCreation' &&
    args.initialUser.authority !== undefined
  )
    throw new Error(
      'At-creation authority must be supplied when creating the wallet',
    );
  if (args.initialUser.keySource === 'stored')
    policyAuthority(args.initialUser.authority);
  return {
    name: args.name,
    description: args.description,
    initialUser: {
      label: args.initialUser.label,
      keySource:
        args.initialUser.keySource === 'atCreation'
          ? 'WALLET_POLICY_KEY_SOURCE_AT_CREATION'
          : 'WALLET_POLICY_KEY_SOURCE_STORED',
      authority: args.initialUser.authority,
    },
    initialActions: args.initialActions.map(storedAction),
    additionalUsers: (args.additionalUsers ?? []).map((user) => {
      policyAuthority(user.authority);
      if (!user.actions.length)
        throw new Error('Additional user actions are required');
      return {
        label: user.label,
        authority: user.authority,
        actions: user.actions.map(storedAction),
      };
    }),
  };
}

function storedAction(action: AddRoleAction): Record<string, unknown> {
  const result: Record<string, unknown> = {
    ...action,
    type: action.type.charAt(0).toUpperCase() + action.type.slice(1),
  };
  for (const field of ['amount', 'recurringAmount', 'window']) {
    const value = result[field];
    if (value !== undefined) {
      if (
        typeof value === 'number' &&
        (!Number.isSafeInteger(value) || value < 0)
      )
        throw new Error(`Invalid policy ${field}`);
      if (
        typeof value !== 'string' &&
        typeof value !== 'number' &&
        typeof value !== 'bigint'
      )
        throw new Error(`Invalid policy ${field}`);
      const encoded = String(value);
      if (
        !/^(0|[1-9][0-9]*)$/.test(encoded) ||
        BigInt(encoded) > 18446744073709551615n
      )
        throw new Error(`Invalid policy ${field}`);
      result[field] = encoded;
    }
  }
  return result;
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function policyAuthority(value: unknown): DirectWalletAuthority {
  if (!record(value) || Object.keys(value).length !== 1)
    throw new Error('Invalid policy authority');
  for (const kind of ['ed25519', 'secp256k1', 'secp256r1'] as const) {
    const key = value[kind];
    if (
      record(key) &&
      typeof key.publicKey === 'string' &&
      key.publicKey.length > 0
    ) {
      const authority = { publicKey: key.publicKey };
      if (kind === 'ed25519') return { ed25519: authority };
      if (kind === 'secp256k1') return { secp256k1: authority };
      return { secp256r1: authority };
    }
  }
  throw new Error('Invalid policy authority');
}

function actions(value: unknown): unknown[] {
  if (
    !Array.isArray(value) ||
    !value.length ||
    !value.every(
      (action: unknown) => record(action) && typeof action.type === 'string',
    )
  )
    throw new Error('Invalid saved policy actions');
  return value;
}

export function normalizePolicyList(response: unknown): Policy[] {
  if (!record(response) || !Array.isArray(response.policies))
    throw new Error('Invalid policy list response');
  return response.policies.map((value: unknown): Policy => {
    if (
      !record(value) ||
      typeof value.id !== 'string' ||
      !value.id ||
      typeof value.name !== 'string' ||
      !value.name ||
      typeof value.description !== 'string' ||
      typeof value.initialUserLabel !== 'string'
    )
      throw new Error('Invalid saved policy');
    if (
      value.initialKeySource !== 'WALLET_POLICY_KEY_SOURCE_AT_CREATION' &&
      value.initialKeySource !== 'WALLET_POLICY_KEY_SOURCE_STORED'
    )
      throw new Error('Invalid policy key source');
    const authority =
      value.initialAuthority == null
        ? undefined
        : policyAuthority(value.initialAuthority);
    if (
      value.initialKeySource === 'WALLET_POLICY_KEY_SOURCE_STORED' &&
      !authority
    )
      throw new Error('Stored policy authority is missing');
    if (!Array.isArray(value.additionalUsers))
      throw new Error('Invalid saved policy users');
    const additionalUsers = value.additionalUsers.map(
      (user: unknown): PolicyUser => {
        if (
          !record(user) ||
          typeof user.id !== 'string' ||
          !user.id ||
          typeof user.label !== 'string'
        )
          throw new Error('Invalid saved policy user');
        return {
          id: user.id,
          label: user.label,
          authority: policyAuthority(user.authority),
          actions: actions(user.actions),
        };
      },
    );
    if (
      typeof value.guardianEnabled !== 'boolean' ||
      typeof value.guardianDelaySeconds !== 'number' ||
      !Number.isSafeInteger(value.guardianDelaySeconds) ||
      value.guardianDelaySeconds < 0 ||
      typeof value.guardianKeySource !== 'string'
    )
      throw new Error('Invalid policy guardian metadata');
    return {
      id: value.id,
      name: value.name,
      description: value.description,
      initialUserLabel: value.initialUserLabel,
      initialKeySource: value.initialKeySource,
      authority,
      actions: actions(value.initialActions),
      additionalUsers,
      guardianEnabled: value.guardianEnabled,
      guardianKeySource: value.guardianKeySource,
      guardianAuthority:
        value.guardianAuthority == null
          ? undefined
          : policyAuthority(value.guardianAuthority),
      guardianDelaySeconds: value.guardianDelaySeconds,
    };
  });
}

export function normalizeCreatedPolicy(response: unknown): CreatePolicyResult {
  if (
    !record(response) ||
    typeof response.policyId !== 'string' ||
    !response.policyId
  )
    throw new Error('Invalid policy creation response');
  return { policyId: response.policyId };
}
