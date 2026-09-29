/**
 * Diagnostics for code shared between commands, written to stderr.
 *
 * Command results (JSON, node listings) go to stdout so they can be piped, while
 * progress and errors go to stderr. The MCP server relies on that split: its stdout
 * carries the protocol, so nothing but protocol frames may be written there.
 */

type Level = 'info' | 'warn' | 'error';

function write(level: Level, message: string) {
  process.stderr.write(`${level}: ${message}\n`);
}

export const log = {
  info: (message: string) => write('info', message),
  warn: (message: string) => write('warn', message),
  error: (message: string) => write('error', message),
};
