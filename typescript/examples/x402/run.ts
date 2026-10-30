import type { Server } from 'node:http';

import { runDeveloper } from './developer.js';
import { startFacilitator } from './facilitator.js';
import { startResourceServer } from './resource-server.js';
import { ensureX402Fixture, X402SetupError } from './setup.js';

async function main(): Promise<void> {
  const arguments_ = process.argv.slice(2);
  if (
    arguments_.length === 1 &&
    (arguments_[0] === '--help' || arguments_[0] === '-h')
  ) {
    console.log('x402 [--server-only]');
    console.log(
      'Default: set up Devnet fixtures and run the TypeScript payer.',
    );
    console.log(
      '--server-only: serve the resource and facilitator until SIGINT/SIGTERM; requires SWIG_ED25519_PRIVATE_KEY shared with the Rust payer.',
    );
    return;
  }
  const serverOnly =
    arguments_.length === 1 && arguments_[0] === '--server-only';
  if (arguments_.length !== 0 && !serverOnly) {
    throw new X402SetupError('Unexpected arguments. Use --help.');
  }
  if (serverOnly && !process.env.SWIG_ED25519_PRIVATE_KEY?.trim()) {
    throw new X402SetupError(
      'Set SWIG_ED25519_PRIVATE_KEY to the same key used by the Rust payer.',
    );
  }

  const fixture = await ensureX402Fixture();
  const facilitatorUrl = `http://127.0.0.1:${fixture.facilitatorPort}`;
  const resourceUrl = `http://127.0.0.1:${fixture.resourceServerPort}/weather`;
  const shutdown = new AbortController();
  const stop = (): void => shutdown.abort();
  if (serverOnly) {
    process.once('SIGINT', stop);
    process.once('SIGTERM', stop);
  }
  let facilitator: Server | undefined;
  let resourceServer: Server | undefined;
  try {
    facilitator = await startFacilitator({
      rpcUrl: fixture.rpcUrl,
      network: fixture.x402Network,
      keypair: fixture.facilitator,
      port: fixture.facilitatorPort,
    });
    if (shutdown.signal.aborted) return;
    resourceServer = await startResourceServer({
      network: fixture.x402Network,
      facilitatorUrl,
      resourceProvider: fixture.resourceProvider.publicKey,
      mint: fixture.mint,
      paymentAmount: fixture.paymentAmount,
      port: fixture.resourceServerPort,
    });
    if (shutdown.signal.aborted) return;

    if (serverOnly) {
      console.log(
        `Developer signer: ${fixture.developer.publicKey.toBase58()}`,
      );
      console.log(`export SWIG_CONFIG_ADDRESS=${fixture.swigConfigAddress}`);
      console.log(`export SWIG_WALLET_ADDRESS=${fixture.swigWalletAddress}`);
      console.log(`export SWIG_X402_RESOURCE_URL=${resourceUrl}`);
      console.log(`export SWIG_X402_ASSET=${fixture.mint.toBase58()}`);
      console.log(
        `export SWIG_X402_PAY_TO=${fixture.resourceProvider.publicKey.toBase58()}`,
      );
      console.log(
        `export SWIG_X402_MAX_AMOUNT=${fixture.paymentAmount.toString()}`,
      );
      await new Promise<void>((resolve) => {
        shutdown.signal.addEventListener('abort', () => resolve(), {
          once: true,
        });
      });
      return;
    }
    await runDeveloper({
      apiKey: fixture.apiKey,
      backendUrl: fixture.backendUrl,
      network: fixture.network,
      swigConfigAddress: fixture.swigConfigAddress,
      swigWalletAddress: fixture.swigWalletAddress,
      developer: fixture.developer,
      resourceUrl,
    });
  } finally {
    process.removeListener('SIGINT', stop);
    process.removeListener('SIGTERM', stop);
    const closing: Promise<void>[] = [];
    if (resourceServer) closing.push(closeServer(resourceServer));
    if (facilitator) closing.push(closeServer(facilitator));
    const results = await Promise.allSettled(closing);
    if (results.some((result) => result.status === 'rejected')) {
      console.error('Could not close the x402 example servers');
      process.exitCode = 1;
    }
  }
}

async function closeServer(server: Server): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => {
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    });
    server.closeAllConnections();
  });
}

try {
  await main();
} catch (error) {
  console.error(
    error instanceof X402SetupError
      ? error.message
      : 'Could not run the x402 example. Check the local services, configured ports, and Devnet funding.',
  );
  process.exitCode = 1;
}
