import { describe, expect, test } from 'bun:test';

import { SwigDeveloperSdkError } from './errors.js';

describe('SwigDeveloperSdkError', () => {
  test('falls back when the top-level gateway code is not a string', () => {
    const error = SwigDeveloperSdkError.fromResponse(
      new Response(null, { status: 503 }),
      { code: 14, message: 'upstream unavailable' },
    );

    expect(error.code).toBe('HTTP_503');
    expect(error.message).toBe('upstream unavailable');
  });

  test('falls back when a nested gateway code is not a string', () => {
    const error = SwigDeveloperSdkError.fromResponse(
      new Response(null, { status: 400 }),
      { error: { code: 3, message: 'invalid argument' } },
    );

    expect(error.code).toBe('HTTP_400');
    expect(error.message).toBe('invalid argument');
  });
});
