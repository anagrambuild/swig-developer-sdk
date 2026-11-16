import type { Network } from './common.js';

export interface GetProjectSummaryArgs {
  network?: Network;
}

/** Indexed project wallet totals. TVL excludes rent, paymasters, and unpriced assets. */
export interface ProjectSummary {
  organizationId: string;
  projectName: string;
  network: Network;
  /** Decimal uint64 string, including indexed wallets that have since closed. */
  walletsCreated: string;
  tvlUsd: number;
  /** Decimal uint64 string. Nonzero means the valuation is partial. */
  unpricedAssetCount: string;
  /** Up to five wallets, newest indexed creation first. Empty on older servers. */
  recentWallets: RecentProjectWallet[];
}

export interface RecentProjectWallet {
  swigConfigAddress: string;
  walletAddress: string;
  active: boolean;
}
