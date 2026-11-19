import type { AddRoleAction } from '@swig-wallet/developer-sdk';
import assert from 'node:assert/strict';
import test from 'node:test';
import { readAgentAllowance, usdcMint } from '../src/policy';

test('wallet review displays the saved amounts and interval exactly', () => {
  const actions: AddRoleAction[] = [
    { type: 'programAll' },
    { type: 'solRecurringLimit', recurringAmount: '1001', window: '432000' },
    {
      type: 'tokenRecurringLimit',
      recurringAmount: '9007199254740993',
      mint: usdcMint,
      window: '432000',
    },
  ];
  assert.deepEqual(readAgentAllowance(actions), {
    token: '9007199254740993',
    sol: '1001',
    window: '432000',
  });
  assert.throws(() => readAgentAllowance([...actions, { type: 'all' }]));
  assert.throws(() =>
    readAgentAllowance(
      actions.map((action) =>
        action.type === 'tokenRecurringLimit'
          ? { ...action, mint: 'another' }
          : action,
      ),
    ),
  );
  assert.throws(() =>
    readAgentAllowance(
      actions.map((action) =>
        action.type === 'solRecurringLimit'
          ? { ...action, window: '216000' }
          : action,
      ),
    ),
  );
});
