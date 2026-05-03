import { executePluginCommand } from '../moyu';
import {
  startDebugSession,
  stopDebugSession,
  type CombinedCheckpoint,
  type DebugSessionController,
} from './session';

interface DebugParams {
  debugSessionId?: string;
  debugWsUrl?: string;
}

interface MarkerEnterMessage {
  type: 'marker:enter';
  sessionId: string;
  markerId: string;
  story: string;
  paragraph: string;
}

interface JumpRequestMessage {
  type: 'jump:request';
  sessionId: string;
  requestId: number;
  markerId: string;
}

interface JumpDoneMessage {
  type: 'jump:done';
  sessionId: string;
  requestId: number;
  markerId: string;
}

interface JumpErrorMessage {
  type: 'jump:error';
  sessionId: string;
  requestId: number;
  markerId: string;
  error: string;
}

type RuntimeDebugIncomingMessage = JumpRequestMessage | { type?: string; sessionId?: string; [key: string]: unknown };

interface RuntimeWebSocketMessageEvent {
  data: unknown;
}

interface RuntimeWebSocket {
  readonly readyState: number;
  onopen: (() => void) | null;
  onclose: (() => void) | null;
  onerror: ((event: unknown) => void) | null;
  onmessage: ((event: RuntimeWebSocketMessageEvent) => void) | null;
  send(data: string): void;
  close(): void;
}

interface RuntimeWebSocketConstructor {
  readonly OPEN: number;
  readonly CONNECTING: number;
  new (url: string): RuntimeWebSocket;
}

type RuntimeDebugConnection = {
  sessionId: string;
  socket: RuntimeWebSocket;
};

let currentConnection: RuntimeDebugConnection | null = null;
let currentController: DebugSessionController | null = null;

function getRuntimeWebSocket(): RuntimeWebSocketConstructor | null {
  return (globalThis as { WebSocket?: RuntimeWebSocketConstructor }).WebSocket ?? null;
}

function isJumpRequestMessage(message: RuntimeDebugIncomingMessage): message is JumpRequestMessage {
  return (
    message.type === 'jump:request' &&
    typeof message.sessionId === 'string' &&
    typeof message.requestId === 'number' &&
    typeof message.markerId === 'string'
  );
}

function sendRuntimeDebugMessage(
  socket: RuntimeWebSocket,
  message: MarkerEnterMessage | JumpDoneMessage | JumpErrorMessage,
): void {
  const RuntimeWebSocket = getRuntimeWebSocket();
  if (!RuntimeWebSocket || socket.readyState !== RuntimeWebSocket.OPEN) {
    return;
  }

  socket.send(JSON.stringify(message));
}

async function readDebugParams(): Promise<DebugParams> {
  const rawParams = (await executePluginCommand('system', {
    subCommand: 'getParams',
  })) as string | undefined;

  if (!rawParams) {
    return {};
  }

  try {
    const parsed = JSON.parse(rawParams) as Record<string, unknown>;
    return {
      debugSessionId: typeof parsed.debugSessionId === 'string' ? parsed.debugSessionId : undefined,
      debugWsUrl: typeof parsed.debugWsUrl === 'string' ? parsed.debugWsUrl : undefined,
    };
  } catch {
    return {};
  }
}

export async function stopRuntimeDebugSession(): Promise<void> {
  const connection = currentConnection;
  currentConnection = null;
  currentController = null;
  const RuntimeWebSocket = getRuntimeWebSocket();

  await stopDebugSession();

  if (!connection) {
    return;
  }

  if (
    RuntimeWebSocket &&
    (connection.socket.readyState === RuntimeWebSocket.OPEN ||
      connection.socket.readyState === RuntimeWebSocket.CONNECTING)
  ) {
    connection.socket.close();
  }
}

export async function startRuntimeDebugSession(): Promise<void> {
  await stopRuntimeDebugSession();

  const params = await readDebugParams();
  if (!params.debugSessionId || !params.debugWsUrl) {
    return;
  }

  const RuntimeWebSocket = getRuntimeWebSocket();
  if (!RuntimeWebSocket) {
    return;
  }

  const socket = new RuntimeWebSocket(params.debugWsUrl);
  const sessionId = params.debugSessionId;
  currentConnection = { sessionId, socket };

  socket.onopen = () => {
    if (currentConnection?.socket !== socket) {
      socket.close();
      return;
    }

    void startDebugSession({
      onMarkerEnter(checkpoint: CombinedCheckpoint<unknown>) {
        if (!checkpoint.cursor.markerId) {
          return;
        }

        sendRuntimeDebugMessage(socket, {
          type: 'marker:enter',
          sessionId,
          markerId: checkpoint.cursor.markerId,
          story: checkpoint.cursor.story,
          paragraph: checkpoint.cursor.paragraph,
        });
      },
      onError(error) {
        console.error('[debug] runtime debug session error:', error);
      },
    })
      .then((controller) => {
        currentController = controller;
      })
      .catch((error) => {
        console.error('[debug] failed to start runtime debug session:', error);
        socket.close();
      });
  };

  socket.onclose = () => {
    if (currentConnection?.socket !== socket) {
      return;
    }

    currentConnection = null;
    currentController = null;
    void stopDebugSession();
  };

  socket.onerror = (event: unknown) => {
    if (currentConnection?.socket !== socket) {
      return;
    }

    console.error('[debug] runtime websocket error:', event);
  };

  socket.onmessage = (event: RuntimeWebSocketMessageEvent) => {
    if (currentConnection?.socket !== socket) {
      return;
    }

    try {
      const message = JSON.parse(String(event.data)) as RuntimeDebugIncomingMessage;
      if (message.sessionId && message.sessionId !== sessionId) {
        return;
      }

      if (isJumpRequestMessage(message)) {
        const controller = currentController;
        if (!controller) {
          sendRuntimeDebugMessage(socket, {
            type: 'jump:error',
            sessionId,
            requestId: message.requestId,
            markerId: message.markerId,
            error: 'Debug session is not ready',
          });
          return;
        }

        void controller
          .restoreCheckpoint(message.markerId)
          .then((restored) => {
            if (!restored) {
              sendRuntimeDebugMessage(socket, {
                type: 'jump:error',
                sessionId,
                requestId: message.requestId,
                markerId: message.markerId,
                error: 'Checkpoint not found',
              });
              return;
            }

            sendRuntimeDebugMessage(socket, {
              type: 'jump:done',
              sessionId,
              requestId: message.requestId,
              markerId: message.markerId,
            });
          })
          .catch((error) => {
            sendRuntimeDebugMessage(socket, {
              type: 'jump:error',
              sessionId,
              requestId: message.requestId,
              markerId: message.markerId,
              error: error instanceof Error ? error.message : String(error),
            });
          });
        return;
      }

      if (message.type && message.type !== 'session:init') {
        console.debug('[debug] runtime received message:', message);
      }
    } catch (error) {
      console.error('[debug] failed to parse runtime debug message:', error);
    }
  };
}
