import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => ({
  define: {
    'process.env.NODE_ENV': JSON.stringify(mode === 'production' ? 'production' : 'development'),
  },
  optimizeDeps: {
    entries: ['./index.js'],
  },
  server: {
    port: 6022,
    strictPort: true,
    cors: true,
    hmr: false,
    watch: {
      ignored: ['**/.git/**', 'assets/**'],
    },
  },
  build: {
    target: 'baseline-widely-available',
    sourcemap: true,
    lib: {
      entry: './src/index.ts',
      formats: ['es'],
      fileName: () => 'index.js',
    },
  },
}));
