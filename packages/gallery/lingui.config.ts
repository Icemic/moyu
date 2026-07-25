import { defineConfig } from '@lingui/cli';

export default defineConfig({
  sourceLocale: 'zh',
  locales: ['zh'],
  catalogs: [
    {
      path: '<rootDir>/src/locales/{locale}/messages',
      include: ['<rootDir>/src'],
    },
  ],
});
