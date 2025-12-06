import type { StorybookConfig } from '@storybook/react-vite';
import { mergeConfig } from 'vite';

const config: StorybookConfig = {
  stories: ['../packages/kit/stories/**/*.mdx', '../packages/kit/stories/**/*.stories.@(js|jsx|mjs|ts|tsx)'],
  staticDirs: [
    { from: './static', to: '/' },
    { from: '../packages/kit/stories/static', to: '/assets' },
  ],
  addons: ['@storybook/addon-onboarding', '@storybook/addon-docs'],
  framework: '@storybook/react-vite',
  core: {
    disableTelemetry: true,
  },

  async viteFinal(config) {
    return mergeConfig(config, {
      resolve: {
        preserveSymlinks: false,
        extensions: ['.ts', '.tsx', '.jsx', '.js'],
        alias: {
          // Map @momoyu-ink/kit jsx-runtime to React's jsx-runtime
          '@momoyu-ink/kit/jsx-runtime': 'react/jsx-runtime',
          '@momoyu-ink/kit/jsx-dev-runtime': 'react/jsx-dev-runtime',
        },
      },
      optimizeDeps: {
        include: ['@momoyu-ink/kit'],
      },
    });
  },
};

export default config;
