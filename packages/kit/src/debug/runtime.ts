import { executePluginCommand } from '../moyu';
import { replaceUiData } from '../ui';
import {
  resetRuntimeDebugVariableMonitor,
  setRuntimeDebugVariableEmissionSuspended,
  setRuntimeDebugVariableMessageSender,
  syncRuntimeDebugVariablesSnapshot,
  type RuntimeDebugVariableMessagePayload,
  type RuntimeDebugVariablesSnapshotReason,
} from './variableMonitor';
import {
  startDebugSession,
  stopDebugSession,
  type CombinedCheckpoint,
  type DebugSessionController,
} from './session';

interface DebugParams {
  debug?: boolean;
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
  story?: string;
  boundary?: 'before' | 'after';
  strategy?: 'fast-forward' | 'warp';
}

interface RouteRequestMessage {
  type: 'route:request';
  sessionId: string;
  requestId: number;
  page: string;
  params?: Record<string, unknown>;
}

interface StoryReplaceMessage {
  type: 'story:replace';
  sessionId: string;
  requestId: number;
  story: string;
  content: string;
  seekMode?: 'off' | 'fast-forward' | 'warp';
  targetMarkerId?: string;
}

interface UiReplaceMessage {
  type: 'ui:replace';
  sessionId: string;
  data: unknown;
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

interface RouteDoneMessage {
  type: 'route:done';
  sessionId: string;
  requestId: number;
  page: string;
}

interface RouteErrorMessage {
  type: 'route:error';
  sessionId: string;
  requestId: number;
  page: string;
  error: string;
}

interface StoryReplaceDoneMessage {
  type: 'story:replace:done';
  sessionId: string;
  requestId: number;
  story: string;
}

interface StoryReplaceErrorMessage {
  type: 'story:replace:error';
  sessionId: string;
  requestId: number;
  story: string;
  error: string;
}

type RuntimeDebugIncomingMessage =
  | JumpRequestMessage
  | RouteRequestMessage
  | StoryReplaceMessage
  | UiReplaceMessage
  | { type?: string; sessionId?: string; [key: string]: unknown };

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

type RuntimeDebugRequestMessage = JumpRequestMessage | RouteRequestMessage | StoryReplaceMessage | UiReplaceMessage;

type RuntimeDebugOutgoingMessage =
  | MarkerEnterMessage
  | JumpDoneMessage
  | JumpErrorMessage
  | RouteDoneMessage
  | RouteErrorMessage
  | StoryReplaceDoneMessage
  | StoryReplaceErrorMessage
  | (RuntimeDebugVariableMessagePayload & { sessionId: string });

let currentConnection: RuntimeDebugConnection | null = null;
let currentController: DebugSessionController | null = null;
let bufferedRequestMessages: RuntimeDebugRequestMessage[] = [];

function getRuntimeWebSocket(): RuntimeWebSocketConstructor | null {
  return (globalThis as { WebSocket?: RuntimeWebSocketConstructor }).WebSocket ?? null;
}

function isJumpRequestMessage(message: RuntimeDebugIncomingMessage): message is JumpRequestMessage {
  return (
    message.type === 'jump:request' &&
    typeof message.sessionId === 'string' &&
    typeof message.requestId === 'number' &&
    typeof message.markerId === 'string' &&
    (message.story === undefined || typeof message.story === 'string') &&
    (message.boundary === undefined || message.boundary === 'before' || message.boundary === 'after') &&
    (message.strategy === undefined || message.strategy === 'fast-forward' || message.strategy === 'warp')
  );
}

function isRouteRequestMessage(message: RuntimeDebugIncomingMessage): message is RouteRequestMessage {
  return (
    message.type === 'route:request' &&
    typeof message.sessionId === 'string' &&
    typeof message.requestId === 'number' &&
    typeof message.page === 'string' &&
    (message.params === undefined ||
      (typeof message.params === 'object' && message.params !== null && !Array.isArray(message.params)))
  );
}

function isStoryReplaceMessage(message: RuntimeDebugIncomingMessage): message is StoryReplaceMessage {
  return (
    message.type === 'story:replace' &&
    typeof message.sessionId === 'string' &&
    typeof message.requestId === 'number' &&
    typeof message.story === 'string' &&
    typeof message.content === 'string' &&
    (message.seekMode === undefined || message.seekMode === 'off' || message.seekMode === 'fast-forward' || message.seekMode === 'warp') &&
    (message.targetMarkerId === undefined || typeof message.targetMarkerId === 'string')
  );
}

function isUiReplaceMessage(message: RuntimeDebugIncomingMessage): message is UiReplaceMessage {
  return message.type === 'ui:replace' && typeof message.sessionId === 'string';
}

function sendRuntimeDebugMessage(
  socket: RuntimeWebSocket,
  message: RuntimeDebugOutgoingMessage,
): void {
  const RuntimeWebSocket = getRuntimeWebSocket();
  if (!RuntimeWebSocket || socket.readyState !== RuntimeWebSocket.OPEN) {
    return;
  }

  socket.send(JSON.stringify(message));
}

function clearBufferedRequestMessages() {
  bufferedRequestMessages = [];
}

function bufferRequestMessage(message: RuntimeDebugRequestMessage) {
  bufferedRequestMessages.push(message);
}

function flushBufferedRequestMessages(socket: RuntimeWebSocket, sessionId: string) {
  const messages = bufferedRequestMessages;
  bufferedRequestMessages = [];

  for (const message of messages) {
    handleRuntimeDebugRequest(socket, sessionId, message);
  }
}

async function syncRuntimeDebugVariablesSnapshotSafe(reason: RuntimeDebugVariablesSnapshotReason): Promise<void> {
  try {
    await syncRuntimeDebugVariablesSnapshot(reason);
  } catch (error) {
    console.error('[debug] failed to sync runtime variables snapshot:', error);
  }
}

function handleRuntimeDebugRequest(
  socket: RuntimeWebSocket,
  sessionId: string,
  message: RuntimeDebugRequestMessage,
) {
  if (isUiReplaceMessage(message)) {
    try {
      replaceUiData(message.data);
    } catch (error) {
      console.error('[debug] failed to replace runtime UI data:', error);
    }
    return;
  }

  const controller = currentController;
  if (!controller) {
    bufferRequestMessage(message);
    return;
  }

  if (isJumpRequestMessage(message)) {
    setRuntimeDebugVariableEmissionSuspended(true);
    const runJump =
      message.strategy === 'warp'
        ? controller.warp({
            markerId: message.markerId,
            story: message.story,
            boundary: message.boundary,
          }).then(() => true)
        : controller.restoreCheckpoint(message.markerId, {
            story: message.story,
          });

    void runJump
      .then(async (restored) => {
        if (!restored) {
          sendRuntimeDebugMessage(socket, {
            type: 'jump:error',
            sessionId,
            requestId: message.requestId,
            markerId: message.markerId,
            error: 'Checkpoint not found',
          });
          await syncRuntimeDebugVariablesSnapshotSafe('jump-error');
          return;
        }

        sendRuntimeDebugMessage(socket, {
          type: 'jump:done',
          sessionId,
          requestId: message.requestId,
          markerId: message.markerId,
        });
        await syncRuntimeDebugVariablesSnapshotSafe('jump-done');
      })
      .catch(async (error) => {
        sendRuntimeDebugMessage(socket, {
          type: 'jump:error',
          sessionId,
          requestId: message.requestId,
          markerId: message.markerId,
          error: error instanceof Error ? error.message : String(error),
        });
        await syncRuntimeDebugVariablesSnapshotSafe('jump-error');
      })
      .finally(() => {
        setRuntimeDebugVariableEmissionSuspended(false);
      });
    return;
  }

  if (isStoryReplaceMessage(message)) {
    setRuntimeDebugVariableEmissionSuspended(true);
    void controller
      .replaceStory({
        story: message.story,
        content: message.content,
        seekMode: message.seekMode,
        targetMarkerId: message.targetMarkerId,
      })
      .then(async () => {
        sendRuntimeDebugMessage(socket, {
          type: 'story:replace:done',
          sessionId,
          requestId: message.requestId,
          story: message.story,
        });
        await syncRuntimeDebugVariablesSnapshotSafe('story-replace-done');
      })
      .catch(async (error) => {
        sendRuntimeDebugMessage(socket, {
          type: 'story:replace:error',
          sessionId,
          requestId: message.requestId,
          story: message.story,
          error: error instanceof Error ? error.message : String(error),
        });
        await syncRuntimeDebugVariablesSnapshotSafe('story-replace-error');
      })
      .finally(() => {
        setRuntimeDebugVariableEmissionSuspended(false);
      });
    return;
  }

  setRuntimeDebugVariableEmissionSuspended(true);
  void controller
    .switchRoute(message.page, message.params)
    .then(async () => {
      sendRuntimeDebugMessage(socket, {
        type: 'route:done',
        sessionId,
        requestId: message.requestId,
        page: message.page,
      });
      await syncRuntimeDebugVariablesSnapshotSafe('route-done');
    })
    .catch(async (error) => {
      sendRuntimeDebugMessage(socket, {
        type: 'route:error',
        sessionId,
        requestId: message.requestId,
        page: message.page,
        error: error instanceof Error ? error.message : String(error),
      });
      await syncRuntimeDebugVariablesSnapshotSafe('route-error');
    })
    .finally(() => {
      setRuntimeDebugVariableEmissionSuspended(false);
    });
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
      debug: parsed.debug === true,
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
  resetRuntimeDebugVariableMonitor();
  clearBufferedRequestMessages();
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
  if (params.debug !== true || !params.debugSessionId || !params.debugWsUrl) {
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
        setRuntimeDebugVariableMessageSender((message) => {
          sendRuntimeDebugMessage(socket, {
            ...message,
            sessionId,
          });
        });
        void syncRuntimeDebugVariablesSnapshotSafe('init');
        flushBufferedRequestMessages(socket, sessionId);
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
    resetRuntimeDebugVariableMonitor();
    clearBufferedRequestMessages();
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
        handleRuntimeDebugRequest(socket, sessionId, message);
        return;
      }

      if (isRouteRequestMessage(message)) {
        handleRuntimeDebugRequest(socket, sessionId, message);
        return;
      }

      if (isStoryReplaceMessage(message)) {
        handleRuntimeDebugRequest(socket, sessionId, message);
        return;
      }

      if (isUiReplaceMessage(message)) {
        handleRuntimeDebugRequest(socket, sessionId, message);
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
