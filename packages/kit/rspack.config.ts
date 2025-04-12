import rspack from '@rspack/core';
// const refreshPlugin = require("@rspack/plugin-react-refresh");
const isDev = process.env.NODE_ENV === 'development';
/**
 * @type {import('@rspack/cli').Configuration}
 */
module.exports = {
  context: __dirname,
  entry: {
    bunnymark: './examples/bunnyMark/index.ts',
  },
  resolve: {
    symlinks: false,
    extensions: ['...', '.ts', '.tsx', '.jsx'],
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
                    refresh: false,
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
    hot: false,
    liveReload: false,
    webSocketServer: false,
    static: {
      publicPath: '/',
      directory: './examples',
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
    // new rspack.ProgressPlugin({}),
    // new rspack.HtmlRspackPlugin({
    //   template: './index.html',
    // }),
    // isDev ? new refreshPlugin() : null
  ].filter(Boolean),
};
