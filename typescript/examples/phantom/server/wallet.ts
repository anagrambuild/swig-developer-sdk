import type { Connection } from '@solana/web3.js';
import type { SwigClient } from '@swig-wallet/developer-sdk';
import {
  advanceReservedWalletSetup,
  type ReservedWalletSetup,
} from '@swig-wallet/developer-sdk/onboarding';
import type { DemoState } from './state';

export async function advanceAppWallet(
  state: DemoState,
  client: SwigClient,
  rpc: Connection,
  attempt: ReservedWalletSetup,
  approval?: Parameters<typeof advanceReservedWalletSetup>[3],
): Promise<ReservedWalletSetup> {
  let next = await advanceReservedWalletSetup(client, rpc, attempt, approval);
  state.saveAttempt(next);
  if (next.step.kind === 'submit') {
    // Finish the write-ahead/send pair even if the browser closes after this request.
    next = await advanceReservedWalletSetup(client, rpc, next);
    state.saveAttempt(next);
  }
  return next;
}
