import { describe, expect, test } from 'bun:test';

import { HttpClient } from './http.js';

function client(
  fetchImpl: (
    input: RequestInfo | URL,
    init?: RequestInit,
  ) => Promise<Response>,
): HttpClient {
  return new HttpClient({
    apiKey: 'secret',
    baseUrl: 'https://example.test',
    retry: { maxRetries: 1, retryDelay: 0, backoffMultiplier: 1 },
    fetch: fetchImpl as typeof fetch,
  });
}

describe('HttpClient retry safety', () => {
  test('retries GET requests', async () => {
    let attempts = 0;
    const http = client(async () => {
      attempts++;
      if (attempts === 1) {
        return new Response('temporary', { status: 503 });
      }
      return Response.json({ ok: true });
    });

    await expect(http.get('/resource')).resolves.toEqual({ ok: true });
    expect(attempts).toBe(2);
  });

  test('does not retry POST requests unless the caller opts in', async () => {
    let attempts = 0;
    const http = client(async () => {
      attempts++;
      return new Response('temporary', { status: 503 });
    });

    await expect(http.post('/write', {})).rejects.toThrow();
    expect(attempts).toBe(1);
  });

  test('retries an explicitly idempotent POST request', async () => {
    let attempts = 0;
    const http = client(async () => {
      attempts++;
      if (attempts === 1) {
        return new Response('temporary', { status: 503 });
      }
      return Response.json({ ok: true });
    });

    await expect(http.post('/write', {}, { retry: true })).resolves.toEqual({
      ok: true,
    });
    expect(attempts).toBe(2);
  });
});
