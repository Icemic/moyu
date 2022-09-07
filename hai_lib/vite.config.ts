import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

/**
 * Build configuration for the application
 */
export default defineConfig({
  plugins: [react()],

  envPrefix: 'EXPORT_',

  build: {
    manifest: true,
    minify: 'esbuild',
    sourcemap: true,
  },

  optimizeDeps: {
    exclude: [],
  },

  css: {
    preprocessorOptions: {
      less: {
        javascriptEnabled: true,
      },
    },
  },
  server: {
    host: '0.0.0.0',
    port: 3020,
  },
});
