import assert from 'node:assert/strict';
import test from 'node:test';
import {
  phantomConnectionLabel,
  phantomSetupMessage,
  requestErrorMessage,
} from '../src/phantom-feedback';

test('a Phantom chain mismatch gives the Devnet recovery steps', () => {
  const mismatch = new Error(
    "The app's signature request cannot be shown as the chain ID does not match the provided chain ID for verification.",
  );
  mismatch.name = 'WalletSignMessageError';
  const message = requestErrorMessage(mismatch);
  assert.match(message, /Sign-in needs Solana Devnet/);
  assert.match(
    message,
    /Developer Settings.*Testnet Mode.*Solana Devnet.*retry/,
  );
});
test('declining the later transaction is distinguished from a login error', () => {
  const declined = Object.assign(new Error('Request declined'), {
    name: 'WalletSignTransactionError',
    error: { code: 4001 },
  });
  assert.match(
    requestErrorMessage(declined),
    /Agent role approval was cancelled/,
  );
  assert.equal(
    requestErrorMessage(new Error('Login proof expired')),
    'Login proof expired',
  );
});

test('wallet status distinguishes extension detection from an account connection', () => {
  assert.equal(
    phantomConnectionLabel('NotDetected', null),
    'Extension not detected',
  );
  assert.equal(
    phantomConnectionLabel('Installed', null),
    'Detected · disconnected',
  );
  assert.equal(
    phantomConnectionLabel('Installed', 'connected-address'),
    'Connected',
  );
  assert.equal(
    phantomConnectionLabel('Unsupported', null),
    'Unavailable in this browser',
  );
  assert.equal(
    phantomConnectionLabel('Loadable', null),
    'Desktop extension required',
  );
});

test('the action explains the desktop requirement on mobile and unsupported browsers', () => {
  for (const state of ['Loadable', 'Unsupported'] as const) {
    assert.match(
      phantomSetupMessage(state) ?? '',
      /computer running it.*desktop browser with the Phantom extension/,
    );
  }
  assert.match(
    phantomSetupMessage('NotDetected') ?? '',
    /Install the Phantom browser extension/,
  );
  assert.equal(phantomSetupMessage('Installed'), null);
});
