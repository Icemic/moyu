/**
 * Debug subcommands.
 *
 * Each subcommand starts a session (see `utils/debug-session.ts`), asks the engine one
 * question, and prints the answer. Results go to stdout so they can be piped; progress
 * and errors go to stderr.
 */

import { writeFile } from 'node:fs/promises';
import { defineCommand } from 'citty';
import {
  DEFAULT_CONNECT_TIMEOUT_MS,
  DEFAULT_READY_TIMEOUT_MS,
  type DebugHost,
  type DebugMessage,
  type SessionOptions,
  startDebugSession,
} from '../utils/debug-session.js';
import { log } from '../utils/log.js';

/** Port for the web engine server when the caller does not pick one. */
const DEFAULT_WEB_PORT = 6320;
/** Port that attach mode listens on, unless the caller picks another one. */
const DEFAULT_ATTACH_PORT = 6321;
/** Levels of children `tree` prints unless asked for another number. */
const DEFAULT_TREE_DEPTH = 8;

/** Options shared by every subcommand: how to reach or start the engine. */
const sessionArgs = {
  web: {
    type: 'boolean',
    alias: 'w',
    description: 'Debug an engine running in a browser page',
  },
  port: {
    type: 'string',
    description: 'Port for the web engine server',
    default: String(DEFAULT_WEB_PORT),
  },
  entry: {
    type: 'string',
    description: 'Entry file passed to a native engine',
  },
  attach: {
    type: 'boolean',
    description: 'Attach to an engine started by hand instead of starting one',
  },
  listen: {
    type: 'string',
    description: 'Port to listen on in attach mode',
    default: String(DEFAULT_ATTACH_PORT),
  },
  timeout: {
    type: 'string',
    description: 'Milliseconds to wait for the engine to connect',
    default: String(DEFAULT_CONNECT_TIMEOUT_MS),
  },
} as const;

export default defineCommand({
  meta: {
    name: 'debug',
    description: 'Inspect a running engine through the debug bridge',
  },
  subCommands: {
    state: defineCommand({
      meta: {
        name: 'state',
        description: 'Print the state snapshot of an engine',
      },
      args: sessionArgs,
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(readyTimeout(args.timeout));
          printJson(await host.request('engine:state'));
        }),
    }),
    eval: defineCommand({
      meta: {
        name: 'eval',
        description: 'Evaluate JavaScript in the engine and print the result',
      },
      args: {
        ...sessionArgs,
        code: {
          type: 'positional',
          description: 'JavaScript to evaluate',
          required: true,
        },
        'eval-timeout': {
          type: 'string',
          description: 'Milliseconds the engine may spend on the evaluation',
          default: '5000',
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          // Evaluating before the project script has run would report nothing useful.
          await host.waitForReady(readyTimeout(args.timeout));

          const result = await host.request('engine:eval', {
            code: args.code,
            timeoutMs: Number(args['eval-timeout']),
          });

          write(result.value === undefined ? String(result.repr) : JSON.stringify(result.value, null, 2));

          if (result.truncated === true) {
            log.warn('Result exceeded the size limit and is only shown in short form.');
          }
        }),
    }),
    logs: defineCommand({
      meta: {
        name: 'logs',
        description: 'Read engine logs, optionally following new entries',
      },
      args: {
        ...sessionArgs,
        level: {
          type: 'string',
          description: 'Only entries at this level or more severe',
        },
        limit: {
          type: 'string',
          description: 'Maximum number of entries to return',
        },
        since: {
          type: 'string',
          description: 'Only entries newer than this sequence number',
        },
        follow: {
          type: 'boolean',
          alias: 'f',
          description: 'Keep printing new entries until interrupted',
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          // Pushes may arrive before the response that carries the snapshot, and some of
          // them are also part of that snapshot, so they wait here until it is printed.
          const pending: LogEntry[] = [];
          let watching = false;
          let lastSeq = 0;

          const printNewEntry = (entry: LogEntry) => {
            // Entries at or below the cursor are either already printed or part of the
            // snapshot; a jump means the engine dropped pushes for a slow client.
            if (entry.seq <= lastSeq) return;

            lastSeq = entry.seq;
            printLogEntry(entry);
          };

          host.onMessage((message) => {
            if (message.type !== 'engine:log') return;

            if (watching) {
              printNewEntry(message.entry);
            } else {
              pending.push(message.entry);
            }
          });

          const result = await host.request('engine:logs', {
            level: args.level,
            limit: args.limit === undefined ? undefined : Number(args.limit),
            sinceSeq: args.since === undefined ? undefined : Number(args.since),
            subscribe: Boolean(args.follow),
          });

          for (const entry of result.entries) {
            printLogEntry(entry);
            lastSeq = Math.max(lastSeq, entry.seq);
          }

          if (!args.follow) {
            return;
          }

          watching = true;

          for (const entry of pending) {
            printNewEntry(entry);
          }

          pending.length = 0;

          if (result.dropped > 0) {
            log.warn(`${result.dropped} earlier entries were dropped.`);
          }

          log.info('Following. Press Ctrl+C to stop.');

          // The session stays open for as long as the user watches the logs.
          await waitForInterrupt();
        }),
    }),
    tree: defineCommand({
      meta: {
        name: 'tree',
        description: 'Print the node tree',
      },
      args: {
        ...sessionArgs,
        node: {
          type: 'positional',
          description: 'Node id to start from (defaults to the root)',
          required: false,
        },
        depth: {
          type: 'string',
          description: 'Levels of children to print',
          default: String(DEFAULT_TREE_DEPTH),
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(readyTimeout(args.timeout));

          const result = await host.request('engine:tree', {
            nodeId: args.node === undefined ? undefined : Number(args.node),
            depth: Number(args.depth),
          });

          printNode(result.node, 0);
        }),
    }),
    props: defineCommand({
      meta: {
        name: 'props',
        description: 'Print one node properties and derived state',
      },
      args: {
        ...sessionArgs,
        node: {
          type: 'positional',
          description: 'Node id to read',
          required: true,
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(readyTimeout(args.timeout));

          const result = await host.request('engine:props', { nodeId: Number(args.node) });
          printJson(result.node);
        }),
    }),
    screenshot: defineCommand({
      meta: {
        name: 'screenshot',
        description: 'Capture the stage and write it to a file',
      },
      args: {
        ...sessionArgs,
        file: {
          type: 'positional',
          description: 'File to write',
          required: true,
        },
        'max-width': {
          type: 'string',
          description: 'Largest width to return; needs --max-height as well',
        },
        'max-height': {
          type: 'string',
          description: 'Largest height to return; needs --max-width as well',
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(readyTimeout(args.timeout));

          const result = await host.request('engine:screenshot', {
            maxWidth: args['max-width'] === undefined ? undefined : Number(args['max-width']),
            maxHeight: args['max-height'] === undefined ? undefined : Number(args['max-height']),
          });

          await writeFile(args.file, Buffer.from(result.data, 'base64'));
          log.info(`Wrote ${result.width}x${result.height} ${result.format} to ${args.file}`);
        }),
    }),
  },
});

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

type SessionArgs = {
  web?: boolean;
  port: string;
  entry?: string;
  attach?: boolean;
  listen: string;
  timeout: string;
};

/**
 * Start a session, run `task` against the connected engine, then shut everything down.
 * A task that needs the session to stay open simply does not return until it is done.
 */
async function runSession(args: SessionArgs, task: (host: DebugHost) => Promise<void>) {
  const options: SessionOptions = args.attach
    ? { attachPort: Number(args.listen) }
    : { web: args.web, port: Number(args.port), entry: args.entry };

  let session: Awaited<ReturnType<typeof startDebugSession>> | null = null;
  let exitCode = 0;

  try {
    session = await startDebugSession(options);
    await session.host.waitForEngine(Number(args.timeout));
    await task(session.host);
  } catch (error) {
    log.error(error instanceof Error ? error.message : String(error));
    exitCode = 1;
  } finally {
    session?.stop();
  }

  process.exit(exitCode);
}

/**
 * The engine starts up on its own schedule, so waiting for it gets its own budget
 * rather than sharing the connect timeout.
 */
function readyTimeout(connectTimeout: string): number {
  return Math.max(Number(connectTimeout), DEFAULT_READY_TIMEOUT_MS);
}

/** Resolve when the user interrupts the process. */
function waitForInterrupt(): Promise<void> {
  return new Promise((resolve) => {
    process.once('SIGINT', () => resolve());
    process.once('SIGTERM', () => resolve());
  });
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/** One node of a tree response. */
interface NodeSummary {
  id: number;
  type: string;
  label: string;
  visible: boolean;
  children: NodeSummary[];
}

/** One entry of the engine's log buffer, as sent by `engine:logs` and `engine:log`. */
interface LogEntry {
  seq: number;
  level: string;
  target: string;
  message: string;
  timestampMs: number;
}

function write(line: string) {
  process.stdout.write(`${line}\n`);
}

function printJson(value: unknown) {
  write(JSON.stringify(value, null, 2));
}

/** Print a node and its children as an indented list. */
function printNode(node: NodeSummary, indent: number) {
  const label = node.label === '' ? '' : ` ${JSON.stringify(node.label)}`;
  const hidden = node.visible ? '' : ' (hidden)';

  write(`${'  '.repeat(indent)}#${node.id} ${node.type}${label}${hidden}`);

  for (const child of node.children) {
    printNode(child, indent + 1);
  }
}

function printLogEntry(entry: DebugMessage) {
  const stamp = new Date(entry.timestampMs).toISOString().slice(11, 23);
  write(`${stamp} ${String(entry.level).toUpperCase()} ${entry.target}: ${entry.message}`);
}
