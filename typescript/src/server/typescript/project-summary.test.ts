import { describe, expect, test } from 'bun:test';
import { SwigClient } from '../../index.js';

describe('project summary', () => {
  test('uses the selected network and preserves uint64 totals', async () => {
    const calls: string[] = [];
    const swig = new SwigClient({
      apiKey: 'test',
      network: 'mainnet',
      fetch: Object.assign(
        async (url: RequestInfo | URL) => {
          calls.push(String(url));
          return Response.json({
            organizationId: 'project',
            projectName: 'App',
            network: 'NETWORK_DEVNET',
            walletsCreated: '9007199254740993',
            tvlUsd: 42.5,
            unpricedAssetCount: '2',
          });
        },
        { preconnect: fetch.preconnect },
      ),
    });
    expect(await swig.getProjectSummary({ network: 'devnet' })).toEqual({
      organizationId: 'project',
      projectName: 'App',
      network: 'devnet',
      walletsCreated: '9007199254740993',
      tvlUsd: 42.5,
      unpricedAssetCount: '2',
    });
    expect(new URL(calls[0] ?? '').pathname).toBe(
      '/wallet/api/project/summary',
    );
    expect(new URL(calls[0] ?? '').searchParams.get('network')).toBe(
      'NETWORK_DEVNET',
    );
  });

  test('accepts ProtoJSON zero defaults and requires a network before sending', async () => {
    let calls = 0;
    const swig = new SwigClient({
      apiKey: 'test',
      fetch: Object.assign(
        async () => {
          calls++;
          return Response.json({
            organizationId: 'project',
            network: 'NETWORK_MAINNET',
          });
        },
        { preconnect: fetch.preconnect },
      ),
    });
    await expect(swig.getProjectSummary()).rejects.toThrow('network');
    expect(calls).toBe(0);
    expect(await swig.getProjectSummary({ network: 'mainnet' })).toMatchObject({
      walletsCreated: '0',
      tvlUsd: 0,
      unpricedAssetCount: '0',
    });
  });

  for (const invalid of [
    null,
    {},
    { organizationId: '' },
    { network: 'NETWORK_MAINNET' },
    { walletsCreated: 12 },
    { walletsCreated: '-1' },
    { walletsCreated: '18446744073709551616' },
    { unpricedAssetCount: '1.5' },
    { unpricedAssetCount: null },
    { tvlUsd: -1 },
    { tvlUsd: '42' },
    { tvlUsd: null },
  ]) {
    test(`rejects invalid summary ${JSON.stringify(invalid)}`, async () => {
      const response =
        invalid === null || Object.keys(invalid).length === 0
          ? invalid
          : {
              organizationId: 'project',
              network: 'NETWORK_DEVNET',
              ...invalid,
            };
      const swig = new SwigClient({
        apiKey: 'test',
        network: 'devnet',
        fetch: Object.assign(async () => Response.json(response), {
          preconnect: fetch.preconnect,
        }),
      });
      await expect(swig.getProjectSummary()).rejects.toThrow(
        'Invalid project summary',
      );
    });
  }
});
