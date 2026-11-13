import type {
  RoleTemplate,
  RoleTemplateInput,
  TemplateAuthority,
  TemplateRecovery,
  WalletTemplate,
  WalletTemplateInput,
  WalletTemplateSetup,
} from '../types/policy.js';
import type {
  AddRoleAction,
  DirectWalletAuthority,
} from '../types/wallet-actions.js';
export function storedAction(action: AddRoleAction): Record<string, unknown> {
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

export function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function policyAuthority(value: unknown): DirectWalletAuthority {
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

export function text(value: unknown, field: string): string {
  if (typeof value !== 'string' || !value.trim())
    throw new Error(`Invalid template ${field}`);
  return value;
}

function amount(value: unknown): string {
  if (
    typeof value !== 'string' ||
    !/^(0|[1-9][0-9]*)$/.test(value) ||
    BigInt(value) > 18446744073709551615n
  )
    throw new Error('Invalid template amount');
  return value;
}

function permission(value: unknown): AddRoleAction {
  if (!record(value)) throw new Error('Invalid template permission');
  switch (value.type) {
    case 'All':
      return { type: 'all' };
    case 'AllButManageAuthority':
      return { type: 'allButManageAuthority' };
    case 'ManageAuthority':
      return { type: 'manageAuthority' };
    case 'Program':
      return { type: 'program', programId: text(value.programId, 'programId') };
    case 'ProgramAll':
      return { type: 'programAll' };
    case 'ProgramCurated':
      return { type: 'programCurated' };
    case 'StakeAll':
      return { type: 'stakeAll' };
    case 'SubAccount':
      return { type: 'subAccount' };
    case 'SolLimit':
      return { type: 'solLimit', amount: amount(value.amount) };
    case 'SolRecurringLimit':
      return {
        type: 'solRecurringLimit',
        recurringAmount: amount(value.recurringAmount),
        window: amount(value.window),
      };
    case 'SolDestinationLimit':
      return {
        type: 'solDestinationLimit',
        amount: amount(value.amount),
        destination: text(value.destination, 'destination'),
      };
    case 'SolRecurringDestinationLimit':
      return {
        type: 'solRecurringDestinationLimit',
        recurringAmount: amount(value.recurringAmount),
        window: amount(value.window),
        destination: text(value.destination, 'destination'),
      };
    case 'TokenLimit':
      return {
        type: 'tokenLimit',
        mint: text(value.mint, 'mint'),
        amount: amount(value.amount),
      };
    case 'TokenRecurringLimit':
      return {
        type: 'tokenRecurringLimit',
        mint: text(value.mint, 'mint'),
        recurringAmount: amount(value.recurringAmount),
        window: amount(value.window),
      };
    case 'TokenDestinationLimit':
      return {
        type: 'tokenDestinationLimit',
        mint: text(value.mint, 'mint'),
        amount: amount(value.amount),
        destination: text(value.destination, 'destination'),
      };
    case 'TokenRecurringDestinationLimit':
      return {
        type: 'tokenRecurringDestinationLimit',
        mint: text(value.mint, 'mint'),
        recurringAmount: amount(value.recurringAmount),
        window: amount(value.window),
        destination: text(value.destination, 'destination'),
      };
    case 'StakeLimit':
      return { type: 'stakeLimit', amount: amount(value.amount) };
    case 'StakeRecurringLimit':
      return {
        type: 'stakeRecurringLimit',
        recurringAmount: amount(value.recurringAmount),
        window: amount(value.window),
      };
    default:
      throw new Error('Unknown template permission');
  }
}

function templateAuthority(value: Record<string, unknown>): TemplateAuthority {
  if (
    value.keySource === 'WALLET_POLICY_KEY_SOURCE_AT_CREATION' &&
    value.authority == null
  )
    return { keySource: 'atCreation' };
  if (
    value.keySource === 'WALLET_POLICY_KEY_SOURCE_STORED' &&
    value.authority != null
  )
    return { keySource: 'stored', authority: policyAuthority(value.authority) };
  throw new Error('Invalid template authority source');
}

export function roleTemplate(value: unknown): RoleTemplate {
  if (
    !record(value) ||
    !Array.isArray(value.permissions) ||
    !value.permissions.length
  )
    throw new Error('Invalid role template');
  return {
    id: text(value.id, 'id'),
    name: text(value.name, 'name'),
    permissions: value.permissions.map(permission),
    ...templateAuthority(value),
  };
}

function recovery(value: unknown): TemplateRecovery {
  if (
    !record(value) ||
    typeof value.delaySeconds !== 'number' ||
    !Number.isSafeInteger(value.delaySeconds) ||
    value.delaySeconds < 86_400 ||
    value.delaySeconds > 2147483647
  )
    throw new Error('Invalid recovery settings');
  const source = templateAuthority(value);
  if (source.keySource === 'stored' && !('ed25519' in source.authority))
    throw new Error('Guardian must be Ed25519');
  if (value.label !== undefined && typeof value.label !== 'string')
    throw new Error('Invalid recovery label');
  return {
    ...source,
    delaySeconds: value.delaySeconds,
    ...(typeof value.label === 'string' ? { label: value.label } : {}),
  };
}

export function walletTemplate(value: unknown): WalletTemplate {
  if (!record(value)) throw new Error('Invalid wallet template');
  const initialRole = roleTemplate(value.initialRole);
  const additional = value.additionalRoles ?? [];
  if (!Array.isArray(additional)) throw new Error('Invalid template roles');
  const additionalRoles = additional.map(roleTemplate);
  const ids = [initialRole.id, ...additionalRoles.map((role) => role.id)];
  if (new Set(ids).size !== ids.length)
    throw new Error('Duplicate template role');
  if (value.description !== undefined && typeof value.description !== 'string')
    throw new Error('Invalid template description');
  return {
    id: text(value.id, 'id'),
    name: text(value.name, 'name'),
    description: value.description ?? '',
    initialRole,
    additionalRoles,
    ...(value.rentClaimer != null
      ? { rentClaimer: text(value.rentClaimer, 'rentClaimer') }
      : {}),
    ...(value.recovery != null ? { recovery: recovery(value.recovery) } : {}),
  };
}

export function roleRequest(input: RoleTemplateInput) {
  text(input.name, 'name');
  if (!input.permissions.length)
    throw new Error('Role permissions are required');
  return {
    name: input.name,
    permissions: input.permissions.map(storedAction),
    ...(input.authority ? { authority: policyAuthority(input.authority) } : {}),
  };
}

export function walletRequest(input: WalletTemplateInput) {
  const ids = [
    text(input.initialRoleTemplateId, 'initialRoleTemplateId'),
    ...(input.additionalRoleTemplateIds ?? []).map((id) =>
      text(id, 'roleTemplateId'),
    ),
  ];
  if (new Set(ids).size !== ids.length)
    throw new Error('Duplicate template role');
  const wireRecovery = input.recovery
    ? {
        ...input.recovery,
        keySource:
          input.recovery.keySource === 'stored'
            ? 'WALLET_POLICY_KEY_SOURCE_STORED'
            : 'WALLET_POLICY_KEY_SOURCE_AT_CREATION',
      }
    : undefined;
  if (wireRecovery) recovery(wireRecovery);
  return {
    name: text(input.name, 'name'),
    description: input.description,
    initialRoleTemplateId: input.initialRoleTemplateId,
    additionalRoleTemplateIds: input.additionalRoleTemplateIds ?? [],
    rentClaimer: input.rentClaimer,
    recovery: wireRecovery,
  };
}

export function templateSetup(value: unknown): WalletTemplateSetup {
  if (!record(value) || !Array.isArray(value.authorities))
    throw new Error('Invalid wallet template setup');
  const template = walletTemplate(value.template);
  const authorities = value.authorities.map((binding: unknown) => {
    if (!record(binding)) throw new Error('Invalid role authority binding');
    return {
      roleTemplateId: text(binding.roleTemplateId, 'roleTemplateId'),
      authority: policyAuthority(binding.authority),
    };
  });
  const roleIds = [
    template.initialRole.id,
    ...template.additionalRoles.map((role) => role.id),
  ];
  if (
    authorities.length !== roleIds.length ||
    new Set(authorities.map((binding) => binding.roleTemplateId)).size !==
      roleIds.length ||
    authorities.some((binding) => !roleIds.includes(binding.roleTemplateId))
  )
    throw new Error('Template setup is missing or has extra role authorities');
  for (const role of [template.initialRole, ...template.additionalRoles]) {
    const bound = authorities.find(
      (binding) => binding.roleTemplateId === role.id,
    );
    if (
      role.keySource === 'stored' &&
      JSON.stringify(role.authority) !== JSON.stringify(bound?.authority)
    )
      throw new Error('Template setup changed a stored authority');
  }
  const guardianAuthority =
    value.guardianAuthority == null
      ? undefined
      : policyAuthority(value.guardianAuthority);
  if (!!template.recovery !== !!guardianAuthority)
    throw new Error('Template setup guardian does not match recovery');
  if (
    template.recovery?.keySource === 'stored' &&
    JSON.stringify(template.recovery.authority) !==
      JSON.stringify(guardianAuthority)
  )
    throw new Error('Template setup changed the stored guardian');
  return {
    template,
    authorities,
    ...(guardianAuthority ? { guardianAuthority } : {}),
  };
}
