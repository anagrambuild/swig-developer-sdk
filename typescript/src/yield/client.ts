import type { HttpClient } from '../core/index.js';
import type {
  EstimateYieldArgs,
  EstimateYieldResponseWire,
  GetYieldSummaryResponseWire,
  ListYieldOptionsResponseWire,
  ListYieldPositionsResponseWire,
  ListYieldRewardsResponseWire,
  Network,
  PrepareYieldDepositArgs,
  PrepareYieldDepositResponseWire,
  PrepareYieldExitArgs,
  PrepareYieldExitResponseWire,
  PrepareYieldRoleArgs,
  PrepareYieldRoleResponseWire,
  PrepareYieldWithdrawalArgs,
  PrepareYieldWithdrawalResponseWire,
  PreparedYieldDeposit,
  PreparedYieldExit,
  PreparedYieldWithdrawal,
  YieldOption,
  YieldPageArgs,
  YieldPositionsPage,
  YieldProjection,
  YieldRewardsPage,
  YieldRoleResult,
  YieldSignerArgs,
  YieldSummary,
  YieldWalletArgs,
} from '../types/index.js';
import { normalizeAmount, toProtoNetwork } from '../wallets/normalizers.js';
import { walletAuthorityRequest } from '../wallets/requests.js';
import {
  normalizeYieldDeposit,
  normalizeYieldExit,
  normalizeYieldOptions,
  normalizeYieldPositions,
  normalizeYieldProjection,
  normalizeYieldRewards,
  normalizeYieldRole,
  normalizeYieldSummary,
  normalizeYieldWithdrawal,
} from './normalizers.js';

/** Wallet-scoped yield calls, with the wallet fields already bound. */
export type BoundYieldClient = {
  prepareRole(
    args: Omit<PrepareYieldRoleArgs, keyof YieldWalletArgs>,
  ): Promise<YieldRoleResult>;
  deposit(
    args: Omit<PrepareYieldDepositArgs, keyof YieldWalletArgs>,
  ): Promise<PreparedYieldDeposit>;
  exit(
    args: Omit<PrepareYieldExitArgs, keyof YieldWalletArgs>,
  ): Promise<PreparedYieldExit>;
  withdraw(
    args: Omit<PrepareYieldWithdrawalArgs, keyof YieldWalletArgs>,
  ): Promise<PreparedYieldWithdrawal>;
  listOptions(): Promise<YieldOption[]>;
  listPositions(
    args?: Omit<YieldPageArgs, keyof YieldWalletArgs>,
  ): Promise<YieldPositionsPage>;
  getSummary(): Promise<YieldSummary>;
  listRewards(
    args?: Omit<YieldPageArgs, keyof YieldWalletArgs>,
  ): Promise<YieldRewardsPage>;
  estimate(
    args: Omit<EstimateYieldArgs, keyof YieldWalletArgs>,
  ): Promise<YieldProjection>;
};

/**
 * Yield on Swig wallets. Native staking to allowlisted validators is the first
 * method. Prepare calls return unsigned transactions: authorize them with the
 * requester authority, sign with the fee payer, then submit or sponsor them.
 */
export class YieldClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  /** Bind a wallet so later calls only need the operation's own fields. */
  forWallet = (wallet: YieldWalletArgs): BoundYieldClient => ({
    prepareRole: (args) => this.prepareRole({ ...wallet, ...args }),
    deposit: (args) => this.deposit({ ...wallet, ...args }),
    exit: (args) => this.exit({ ...wallet, ...args }),
    withdraw: (args) => this.withdraw({ ...wallet, ...args }),
    listOptions: () => this.listOptions(wallet),
    listPositions: (args) => this.listPositions({ ...wallet, ...args }),
    getSummary: () => this.getSummary(wallet),
    listRewards: (args) => this.listRewards({ ...wallet, ...args }),
    estimate: (args) => this.estimate({ ...wallet, ...args }),
  });

  /**
   * Reuse a role that can deposit, exit and withdraw, or prepare an AddRole
   * transaction creating one. After that transaction confirms, call again
   * without `roleId` to resolve the new role.
   */
  prepareRole = async (args: PrepareYieldRoleArgs): Promise<YieldRoleResult> =>
    normalizeYieldRole(
      await this.http.post<PrepareYieldRoleResponseWire>(
        `${basePath(args)}/role/prepare`,
        {
          network: this.network(args),
          swigConfigAddress: args.swigConfigAddress,
          feePayer: args.feePayer,
          ...(args.roleId === undefined ? {} : { roleId: args.roleId }),
          yieldAuthority: walletAuthorityRequest(args.yieldAuthority),
          validatorVoteAddress: args.validatorVoteAddress,
          maxDepositLamports: normalizeAmount(args.maxDeposit),
          ...(args.roleManagerAuthority
            ? {
                roleManagerAuthority: walletAuthorityRequest(
                  args.roleManagerAuthority,
                ),
              }
            : {}),
        },
      ),
    );

  /** Fund a new native position from the wallet and delegate it. */
  deposit = async (
    args: PrepareYieldDepositArgs,
  ): Promise<PreparedYieldDeposit> =>
    normalizeYieldDeposit(
      await this.http.post<PrepareYieldDepositResponseWire>(
        `${basePath(args)}/deposit/prepare`,
        {
          wallet: this.wallet(args),
          lamports: normalizeAmount(args.amount),
          validatorVoteAddress: args.validatorVoteAddress,
          positionSeed: args.positionSeed,
        },
      ),
    );

  /** Deactivate a whole position, or split part of an active one and deactivate that. */
  exit = async (args: PrepareYieldExitArgs): Promise<PreparedYieldExit> =>
    normalizeYieldExit(
      await this.http.post<PrepareYieldExitResponseWire>(
        `${basePath(args)}/exit/prepare`,
        {
          wallet: this.wallet(args),
          positionAddress: args.positionAddress,
          ...(args.amount === 'all'
            ? { all: {} }
            : {
                partial: {
                  lamports: normalizeAmount(args.amount.lamports),
                  positionSeed: args.amount.positionSeed,
                },
              }),
        },
      ),
    );

  /** Withdraw an inactive position's balance back to the Swig wallet. */
  withdraw = async (
    args: PrepareYieldWithdrawalArgs,
  ): Promise<PreparedYieldWithdrawal> =>
    normalizeYieldWithdrawal(
      await this.http.post<PrepareYieldWithdrawalResponseWire>(
        `${basePath(args)}/withdraw/prepare`,
        { wallet: this.wallet(args), positionAddress: args.positionAddress },
      ),
    );

  listOptions = async (args: YieldWalletArgs): Promise<YieldOption[]> =>
    normalizeYieldOptions(
      await this.http.get<ListYieldOptionsResponseWire>(
        `${basePath(args)}/options?${this.query(args)}`,
      ),
    );

  listPositions = async (args: YieldPageArgs): Promise<YieldPositionsPage> =>
    normalizeYieldPositions(
      await this.http.get<ListYieldPositionsResponseWire>(
        `${basePath(args)}/positions?${this.query(args, page(args))}`,
      ),
    );

  getSummary = async (args: YieldWalletArgs): Promise<YieldSummary> =>
    normalizeYieldSummary(
      await this.http.get<GetYieldSummaryResponseWire>(
        `${basePath(args)}/summary?${this.query(args)}`,
      ),
    );

  listRewards = async (args: YieldPageArgs): Promise<YieldRewardsPage> =>
    normalizeYieldRewards(
      await this.http.get<ListYieldRewardsResponseWire>(
        `${basePath(args)}/rewards?${this.query(args, page(args))}`,
      ),
    );

  estimate = async (args: EstimateYieldArgs): Promise<YieldProjection> => {
    if (
      !Number.isInteger(args.horizonDays) ||
      args.horizonDays < 1 ||
      args.horizonDays > 365
    ) {
      throw new Error('horizonDays must be an integer between 1 and 365');
    }
    return normalizeYieldProjection(
      await this.http.get<EstimateYieldResponseWire>(
        `${basePath(args)}/estimate?${this.query(args, {
          validatorVoteAddress: args.validatorVoteAddress,
          principalLamports: normalizeAmount(args.principal),
          horizonDays: String(args.horizonDays),
        })}`,
      ),
    );
  };

  private network(args: YieldWalletArgs) {
    const network = args.network ?? this.defaultNetwork;
    if (!network) throw new Error('network is required');
    return toProtoNetwork(network);
  }

  private wallet(args: YieldSignerArgs) {
    return {
      network: this.network(args),
      swigConfigAddress: args.swigConfigAddress,
      feePayer: args.feePayer,
      ...(args.roleId === undefined ? {} : { roleId: args.roleId }),
      requesterAuthority: walletAuthorityRequest(args.requesterAuthority),
    };
  }

  private query(
    args: YieldWalletArgs,
    extra: Record<string, string> = {},
  ): string {
    return new URLSearchParams({
      network: this.network(args),
      ...extra,
    }).toString();
  }
}

function basePath(args: YieldWalletArgs): string {
  if (!args.swigConfigAddress) throw new Error('swigConfigAddress is required');
  return `/api/yield/${encodeURIComponent(args.swigConfigAddress)}`;
}

function page(args: YieldPageArgs): Record<string, string> {
  if (
    args.pageSize !== undefined &&
    (args.pageSize < 1 || args.pageSize > 200)
  ) {
    throw new Error('pageSize must be between 1 and 200');
  }
  return {
    ...(args.pageSize === undefined ? {} : { pageSize: String(args.pageSize) }),
    ...(args.pageToken ? { pageToken: args.pageToken } : {}),
  };
}
