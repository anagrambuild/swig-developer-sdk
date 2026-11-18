import { PublicKey } from '@solana/web3.js';
import type { HttpClient } from '../core/index.js';

export interface EnsureEmailWalletAppInput {
  network: 'devnet';
  callbackUrl: string;
  walletTemplateId: string;
  agentPublicKey: string;
  /** Existing API paymaster of this project. Omit to use the dedicated IdP paymaster. */
  projectPaymasterAddress?: string;
}

export interface EmailWalletApp {
  clientId: string;
  callbackUrl: string;
  walletTemplateId: string;
  agentPublicKey: string;
  /** Existing API paymaster of this project. Omit to use the dedicated IdP paymaster. */
  projectPaymasterAddress?: string;
}

export class IdpClient {
  constructor(private readonly http: HttpClient) {}

  async ensureEmailWalletApp(
    input: EnsureEmailWalletAppInput,
  ): Promise<EmailWalletApp> {
    if (input.network !== 'devnet')
      throw new Error('Select Devnet for this email wallet app');
    const key = new PublicKey(input.agentPublicKey);
    if (
      key.toBase58() !== input.agentPublicKey ||
      !PublicKey.isOnCurve(key.toBytes())
    )
      throw new Error('Expected an Ed25519 agent signing key');
    const { projectPaymasterAddress } = input;
    if (
      projectPaymasterAddress !== undefined &&
      new PublicKey(projectPaymasterAddress).toBase58() !==
        projectPaymasterAddress
    )
      throw new Error('Expected a project paymaster address');
    const result = await this.http.post<unknown>(
      '/identity/api/email-wallet-apps/ensure',
      {
        ...input,
        network: 'NETWORK_DEVNET',
      },
      { retry: true },
    );
    if (
      typeof result !== 'object' ||
      !result ||
      !('clientId' in result) ||
      typeof result.clientId !== 'string' ||
      !result.clientId ||
      !('agentPublicKey' in result) ||
      result.agentPublicKey !== input.agentPublicKey ||
      !('walletTemplateId' in result) ||
      result.walletTemplateId !== input.walletTemplateId ||
      ('projectPaymasterAddress' in result
        ? result.projectPaymasterAddress
        : undefined) !== projectPaymasterAddress ||
      !('callbackUrl' in result) ||
      result.callbackUrl !== input.callbackUrl
    )
      throw new Error(
        'Email wallet registration does not match the requested app',
      );
    return {
      clientId: result.clientId,
      callbackUrl: input.callbackUrl,
      walletTemplateId: input.walletTemplateId,
      agentPublicKey: input.agentPublicKey,
      ...(projectPaymasterAddress !== undefined && { projectPaymasterAddress }),
    };
  }
}
