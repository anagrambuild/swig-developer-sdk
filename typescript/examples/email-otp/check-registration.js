export async function verifyRegistration(origin, clientId, agentPublicKey) {
  const response = await fetch(new URL('/attempt', origin), {
    method: 'POST',
    headers: { Origin: origin.origin },
    redirect: 'error',
    signal: AbortSignal.timeout(10000),
  });
  if (!response.ok)
    throw new Error('The email app could not start a login attempt.');
  const attempt = await response.json();
  if (
    attempt.clientId !== clientId ||
    attempt.agentPublicKey !== agentPublicKey ||
    attempt.callbackUrl !== new URL('/auth/callback', origin).href
  )
    throw new Error(
      'The app endpoint does not match this registered client, callback and agent.',
    );
}
