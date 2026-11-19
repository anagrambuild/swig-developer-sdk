import { createSwigWebClient, Network } from '@swig-wallet/expo-idp-sdk/web';
import { verifyWalletEmail } from './verify.js';

const email = createSwigWebClient({
  redirectUri: location.origin + '/auth/callback',
  network: Network.Devnet,
}).auth.email;
const start = document.querySelector('#start'),
  verify = document.querySelector('#verify'),
  approve = document.querySelector('#approve');
const status = document.querySelector('#status'),
  preview = document.querySelector('#preview'),
  restart = document.querySelector('#restart');
let challenge, app;
restart.onclick = () => {
  email.cancel();
  location.reload();
};
async function run(button, operation) {
  button.disabled = true;
  try {
    await operation();
  } catch (error) {
    status.textContent =
      error instanceof Error ? error.message : 'Email sign-in failed';
    if (button === approve && error?.code !== 'popup_blocked')
      approve.hidden = true;
    if (button.form === verify && error?.code !== 'invalid_code')
      verify.hidden = true;
    restart.hidden = false;
  } finally {
    button.disabled = false;
  }
}
start.onsubmit = (event) => {
  event.preventDefault();
  run(start.querySelector('button'), async () => {
    const response = await fetch('/attempt', { method: 'POST' });
    if (!response.ok) throw new Error('Could not start sign-in');
    app = await response.json();
    challenge = await email.start({
      clientId: app.clientId,
      email: document.querySelector('#email').value.trim(),
      state: app.state,
      oidcNonce: app.oidcNonce,
    });
    status.textContent = 'Code sent to ' + challenge.maskedRecipient;
    start.hidden = true;
    verify.hidden = false;
    document.querySelector('#code').focus();
  });
};
verify.onsubmit = (event) => {
  event.preventDefault();
  run(verify.querySelector('button'), async () => {
    restart.hidden = true;
    status.textContent = 'Verifying email…';
    const result = await verifyWalletEmail(
      email,
      {
        challengeId: challenge.challengeId,
        code: document.querySelector('#code').value,
      },
      app.agentPublicKey,
      () => {
        verify.hidden = true;
        document.querySelector('#code').value = '';
        status.textContent =
          'Email verified. Complete wallet approval in Swig.';
      },
    );
    if (result.status === 'complete') {
      await finish(result);
      return;
    }
    status.textContent = 'Open the Swig window to review wallet access.';
    approve.hidden = false;
    approve.focus();
  });
};
approve.onclick = () =>
  run(approve, async () =>
    finish(await email.approve({ challengeId: challenge.challengeId })),
  );
async function finish(result) {
  const url = new URL(result.callbackUrl);
  const response = await fetch('/complete', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      idToken: new URLSearchParams(url.hash.slice(1)).get('id_token'),
      state: url.searchParams.get('state'),
    }),
  });
  if (!response.ok)
    throw new Error('Email session verification failed. Start again.');
  verify.hidden = true;
  approve.hidden = true;
  restart.hidden = true;
  logout.hidden = false;
  preview.textContent =
    'Devnet wallet ' + url.searchParams.get('wallet_address');
  status.textContent =
    url.searchParams.get('agent_access') === 'review_existing_role'
      ? 'Signed in. Review the existing wallet permissions before using the agent.'
      : 'Signed in · Agent role ' + url.searchParams.get('role_id');
}

const logout = document.querySelector('#logout');
logout.onclick = () =>
  run(logout, async () => {
    if (!(await fetch('/logout', { method: 'POST' })).ok)
      throw new Error('Could not sign out');
    location.reload();
  });
fetch('/session')
  .then((response) => (response.ok ? response.json() : null))
  .then((session) => {
    if (session?.authenticated) {
      start.hidden = true;
      logout.hidden = false;
      status.textContent = 'Signed in';
    }
  })
  .catch(() => {
    status.textContent = 'Could not read session status';
  });
