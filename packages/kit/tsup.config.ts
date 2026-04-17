import { defineConfig } from 'tsup';

export default defineConfig({
  entry: {
    lib: 'src/lib.ts',
    'jsx-runtime': 'src/jsx-runtime.ts',
    'jsx-dev-runtime': 'src/jsx-dev-runtime.ts',
  },
  format: ['esm', 'cjs'],
  target: 'esnext',
  platform: 'neutral',
  bundle: true,
  splitting: false,
  clean: true,
  dts: {
    entry: {
      lib: 'src/lib.ts',
      'jsx-runtime': 'src/jsx-runtime.ts',
      'jsx-dev-runtime': 'src/jsx-dev-runtime.ts',
    },
  },
  tsconfig: './tsconfig.build.json',
  outDir: 'dist',
  outExtension({ format }) {
    return {
      js: format === 'esm' ? '.mjs' : '.cjs',
      dts: '.d.ts',
    };
  },
});
