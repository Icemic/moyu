/**
 * MCP server command.
 *
 * Serves the engine debug bridge as Model Context Protocol tools on stdio, so an AI
 * host can start the project engine and inspect it. See
 * `rfcs/2026-09-25-runtime-debug-bridge.md`.
 */

import { defineCommand } from 'citty';
import { serveMcp } from '../mcp/protocol.js';
import { createTools, stopSession } from '../mcp/tools.js';
import { log } from '../utils/log.js';
import { getCurrentVersion } from '../utils/update-check.js';

export default defineCommand({
  meta: {
    name: 'mcp',
    description: 'Serve the engine debug bridge as MCP tools on stdio',
  },
  run: () => {
    // stdout carries the protocol, so this and every other diagnostic goes to stderr.
    log.info('Moyu MCP server ready on stdio.');

    serveMcp({
      info: { name: 'moyu', version: getCurrentVersion() },
      tools: createTools,
      onShutdown: stopSession,
    });
  },
});
