import { lingui, linguiTransformerBabelPreset } from '@lingui/vite-plugin';
import babel from '@rolldown/plugin-babel';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => ({
  plugins: [
    react({ jsxImportSource: '@momoyu-ink/kit' }),
    lingui(),
    babel({ presets: [linguiTransformerBabelPreset()] }),
  ],
  define: {
    'process.env.NODE_ENV': JSON.stringify(mode === 'production' ? 'production' : 'development'),
  },
  optimizeDeps: {
    entries: ['./index.js'],
  },
  server: {
    port: 6023,
    strictPort: true,
    cors: true,
    hmr: {
      overlay: false,
    },
    watch: {
      ignored: ['**/.git/**', 'assets/**'],
    },
  },
  build: {
    target: 'baseline-widely-available',
    sourcemap: true,
    lib: {
      entry: './src/index.tsx',
      formats: ['es'],
      fileName: () => 'index.js',
    },
  },
}));
