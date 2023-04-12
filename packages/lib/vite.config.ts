import { resolve } from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

/**
 * Build configuration for the application
 */
export default defineConfig({
  plugins: [
    react({
      fastRefresh: false,
    }),
  ],

  define: {
    'process.env': {
      NODE_ENV: 'production',
    },
  },

  envPrefix: 'EXPORT_',

  build: {
    manifest: false,
    minify: 'esbuild',
    sourcemap: true,
    target: 'es2022',
    assetsInlineLimit: 0,
    lib: {
      entry: resolve(__dirname, 'src/lib.ts'),
      fileName: 'hai',
      formats: ['es'],
    },
    rollupOptions: {
      input: {
        reactExample: resolve(__dirname, 'examples/react/index.tsx'),
        bunnyMark: resolve(__dirname, 'examples/bunnyMark/index.ts'),
      },
      output: {
        entryFileNames: '[name]/index.js',
        inlineDynamicImports: false,
        format: 'es',
        manualChunks: {},
        preserveModules: false,
      },
    },
  },

  optimizeDeps: {
    exclude: [],
  },

  server: {
    host: '0.0.0.0',
    port: 3020,
    hmr: false,
  },
});
