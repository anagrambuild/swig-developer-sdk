import { FlatCompat } from '@eslint/eslintrc';
import js from '@eslint/js';
import typescriptEslint from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import { defineConfig, globalIgnores } from 'eslint/config';
import globals from 'globals';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const currentDirectory = path.dirname(fileURLToPath(import.meta.url));
const compat = new FlatCompat({
  baseDirectory: currentDirectory,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default defineConfig([
  globalIgnores(['**/node_modules/', '**/dist/']),
  {
    extends: compat.extends(
      'eslint:recommended',
      'plugin:@typescript-eslint/recommended',
      'prettier',
    ),
    plugins: { '@typescript-eslint': typescriptEslint },
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
      parser: tsParser,
    },
    rules: { '@typescript-eslint/no-explicit-any': 'off' },
  },
]);
