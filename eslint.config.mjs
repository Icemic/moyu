// @ts-check

import ainouCodeStyle from '@ainou/code-style';
import tseslint from 'typescript-eslint';

export default [
  ...ainouCodeStyle,
  {
    files: ['packages/**/vite.config.ts', 'packages/**/*.d.ts'],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    rules: {
      'no-redundant-type-constituents': 'off',
    },
  },
];
