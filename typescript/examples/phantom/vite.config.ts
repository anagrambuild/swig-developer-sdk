import react from '@vitejs/plugin-react';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  root: 'src',
  publicDir: false,
  plugins: [react()],
  server: {
    fs: {
      strict: true,
      allow: [resolve('src'), resolve('node_modules')],
      deny: [
        '**/.env*',
        '**/.local/**',
        '**/server/**',
        '**/server.ts',
        '**/scripts/**',
      ],
    },
  },
  build: { outDir: '../dist', emptyOutDir: true },
});
