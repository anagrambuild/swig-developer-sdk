function units(amount, decimals) {
  const digits = amount.padStart(decimals + 1, '0');
  const fraction = digits.slice(-decimals).replace(/0+$/, '');
  return digits.slice(0, -decimals) + (fraction ? '.' + fraction : '');
}
export function describeGrant(wallet) {
  const { mint, tokenAmount, solAmount, window } = wallet.agentPolicy;
  if (mint !== '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU')
    throw new Error('Unexpected Devnet USDC mint');
  return (
    'Wallet ' +
    wallet.walletAddress +
    '\nAgent ' +
    wallet.agentPublicKey +
    '\nAny program, including transfers. Up to ' +
    units(tokenAmount, 6) +
    ' Devnet USDC (' +
    mint +
    ') and ' +
    units(solAmount, 9) +
    ' SOL per ' +
    window +
    ' slots' +
    (window === '216000' ? ' (about one day).' : '.')
  );
}
