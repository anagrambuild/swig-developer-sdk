export async function verifyWalletEmail(email, input, agentPublicKey) {
  // Reserve the window while the Verify email click still has browser activation.
  const popup = window.open('about:blank', '_blank', 'popup,width=480,height=720');
  try {
    if (popup) {
      popup.document.title = 'Swig wallet';
      popup.document.body.textContent = 'Verifying your email…';
    }
    const result = await email.verify(input);
    if (result.status === 'complete') return result;
    const wallet = result.wallet;
    if (
      !wallet || wallet.agentPublicKey !== agentPublicKey ||
      wallet.agentPolicy.mint !== '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU'
    ) {
      email.cancel();
      throw new Error('The wallet approval does not match the registered agent or Devnet USDC');
    }
    if (!popup) return result;
    return await email.approve({ challengeId: input.challengeId }, { approvalWindow: popup });
  } finally {
    popup?.close();
  }
}
