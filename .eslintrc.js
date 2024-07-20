module.exports = {
  parser: '@typescript-eslint/parser',
  ignorePatterns: ['dist', 'node_modules', ' .prettierrc.js', '.eslintrc.js'],
  extends: [
    'eslint:recommended',
    'eslint-config-prettier',
    'plugin:prettier/recommended',
    'plugin:@typescript-eslint/eslint-recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:@typescript-eslint/recommended-requiring-type-checking',
  ],
  plugins: ['@typescript-eslint'],
  rules: {
    'no-console': 'off',
    'no-case-declarations': 'off',
    // '@typescript-eslint/no-unsafe-call': 'off',
    // '@typescript-eslint/no-unsafe-member-access': 'off',
    '@typescript-eslint/explicit-module-boundary-types': 'off',
    '@typescript-eslint/explicit-function-return-type': 'off',
    '@typescript-eslint/no-explicit-any': 'off',
    '@typescript-eslint/no-unsafe-assignment': 'off',
    '@typescript-eslint/no-unsafe-return': 'off',
    '@typescript-eslint/no-use-before-define': ['error', { functions: false }],
    '@typescript-eslint/no-floating-promises': 'error',
    '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
    '@typescript-eslint/no-namespace': [2, { allowDeclarations: true }],
    '@typescript-eslint/no-empty-interface': ['warn', { allowSingleExtends: true }],
    '@typescript-eslint/no-empty-function': 'warn',
  },
  parserOptions: {
    ecmaVersion: 2019,
    sourceType: 'module',
    tsconfigRootDir: __dirname,
    project: ['./tsconfig.json'],
  },
  overrides: [
    {
      files: ['.eslintrc.js', '.prettierrc.js', 'jest.config.js', 'rspack.config.js'],
      env: {
        node: true,
      },
    },
  ],
};
