import { expect, test } from 'bun:test';

import { normalizePreparedTransaction } from './normalizers.js';

test.each([7, 8, 9])(
  'does not interpret Yield kind %i as an authorization lock',
  (kind) => {
    const prepared = normalizePreparedTransaction({
      transaction: 'prepared',
      kind,
    });
    expect(prepared.kind).toBeUndefined();
  },
);
