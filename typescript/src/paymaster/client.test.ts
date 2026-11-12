import { describe, expect, test } from 'bun:test';

import { SwigClient } from '../server/typescript/index.js';

const paymaster = {
  id: 'paymaster-1',
  address: '11111111111111111111111111111111',
  label: 'API paymaster',
};

describe('API paymaster initialization', () => {
  test.each([true, false])(
    'returns the selected paymaster (created=%s)',
    async (created) => {
      const requests: Request[] = [];
      const swig = new SwigClient({
        apiKey: 'sk_test',
        baseUrl: 'https://api.example.com',
        fetch: Object.assign(
          async (input: RequestInfo | URL, init?: RequestInit) => {
            requests.push(new Request(input, init));
            return Response.json({ ...paymaster, created });
          },
          { preconnect: fetch.preconnect },
        ),
      });
      expect(await swig.paymaster.getOrCreate()).toEqual({
        ...paymaster,
        created,
      });
      expect(requests).toHaveLength(1);
      const request = requests[0];
      if (!request) throw new Error('Expected initialization request');
      expect(request.url).toBe(
        'https://api.example.com/paymaster/api/paymasters/get-or-create',
      );
      expect(request.method).toBe('POST');
      expect(request.headers.get('authorization')).toBe('Bearer sk_test');
      expect(await request.json()).toEqual({});
    },
  );

  test('accepts ProtoJSON omission of created=false', async () => {
    const swig = new SwigClient({
      apiKey: 'sk_test',
      fetch: Object.assign(async () => Response.json(paymaster), {
        preconnect: fetch.preconnect,
      }),
    });
    expect(await swig.paymaster.getOrCreate()).toEqual({
      ...paymaster,
      created: false,
    });
  });

  test('accepts ProtoJSON omission of an empty existing label', async () => {
    const swig = new SwigClient({
      apiKey: 'sk_test',
      fetch: Object.assign(
        async () =>
          Response.json({ id: paymaster.id, address: paymaster.address }),
        {
          preconnect: fetch.preconnect,
        },
      ),
    });
    expect(await swig.paymaster.getOrCreate()).toEqual({
      ...paymaster,
      label: '',
      created: false,
    });
  });

  test('rejects an invalid paymaster address', async () => {
    const swig = new SwigClient({
      apiKey: 'sk_test',
      fetch: Object.assign(
        async () => Response.json({ ...paymaster, address: 'invalid' }),
        {
          preconnect: fetch.preconnect,
        },
      ),
    });
    await expect(swig.paymaster.getOrCreate()).rejects.toThrow();
  });

  test('retries a lost response using the same authenticated initialization operation', async () => {
    let calls = 0;
    const swig = new SwigClient({
      apiKey: 'sk_test',
      retryOptions: { maxRetries: 1, retryDelay: 0 },
      fetch: Object.assign(
        async () => {
          calls++;
          if (calls === 1)
            throw new TypeError('response lost after initialization');
          return Response.json(paymaster);
        },
        { preconnect: fetch.preconnect },
      ),
    });
    expect(await swig.paymaster.getOrCreate()).toEqual({
      ...paymaster,
      created: false,
    });
    expect(calls).toBe(2);
  });

  test.each([
    null,
    {},
    { ...paymaster, id: '' },
    { ...paymaster, address: '' },
    { ...paymaster, created: 'true' },
  ])(
    'rejects a malformed initialization response without reporting success',
    async (response) => {
      let calls = 0;
      const swig = new SwigClient({
        apiKey: 'sk_test',
        fetch: Object.assign(
          async () => {
            calls++;
            return Response.json(response);
          },
          { preconnect: fetch.preconnect },
        ),
      });
      await expect(swig.paymaster.getOrCreate()).rejects.toThrow(
        'response is invalid',
      );
      expect(calls).toBe(1);
    },
  );

  test('does not retry an authorization failure', async () => {
    let calls = 0;
    const swig = new SwigClient({
      apiKey: 'sk_test',
      fetch: Object.assign(
        async () => {
          calls++;
          return Response.json({ message: 'unauthenticated' }, { status: 401 });
        },
        { preconnect: fetch.preconnect },
      ),
    });
    await expect(swig.paymaster.getOrCreate()).rejects.toMatchObject({
      statusCode: 401,
    });
    expect(calls).toBe(1);
  });
});
