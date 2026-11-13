import type {
  AddRoleAction,
  DirectWalletAuthority,
  WalletAuthority,
} from './wallet-actions.js';

export type PolicyAuthority =
  | { type: 'Ed25519'; publicKey: string }
  | { type: 'Secp256k1'; publicKey: string }
  | { type: 'Secp256r1'; publicKey: string }
  | { type: 'Ed25519Session'; publicKey: string; maxDurationSlots: string }
  | { type: 'Secp256k1Session'; publicKey: string; maxDurationSlots: string }
  | { type: 'Secp256r1Session'; publicKey: string; maxDurationSlots: string };

export interface Policy {
  id: string;
  name: string;
  description: string | null;
  authority?: PolicyAuthority | WalletAuthority | null;
  actions?: unknown[];
  guardianEnabled?: boolean;
  initialKeySource?: string;
  guardianAuthority?: PolicyAuthority | WalletAuthority | null;
  guardianKeySource?: string;
  guardianDelaySeconds?: number | string;
  initialUserLabel?: string;
  additionalUsers?: PolicyUser[];
}

export interface PolicyUser {
  id: string;
  label: string;
  authority?: DirectWalletAuthority;
  actions: unknown[];
}

export interface RoleTemplateInput {
  name: string;
  permissions: AddRoleAction[];
  /** Omit to bind a public key when creating a wallet. */
  authority?: DirectWalletAuthority;
}

export type TemplateAuthority =
  | { keySource: 'atCreation'; authority?: never }
  | { keySource: 'stored'; authority: DirectWalletAuthority };

export type RoleTemplate = {
  id: string;
  name: string;
  permissions: AddRoleAction[];
} & TemplateAuthority;

export type TemplateRecovery = {
  label?: string;
  delaySeconds: number;
} & TemplateAuthority;

export interface WalletTemplateInput {
  name: string;
  description?: string;
  initialRoleTemplateId: string;
  additionalRoleTemplateIds?: string[];
  rentClaimer?: string;
  recovery?: TemplateRecovery;
}

export interface WalletTemplate {
  id: string;
  name: string;
  description: string;
  initialRole: RoleTemplate;
  additionalRoles: RoleTemplate[];
  rentClaimer?: string;
  recovery?: TemplateRecovery;
}

export interface RoleAuthorityBinding {
  roleTemplateId: string;
  authority: DirectWalletAuthority;
}

/** Remaining setup after the creation transaction confirms; these are not on-chain roles yet. */
export interface WalletTemplateSetup {
  template: WalletTemplate;
  authorities: RoleAuthorityBinding[];
  guardianAuthority?: DirectWalletAuthority;
}
