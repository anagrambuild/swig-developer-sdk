import { type HttpClient } from '../core/http.js';
import {
  type WalletTemplate,
  type WalletTemplateInput,
} from '../types/policy.js';
import { record, text, walletRequest, walletTemplate } from './codec.js';

export class WalletTemplatesClient {
  constructor(private readonly http: HttpClient) {}
  create = async (
    input: WalletTemplateInput,
  ): Promise<{ walletTemplateId: string }> => {
    const response = await this.http.post<unknown>(
      '/wallet/api/wallet-templates',
      walletRequest(input),
    );
    if (!record(response)) throw new Error('Invalid wallet template response');
    return {
      walletTemplateId: text(response.walletTemplateId, 'walletTemplateId'),
    };
  };
  update = async (
    id: string,
    input: WalletTemplateInput,
  ): Promise<{ walletTemplateId: string }> => {
    const response = await this.http.patch<unknown>(
      `/wallet/api/wallet-templates/${encodeURIComponent(text(id, 'id'))}`,
      walletRequest(input),
    );
    if (!record(response)) throw new Error('Invalid wallet template response');
    const returnedId = text(response.walletTemplateId, 'walletTemplateId');
    if (returnedId !== id)
      throw new Error('Wallet template response ID mismatch');
    return { walletTemplateId: returnedId };
  };
  get = async (id: string): Promise<WalletTemplate> => {
    const response = await this.http.get<unknown>(
      `/wallet/api/wallet-templates/${encodeURIComponent(text(id, 'id'))}`,
    );
    if (!record(response)) throw new Error('Invalid wallet template response');
    const wallet = walletTemplate(response.walletTemplate);
    if (wallet.id !== id)
      throw new Error('Wallet template response ID mismatch');
    return wallet;
  };
  list = async (): Promise<WalletTemplate[]> => {
    const response = await this.http.get<unknown>(
      '/wallet/api/wallet-templates',
    );
    if (!record(response)) throw new Error('Invalid wallet templates response');
    const rows = response.walletTemplates ?? [];
    if (!Array.isArray(rows))
      throw new Error('Invalid wallet templates response');
    return rows.map(walletTemplate);
  };
}
