import type { WalletTemplate } from '@swig-wallet/developer-sdk';

export const usdcMint = '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU';
// This app is intentionally scoped to the policy the developer approved.
// A changed template must not silently change what this screen asks users to grant.
export function validateTemplate(
  template: WalletTemplate,
  policyId: string,
): void {
  const agent = template.additionalRoles[0];
  if (
    template.id !== policyId ||
    template.initialRole.keySource !== 'atCreation' ||
    template.initialRole.permissions.length !== 1 ||
    template.initialRole.permissions[0]?.type !== 'all' ||
    template.additionalRoles.length !== 1 ||
    !agent ||
    agent.keySource !== 'atCreation' ||
    template.rentClaimer ||
    template.recovery ||
    agent.permissions.length !== 3
  ) {
    throw new Error(
      'The saved policy has changed. Review it before creating another wallet.',
    );
  }
  const program = agent.permissions.filter((a) => a.type === 'programAll');
  const token = agent.permissions.filter(
    (a) => a.type === 'tokenRecurringLimit',
  );
  const sol = agent.permissions.filter((a) => a.type === 'solRecurringLimit');
  if (
    program.length !== 1 ||
    token.length !== 1 ||
    sol.length !== 1 ||
    token[0]?.mint !== usdcMint ||
    String(token[0]?.recurringAmount) !== '100000000' ||
    String(token[0]?.window) !== '216000' ||
    String(sol[0]?.recurringAmount) !== '1000000000' ||
    String(sol[0]?.window) !== '216000'
  ) {
    throw new Error(
      'The saved agent permissions differ from the approved 100 USDC and 1 SOL policy.',
    );
  }
}
