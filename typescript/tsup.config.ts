import { defineConfig, type Options } from 'tsup';

const entry = {
  core: 'src/core.ts',
  index: 'src/index.ts',
  signers: 'src/signers.ts',
};

const commonCfg: Partial<Options> = {
  splitting: true,
  sourcemap: false,
  clean: true,
  format: ['cjs', 'esm'],
  target: ['esnext'],
};

export default defineConfig([
  {
    ...commonCfg,
    entry,
    dts: {
      entry,
    },
  },
]);
