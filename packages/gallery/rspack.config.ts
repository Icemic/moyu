import rspack from '@rspack/core';
import refreshPlugin from '@rspack/plugin-react-refresh';

const isDev = process.env.NODE_ENV === 'development';

export default {
  context: __dirname,
  entry: './src/index.tsx',
  output: {
    clean: true,
    filename: 'index.js',
  },
  resolve: {
    symlinks: false,
    extensions: ['...', '.ts', '.tsx', '.jsx'],
  },
  watchOptions: {
    ignored: /[\\/](?:\.git[\\/]|node_modules[\\/](?!@momoyu-ink[\\/]))/,
  },
  performance: {
    hints: false,
  },
  module: {
    rules: [
      {
        test: /\.svg$/,
        type: 'asset',
      },
      {
        test: /\.(jsx?|tsx?)$/,
        use: [
          {
            loader: 'builtin:swc-loader',
            options: {
              sourceMap: true,
              jsc: {
                parser: {
                  syntax: 'typescript',
                  tsx: true,
                },
                transform: {
                  react: {
                    runtime: 'automatic',
                    development: isDev,
                    refresh: isDev,
                    importSource: '@momoyu-ink/kit',
                  },
                },
              },
              env: {
                targets: ['chrome >= 87', 'edge >= 88', 'firefox >= 78', 'safari >= 14'],
              },
            },
          },
        ],
      },
    ],
  },
  devServer: {
    client: {
      overlay: false,
    },
    port: 6023,
    hot: true,
    liveReload: false,
    webSocketServer: 'ws',
    static: {
      publicPath: '/',
      directory: './',
    },
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH, OPTIONS',
      'Access-Control-Allow-Headers':
        'X-Requested-With, content-type, Authorization, cache-control, pragma, upgrade-insecure-requests, user-agent',
    },
    compress: true,
  },
  plugins: [
    new rspack.DefinePlugin({
      'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV),
    }),
    isDev && new refreshPlugin(),
    isDev && new rspack.HotModuleReplacementPlugin(),
  ].filter(Boolean),
};
