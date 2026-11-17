const walletAvailability = {
  Installed: 'Detected · disconnected',
  NotDetected: 'Extension not detected',
  Unsupported: 'Unavailable in this browser',
  Loadable: 'Open in the Phantom browser',
};

export function phantomConnectionLabel(
  readiness: keyof typeof walletAvailability,
  address: string | null,
): string {
  return address ? 'Connected' : walletAvailability[readiness];
}

const walletActions = new Map([
  ['WalletConnectionError', 'Connection'],
  ['WalletSignMessageError', 'Sign-in'],
  ['WalletSignTransactionError', 'Agent role approval'],
]);

export function requestErrorMessage(error: unknown): string {
  if (!(error instanceof Error))
    return 'The request did not complete. Try again.';

  const action = walletActions.get(error.name);
  if (!action)
    return error.message || 'The request did not complete. Try again.';

  if (/chain id does not match|chain id mismatch/i.test(error.message))
    return `${action} needs Solana Devnet. In Phantom, open Settings → Developer Settings, enable Testnet Mode, choose Solana Devnet, then retry.`;

  const provider = 'error' in error ? error.error : undefined;
  const declined =
    (typeof provider === 'object' &&
      provider !== null &&
      'code' in provider &&
      provider.code === 4001) ||
    /user rejected|user denied|user cancelled/i.test(error.message);
  if (declined)
    return `${action} was cancelled in Phantom. Retry when you’re ready.`;
  return `${action} failed in Phantom. ${error.message || 'Open the wallet and retry.'}`;
}
