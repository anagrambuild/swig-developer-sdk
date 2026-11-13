import { type HttpClient } from '../core/http.js';
import { type RoleTemplate, type RoleTemplateInput } from '../types/policy.js';
import { record, roleRequest, roleTemplate, text } from './codec.js';

export class RoleTemplatesClient {
  constructor(private readonly http: HttpClient) {}
  create = async (
    input: RoleTemplateInput,
  ): Promise<{ roleTemplateId: string }> => {
    const response = await this.http.post<unknown>(
      '/wallet/api/role-templates',
      roleRequest(input),
    );
    if (!record(response)) throw new Error('Invalid role template response');
    return { roleTemplateId: text(response.roleTemplateId, 'roleTemplateId') };
  };
  update = async (
    id: string,
    input: RoleTemplateInput,
  ): Promise<{ roleTemplateId: string }> => {
    const response = await this.http.patch<unknown>(
      `/wallet/api/role-templates/${encodeURIComponent(text(id, 'id'))}`,
      roleRequest(input),
    );
    if (!record(response)) throw new Error('Invalid role template response');
    const returnedId = text(response.roleTemplateId, 'roleTemplateId');
    if (returnedId !== id)
      throw new Error('Role template response ID mismatch');
    return { roleTemplateId: returnedId };
  };
  get = async (id: string): Promise<RoleTemplate> => {
    const response = await this.http.get<unknown>(
      `/wallet/api/role-templates/${encodeURIComponent(text(id, 'id'))}`,
    );
    if (!record(response)) throw new Error('Invalid role template response');
    const role = roleTemplate(response.roleTemplate);
    if (role.id !== id) throw new Error('Role template response ID mismatch');
    return role;
  };
  list = async (): Promise<RoleTemplate[]> => {
    const response = await this.http.get<unknown>('/wallet/api/role-templates');
    if (!record(response)) throw new Error('Invalid role templates response');
    const rows = response.roleTemplates ?? [];
    if (!Array.isArray(rows))
      throw new Error('Invalid role templates response');
    return rows.map(roleTemplate);
  };
}
