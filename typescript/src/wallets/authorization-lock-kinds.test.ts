import { expect, test } from 'bun:test';

import type { PreparedTransactionKind } from '../types/transaction.js';
import { normalizePreparedTransaction } from './normalizers.js';

test.each([
  [7, 'yield-deposit'],
  [8, 'yield-exit'],
  [9, 'yield-withdrawal'],
] satisfies [number, PreparedTransactionKind][])(
  'does not interpret Yield kind %i as an authorization lock',
  (kind, expectedKind) => {
    const prepared = normalizePreparedTransaction({
      transaction: 'prepared',
      kind,
    });
    expect(prepared.kind).toBe(expectedKind);
  },
);
