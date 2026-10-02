/**
 * The tools the MCP server exposes, mapped onto the engine debug bridge.
 *
 * A session is started on demand and kept alive across calls, so a host can start an
 * engine once and then inspect it repeatedly. Only one session is kept at a time;
 * starting another replaces it.
 */

import { DEFAULT_CONNECT_TIMEOUT_MS, type DebugSession, startDebugSession } from '../utils/debug-session.js';
import { log } from '../utils/log.js';
import { findProjectRoot } from '../utils/project.js';
import type { ToolContent, ToolDefinition, ToolResult } from './protocol.js';

/** How long `debug_start` waits for the engine to finish starting up. */
const START_READY_TIMEOUT_MS = 60_000;
/** How long a tool waits for the engine to finish starting up before giving up. */
const READY_TIMEOUT_MS = 30_000;
/** Port an attach session listens on when the caller does not pick one. */
const DEFAULT_ATTACH_PORT = 6321;

let session: DebugSession | null = null;

/** Stop the running session, if any. Called on shutdown and by `debug_stop`. */
export function stopSession() {
  session?.stop();
  session = null;
}

export function createTools(): ToolDefinition[] {
  return [
    startTool,
    stopTool,
    stateTool,
    evalTool,
    logsTool,
    treeTool,
    propsTool,
    screenshotTool,
    mouseTool,
    touchTool,
    keyTool,
  ];
}

// ---------------------------------------------------------------------------
// Session control
// ---------------------------------------------------------------------------

const startTool: ToolDefinition = {
  name: 'debug_start',
  title: 'Start a debug session',
  description:
    'Start the project engine with the debug bridge and wait until it is running. Call this before any ' +
    'other moyu tool. Starting a session stops the previous one. Native sessions open a window; web ' +
    'sessions serve the project and report a page URL that has to be opened in a browser. With attach, ' +
    'the session only listens on a port and reports the MOYU_ENGINE_DEBUG_WS value an engine must be ' +
    'started with; nothing is launched or waited for, which is how a locally built or packaged engine ' +
    'gets inspected.',
  inputSchema: {
    type: 'object',
    properties: {
      web: {
        type: 'boolean',
        description: 'Serve the project for a browser instead of opening a native window',
        default: false,
      },
      port: {
        type: 'number',
        description: 'Port for the web server; ignored by native sessions',
      },
      entry: {
        type: 'string',
        description: 'Entry file for a native engine; defaults to the project dev server',
      },
      projectRoot: {
        type: 'string',
        description: 'Project directory to run; defaults to the directory the server was started in',
      },
      attach: {
        type: 'boolean',
        description:
          'Only listen for an engine started by hand and report the MOYU_ENGINE_DEBUG_WS value it ' +
          'must be started with; nothing is launched or waited for',
        default: false,
      },
      listen: {
        type: 'number',
        description: 'Port an attach session listens on; defaults to 6321',
        default: DEFAULT_ATTACH_PORT,
      },
    },
  },
  run: async (args) => {
    stopSession();

    if (args.attach === true) {
      session = await startDebugSession({
        attachPort: typeof args.listen === 'number' ? args.listen : DEFAULT_ATTACH_PORT,
      });

      return text(
        [
          `Listening for an engine on port ${String(session.host.port)} (attach).`,
          'Start the engine yourself and point it at this endpoint:',
          `  MOYU_ENGINE_DEBUG_WS=${String(session.attachEndpoint)}`,
          'Then call any other debug tool to work with it.',
        ].join('\n'),
      );
    }

    const projectRoot = resolveProjectRoot(args);
    const web = args.web === true;

    session = await startDebugSession({
      projectRoot,
      web,
      port: typeof args.port === 'number' ? args.port : undefined,
      entry: typeof args.entry === 'string' ? args.entry : undefined,
    });

    await session.host.waitForEngine(DEFAULT_CONNECT_TIMEOUT_MS);

    // The engine reports readiness on its own schedule; a session that never becomes
    // ready is still worth reporting, because its logs explain why.
    let ready = true;
    try {
      await session.host.waitForReady(START_READY_TIMEOUT_MS);
    } catch {
      ready = false;
    }

    const lines = [
      `Engine started (${session.mode}).`,
      session.url === undefined ? '' : `Open this page to run it: ${session.url}`,
      ready ? '' : 'The engine has not finished starting up yet; engine logs may explain why.',
    ];

    return text(lines.filter((line) => line !== '').join('\n'));
  },
};

const stopTool: ToolDefinition = {
  name: 'debug_stop',
  title: 'Stop the debug session',
  description:
    'Stop the debug session started by debug_start. An attach session only closes the listener; the ' +
    'engine keeps running.',
  inputSchema: { type: 'object', properties: {} },
  run: async () => {
    const hadSession = session !== null;
    stopSession();

    return text(hadSession ? 'Session stopped.' : 'No session was running.');
  },
};

// ---------------------------------------------------------------------------
// Inspection
// ---------------------------------------------------------------------------

const stateTool: ToolDefinition = {
  name: 'debug_state',
  title: 'Read engine state',
  description:
    'Read the engine entry file, platform, version, surface size, node count, uptime, and whether it ' +
    'has finished starting up.',
  inputSchema: { type: 'object', properties: {} },
  run: async () => text(JSON.stringify(await request('engine:state'), null, 2)),
};

const evalTool: ToolDefinition = {
  name: 'debug_eval',
  title: 'Evaluate JavaScript',
  description:
    'Run JavaScript in the engine and return the result. Native engines share one global scope across ' +
    'calls, so values kept on globalThis persist between calls. An object literal needs parentheses. ' +
    'Use this to reach anything the other tools do not cover.',
  inputSchema: {
    type: 'object',
    properties: {
      code: { type: 'string', description: 'JavaScript to evaluate' },
      timeoutMs: {
        type: 'number',
        description: 'Milliseconds the engine may spend on the evaluation',
        default: 5000,
      },
    },
    required: ['code'],
  },
  run: async (args) => {
    const result = await request('engine:eval', {
      code: requireString(args, 'code'),
      timeoutMs: typeof args.timeoutMs === 'number' ? args.timeoutMs : undefined,
    });

    const value = result.value === undefined ? String(result.repr) : JSON.stringify(result.value, null, 2);
    const note = result.truncated === true ? '\n(result exceeded the size limit and is only shown in short form)' : '';

    return text(`${value}${note}`);
  },
};

const logsTool: ToolDefinition = {
  name: 'debug_logs',
  title: 'Read engine logs',
  description:
    'Read buffered engine logs, oldest first. Use sinceSeq with the returned nextSeq to fetch only what ' +
    'is new since the last call.',
  inputSchema: {
    type: 'object',
    properties: {
      level: {
        type: 'string',
        enum: ['error', 'warn', 'info', 'debug', 'trace'],
        description: 'Only entries at this level or more severe',
      },
      limit: { type: 'number', description: 'Maximum number of entries to return' },
      sinceSeq: { type: 'number', description: 'Only entries newer than this sequence number' },
    },
  },
  run: async (args) => {
    const result = await request('engine:logs', {
      level: typeof args.level === 'string' ? args.level : undefined,
      limit: typeof args.limit === 'number' ? args.limit : undefined,
      sinceSeq: typeof args.sinceSeq === 'number' ? args.sinceSeq : undefined,
    });

    const lines = (result.entries as { timestampMs: number; level: string; target: string; message: string }[]).map(
      (entry) =>
        `${new Date(entry.timestampMs).toISOString().slice(11, 23)} ${entry.level.toUpperCase()} ${entry.target}: ${entry.message}`,
    );

    lines.push(`nextSeq: ${String(result.nextSeq)}${result.dropped > 0 ? ` (dropped ${String(result.dropped)})` : ''}`);

    return text(lines.join('\n'));
  },
};

const treeTool: ToolDefinition = {
  name: 'debug_tree',
  title: 'Read the node tree',
  description:
    'List nodes and their children, starting from the root unless a nodeId is given. Each line shows ' +
    'the node id, its type, and its label; use debug_props to inspect one node.',
  inputSchema: {
    type: 'object',
    properties: {
      nodeId: { type: 'number', description: 'Node to start from; defaults to the root node' },
      depth: { type: 'number', description: 'Levels of children to include', default: 4 },
    },
  },
  run: async (args) => {
    const result = await request('engine:tree', {
      nodeId: typeof args.nodeId === 'number' ? args.nodeId : undefined,
      depth: typeof args.depth === 'number' ? args.depth : undefined,
    });

    const lines: string[] = [];
    collectNode(result.node, 0, lines);

    return text(lines.join('\n'));
  },
};

const propsTool: ToolDefinition = {
  name: 'debug_props',
  title: 'Read node properties',
  description:
    'Read what JavaScript asked of one node, next to the values the engine derived from it. Comparing ' +
    'the two shows whether an intended change took effect; the derived size and bounds are in stage pixels.',
  inputSchema: {
    type: 'object',
    properties: { nodeId: { type: 'number', description: 'Node id, as listed by debug_tree' } },
    required: ['nodeId'],
  },
  run: async (args) => {
    const nodeId = args.nodeId;

    if (typeof nodeId !== 'number') {
      throw new Error('nodeId is required');
    }

    return text(JSON.stringify(await request('engine:props', { nodeId }), null, 2));
  },
};

const screenshotTool: ToolDefinition = {
  name: 'debug_screenshot',
  title: 'Capture the stage',
  description:
    'Capture the next rendered frame. Returns the image itself, so it shows what the game currently ' +
    'looks like; use maxWidth and maxHeight to scale a large stage down.',
  inputSchema: {
    type: 'object',
    properties: {
      maxWidth: { type: 'number', description: 'Largest width to return; needs maxHeight as well' },
      maxHeight: { type: 'number', description: 'Largest height to return; needs maxWidth as well' },
    },
  },
  run: async (args) => {
    const result = await request('engine:screenshot', {
      maxWidth: typeof args.maxWidth === 'number' ? args.maxWidth : undefined,
      maxHeight: typeof args.maxHeight === 'number' ? args.maxHeight : undefined,
    });

    const summary = `${String(result.width)}x${String(result.height)} ${String(result.format)}`;

    return {
      content: [
        { type: 'image', data: String(result.data), mimeType: `image/${String(result.format)}` },
        { type: 'text', text: summary },
      ],
    };
  },
};

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

const mouseTool: ToolDefinition = {
  name: 'debug_mouse',
  title: 'Send a mouse action',
  description:
    'Simulate a mouse action (move, down, up, click or wheel) at a stage position or at a node. The ' +
    'action goes through the real input pipeline, so hover, pointer pairing and gestures behave as ' +
    'they do for a user. Give x and y in stage logical coordinates, or a nodeId to aim at the center ' +
    'of a node. Returns the node that was hit, the bubble chain, and the events dispatched.',
  inputSchema: {
    type: 'object',
    properties: {
      action: {
        type: 'string',
        enum: ['move', 'down', 'up', 'click', 'wheel'],
        description: 'Mouse action to perform',
      },
      x: {
        type: 'number',
        description: 'Stage logical X coordinate; use with y, or use nodeId instead',
      },
      y: {
        type: 'number',
        description: 'Stage logical Y coordinate; use with x, or use nodeId instead',
      },
      nodeId: {
        type: 'number',
        description: 'Target node id; the action lands at the center of its bounds',
      },
      button: {
        type: 'string',
        enum: ['left', 'right', 'middle', 'back', 'forward'],
        description: 'Mouse button for down, up and click; defaults to left',
      },
      deltaX: { type: 'number', description: 'Horizontal scroll amount for wheel; defaults to 0' },
      deltaY: { type: 'number', description: 'Vertical scroll amount for wheel; defaults to 0' },
      mode: {
        type: 'string',
        enum: ['line', 'pixel'],
        description: 'Unit of the wheel deltas; defaults to line',
      },
    },
    required: ['action'],
  },
  run: async (args) => text(JSON.stringify(await request('engine:mouse', args), null, 2)),
};

const touchTool: ToolDefinition = {
  name: 'debug_touch',
  title: 'Send a touch phase',
  description:
    'Simulate one touch phase (start, move, end or cancel). A touch is a session: start comes first ' +
    'and later phases may omit the position to keep the touch where it is. Give x and y in stage ' +
    'logical coordinates, or a nodeId to aim at the center of a node. Returns the node that was hit, ' +
    'the bubble chain, and the events dispatched.',
  inputSchema: {
    type: 'object',
    properties: {
      action: {
        type: 'string',
        enum: ['start', 'move', 'end', 'cancel'],
        description: 'Touch phase to perform',
      },
      x: {
        type: 'number',
        description: 'Stage logical X coordinate; required for start, optional afterwards',
      },
      y: {
        type: 'number',
        description: 'Stage logical Y coordinate; required for start, optional afterwards',
      },
      nodeId: {
        type: 'number',
        description: 'Target node id; the touch starts at the center of its bounds',
      },
      identifier: {
        type: 'number',
        description: 'Touch point identifier; defaults to 0',
      },
    },
    required: ['action'],
  },
  run: async (args) => text(JSON.stringify(await request('engine:touch', args), null, 2)),
};

const keyTool: ToolDefinition = {
  name: 'debug_key',
  title: 'Send a keyboard event',
  description:
    'Simulate a keyboard event (down, up or press). Keyboard events go to the engine as a whole, ' +
    'like real key presses. key is the event.key value such as Escape, Enter or a; code defaults to ' +
    'the same value.',
  inputSchema: {
    type: 'object',
    properties: {
      action: {
        type: 'string',
        enum: ['down', 'up', 'press'],
        description: 'Keyboard action to perform',
      },
      key: {
        type: 'string',
        description: 'The event.key value, such as Escape, Enter or a',
      },
      code: { type: 'string', description: 'Physical key name; defaults to key' },
      repeat: { type: 'boolean', description: 'Whether this is a key repeat; defaults to false' },
      ctrlKey: { type: 'boolean', description: 'Whether Ctrl is held; defaults to false' },
      shiftKey: { type: 'boolean', description: 'Whether Shift is held; defaults to false' },
      altKey: { type: 'boolean', description: 'Whether Alt is held; defaults to false' },
      metaKey: { type: 'boolean', description: 'Whether Meta is held; defaults to false' },
    },
    required: ['action', 'key'],
  },
  run: async (args) => text(JSON.stringify(await request('engine:key', args), null, 2)),
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Send one request to the engine. Inspection is only meaningful once the project script
 * has run, so every request waits for that first; it returns immediately when the engine
 * is already up.
 */
async function request(type: string, payload?: Record<string, unknown>) {
  const current = requireSession();

  await current.host.waitForReady(READY_TIMEOUT_MS);

  return current.host.request(type, payload);
}

function requireSession(): DebugSession {
  if (!session) {
    throw new Error('No session is running. Call debug_start first.');
  }

  return session;
}

function resolveProjectRoot(args: Record<string, unknown>): string {
  if (typeof args.projectRoot === 'string') {
    return args.projectRoot;
  }

  const found = findProjectRoot();

  if (!found) {
    throw new Error(
      'Could not find a Moyu project (no index.json in this directory or above it). ' +
        'Pass projectRoot to point at one.',
    );
  }

  log.info(`Using project at ${found}`);

  return found;
}

function requireString(args: Record<string, unknown>, name: string): string {
  const value = args[name];

  if (typeof value !== 'string') {
    throw new Error(`${name} is required`);
  }

  return value;
}

function text(value: string): ToolResult {
  return { content: [{ type: 'text', text: value }] as ToolContent[] };
}

interface TreeNode {
  id: number;
  type: string;
  label: string;
  visible: boolean;
  children: TreeNode[];
}

function collectNode(node: TreeNode, indent: number, lines: string[]) {
  const label = node.label === '' ? '' : ` ${JSON.stringify(node.label)}`;
  const hidden = node.visible ? '' : ' (hidden)';

  lines.push(`${'  '.repeat(indent)}#${node.id} ${node.type}${label}${hidden}`);

  for (const child of node.children) {
    collectNode(child, indent + 1, lines);
  }
}
