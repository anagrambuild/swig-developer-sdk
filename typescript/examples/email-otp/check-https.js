import { appOrigin } from './origin.js';

const origin = appOrigin(process.env.SWIG_APP_ORIGIN);
const response = await fetch(new URL('/attempt', origin), {
  method: 'POST',
  headers: { Origin: origin.origin },
  redirect: 'error',
  signal: AbortSignal.timeout(10000),
});
if (!response.ok)
  throw new Error('The email app could not start an HTTPS login attempt.');
const attempt = await response.json();
if (
  !attempt.clientId ||
  attempt.callbackUrl !== new URL('/auth/callback', origin).href
)
  throw new Error(
    'The registered callback does not match the HTTPS app origin.',
  );
console.log('Ready for email sign-in at', origin.origin);
