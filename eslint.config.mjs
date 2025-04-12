// @ts-check

import ainouCodeStyle from '@ainou/code-style';
import tseslint from 'typescript-eslint';

export default [
  ...ainouCodeStyle,
  {
    files: ['packages/**/vite.config.ts', 'packages/**/rspack.config.ts', 'packages/**/*.d.ts'],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    rules: {
      'no-redundant-type-constituents': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          args: 'all',
          argsIgnorePattern: '^_',
          caughtErrors: 'all',
          caughtErrorsIgnorePattern: '^_',
          destructuredArrayIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          ignoreRestSiblings: true,
        },
      ],
    },
  },
];
