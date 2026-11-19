import { spawnSync } from 'node:child_process';
import { mkdir, readdir } from 'node:fs/promises';
import { appOrigin, isLoopback } from './origin.js';

const origin = appOrigin(process.env.SWIG_APP_ORIGIN);
if (isLoopback(origin.hostname)) {
  await mkdir('.local', { recursive: true, mode: 0o700 });
  const files = await readdir('.local');
  const cert = 'localhost.pem';
  const key = 'localhost-key.pem';
  if (files.includes(cert) !== files.includes(key)) {
    throw new Error(
      'Provide both .local/localhost.pem and .local/localhost-key.pem.',
    );
  }
  if (!files.includes(cert)) {
    const result = spawnSync(
      'mkcert',
      ['-cert-file', cert, '-key-file', key, '127.0.0.1', 'localhost', '::1'],
      {
        cwd: '.local',
        stdio: 'inherit',
      },
    );
    if (result.error || result.status !== 0)
      throw new Error(
        'Provide a trusted localhost certificate and key in .local, or install mkcert and rerun npm run setup:https.',
      );
  }
  console.log(
    'Local TLS files are ready. After startup, run npm run check:https to verify trust.',
  );
} else {
  console.log(
    'Route',
    origin.origin,
    'through your HTTPS proxy to this app on loopback, preserving the original Host header.',
  );
}
