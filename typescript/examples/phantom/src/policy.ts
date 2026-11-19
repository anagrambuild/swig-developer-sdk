import type { AddRoleAction, WalletTemplate } from '@swig-wallet/developer-sdk';

export const usdcMint = '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU';
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
  readAgentAllowance(agent.permissions);
}

export function readAgentAllowance(permissions: AddRoleAction[]) {
  const program = permissions.filter((a) => a.type === 'programAll');
  const token = permissions.filter((a) => a.type === 'tokenRecurringLimit');
  const sol = permissions.filter((a) => a.type === 'solRecurringLimit');
  const tokenLimit = token[0];
  const solLimit = sol[0];
  if (
    permissions.length !== 3 ||
    program.length !== 1 ||
    !tokenLimit ||
    !solLimit ||
    tokenLimit.mint !== usdcMint ||
    String(tokenLimit.window) !== String(solLimit.window)
  )
    throw new Error(
      'This starter requires any-program access with recurring Devnet USDC and SOL limits over the same window.',
    );
  const values = [
    tokenLimit.recurringAmount,
    solLimit.recurringAmount,
    solLimit.window,
  ];
  for (const value of values) {
    if (
      (typeof value === 'number' && !Number.isSafeInteger(value)) ||
      !/^[1-9]\d{0,19}$/.test(String(value)) ||
      BigInt(value) > 18446744073709551615n
    )
      throw new Error(
        'Invalid recurring allowance in the saved wallet policy.',
      );
  }
  return {
    token: String(tokenLimit.recurringAmount),
    sol: String(solLimit.recurringAmount),
    window: String(solLimit.window),
  };
}
