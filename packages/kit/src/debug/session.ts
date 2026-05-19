import { useSnapshot, proxy } from 'valtio';
import type { ExecutionCursor } from '../bindings/ExecutionCursor';
import type { MarkerEnter } from '../bindings/MarkerEnter';
import type { ScenarioCommand } from '../bindings/ScenarioCommand';
import type { FastForwardOptions } from '../hooks/useStage';
import { addEventListener } from '../events';
import { executePluginCommand } from '../moyu';
import { getNavigator } from '../components/navigation';

export interface AppStateAdapter<TState = unknown> {
  capture(): TState | Promise<TState>;
  restore(state: TState): void | Promise<void>;
  switchPage?(page: string, params?: Record<string, unknown>): void | Promise<void>;
  switchOverlay?(overlay: string, params?: Record<string, unknown>): void | Promise<void>;
  enterFastForwardMode?(options?: FastForwardOptions): void | Promise<void>;
  exitFastForwardMode?(): void | Promise<void>;
  restartStoryFromHead?(options: DebugStoryStartOptions): void | Promise<void>;
  restartCurrentStoryFromHead?(): void | Promise<void>;
}

export type CombinedCheckpoint<TState = unknown> = {
  cursor: ExecutionCursor;
  appState: TState;
};

export interface DebugSessionConfig {
  onMarkerEnter?: (checkpoint: CombinedCheckpoint<unknown>) => void | Promise<void>;
  onError?: (error: unknown) => void | Promise<void>;
}

export interface DebugSessionController {
  restoreCheckpoint(markerId: string, options?: DebugJumpTargetOptions): Promise<boolean>;
  warp(options: DebugWarpOptions): Promise<void>;
  switchRoute(routeName: string, params?: Record<string, unknown>): Promise<void>;
}

export interface DebugStoryStartOptions {
  story: string;
  entry?: string;
}

export interface DebugJumpTargetOptions {
  story?: string;
}

export type DebugWarpBoundary = 'before' | 'after';

export interface DebugWarpOptions extends DebugJumpTargetOptions {
  markerId: string;
  boundary?: DebugWarpBoundary;
}

type AnyAdapter = AppStateAdapter<any>;

type ScenarioCommandPayload =
  | ScenarioCommand
  | {
      subCommand: 'warp';
      markerId: string;
      boundary: DebugWarpBoundary;
    };

interface FastForwardCheckpointLookup {
  markerExists: boolean;
  checkpointKey?: string | null;
}

type ActiveSeek = {
  targetMarkerId: string;
  resolve: () => void;
  reject: (error: unknown) => void;
};

type DebugSessionState = {
  active: boolean;
  checkpoints: Record<string, CombinedCheckpoint<unknown>>;
  currentCursor: ExecutionCursor | null;
  currentStory: string | null;
  restoring: boolean;
  lastError: string | null;
};

const debugState = proxy<DebugSessionState>({
  active: false,
  checkpoints: {},
  currentCursor: null,
  currentStory: null,
  restoring: false,
  lastError: null,
});

let appStateAdapter: AnyAdapter | null = null;
let currentConfig: Required<DebugSessionConfig> = {
  onMarkerEnter: () => {},
  onError: () => {},
};
let disposeMarkerListener: (() => void) | null = null;
let disposeFinishedListener: (() => void) | null = null;
let disposeWarpFinishedListener: (() => void) | null = null;
let debugOperationQueue: Promise<void> = Promise.resolve();
let activeFastForward: ActiveSeek | null = null;
let activeWarp: ActiveSeek | null = null;

function enqueueDebugOperation<T>(task: () => Promise<T>): Promise<T> {
  const nextTask = debugOperationQueue.then(task, task);
  debugOperationQueue = nextTask.then(
    () => undefined,
    () => undefined,
  );
  return nextTask;
}

function executeScenarioCommand<T>(payload: ScenarioCommandPayload): Promise<T> {
  return Promise.resolve(executePluginCommand('scenario', payload as never) as T);
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function isPromiseLike<T>(value: T | PromiseLike<T> | undefined): value is PromiseLike<T> {
  return !!value && typeof (value as PromiseLike<T>).then === 'function';
}

function reportDebugSessionError(error: unknown) {
  debugState.lastError = toErrorMessage(error);
  void Promise.resolve(currentConfig.onError(error));
}

function clearLocalRuntimeState() {
  debugState.checkpoints = {};
  debugState.currentCursor = null;
  debugState.currentStory = null;
  debugState.restoring = false;
  debugState.lastError = null;
}

async function restartJumpStoryFromHead(targetStory: string | undefined, currentStory: string | null) {
  debugState.checkpoints = {};
  debugState.currentCursor = null;
  debugState.currentStory = null;

  if (targetStory) {
    if (appStateAdapter?.restartStoryFromHead) {
      await appStateAdapter.restartStoryFromHead({
        story: targetStory,
        entry: 'entry',
      });
    } else if (currentStory === targetStory && appStateAdapter?.restartCurrentStoryFromHead) {
      await appStateAdapter.restartCurrentStoryFromHead();
    } else {
      throw new Error('Restarting the target story from head is not supported by the app state adapter');
    }

    debugState.currentCursor = await executeScenarioCommand<ExecutionCursor | null>({
      subCommand: 'getExecutionCursor',
    });
    debugState.currentStory = debugState.currentCursor?.story ?? targetStory;
    return;
  }

  if (!appStateAdapter?.restartCurrentStoryFromHead) {
    throw new Error('Restarting the current story from head is not supported by the app state adapter');
  }

  await appStateAdapter.restartCurrentStoryFromHead();
  debugState.currentCursor = null;
}

async function doSettleActiveSeek(op: ActiveSeek | null, clearSlot: () => void, error?: unknown) {
  if (!op) {
    return;
  }

  clearSlot();

  try {
    await appStateAdapter?.exitFastForwardMode?.();
  } catch (exitError) {
    if (error === undefined) {
      error = exitError;
    }
  }

  if (error === undefined) {
    op.resolve();
    return;
  }

  op.reject(error);
}

async function settleActiveFastForward(error?: unknown) {
  await doSettleActiveSeek(activeFastForward, () => {
    activeFastForward = null;
  }, error);
}

async function settleActiveWarp(error?: unknown) {
  await doSettleActiveSeek(activeWarp, () => {
    activeWarp = null;
  }, error);
}

export async function clearLocalDebugSessionRuntimeState(): Promise<void> {
  await settleActiveFastForward(new Error('Debug session runtime state cleared'));
  await settleActiveWarp(new Error('Debug session runtime state cleared'));
  clearLocalRuntimeState();
}

async function clearRemoteCheckpoints() {
  await executeScenarioCommand<void>({ subCommand: 'clearCheckpoints' });
}

async function syncLocalWarpCheckpoint(targetMarkerId: string) {
  const cursor = await executeScenarioCommand<ExecutionCursor | null>({
    subCommand: 'getExecutionCursor',
  });

  const checkpointCursor = cursor
    ? {
        story: cursor.story,
        paragraph: cursor.paragraph,
        markerId: targetMarkerId,
      }
    : debugState.currentCursor
      ? {
          story: debugState.currentCursor.story,
          paragraph: debugState.currentCursor.paragraph,
          markerId: targetMarkerId,
        }
      : null;

  if (!checkpointCursor) {
    return;
  }

  const checkpoint: CombinedCheckpoint<unknown> = {
    cursor: checkpointCursor,
    appState: appStateAdapter ? await appStateAdapter.capture() : undefined,
  };

  debugState.currentCursor = checkpointCursor;
  debugState.currentStory = checkpointCursor.story;
  debugState.checkpoints[targetMarkerId] = checkpoint;
  await currentConfig.onMarkerEnter(checkpoint);
}

function captureCheckpointAppStateSync(): unknown {
  if (!appStateAdapter) {
    return undefined;
  }

  const appState = appStateAdapter.capture();
  if (isPromiseLike(appState)) {
    throw new Error('Async app state capture is not supported for debug checkpoints');
  }

  return appState;
}

function notifyMarkerEnter(checkpoint: CombinedCheckpoint<unknown>) {
  try {
    const result = currentConfig.onMarkerEnter(checkpoint);
    if (isPromiseLike(result)) {
      void result.catch(reportDebugSessionError);
    }
  } catch (error) {
    reportDebugSessionError(error);
  }
}

async function prepareJumpStart(markerId: string, options?: DebugJumpTargetOptions): Promise<boolean> {
  const currentStory = debugState.currentStory ?? debugState.currentCursor?.story ?? null;
  const targetStory = options?.story;
  let restartedFromHead = false;

  if (targetStory && targetStory !== currentStory) {
    await restartJumpStoryFromHead(targetStory, currentStory);
    restartedFromHead = true;
  }

  const checkpoint = debugState.checkpoints[markerId];
  if (!restartedFromHead && checkpoint && (!targetStory || checkpoint.cursor.story === targetStory)) {
    const restored = await executeScenarioCommand<boolean>({
      subCommand: 'restoreCheckpoint',
      key: markerId,
    });
    if (!restored) {
      throw new Error(`Failed to restore checkpoint ${markerId}`);
    }

    if (appStateAdapter) {
      await appStateAdapter.restore(checkpoint.appState);
    }

    debugState.currentCursor = checkpoint.cursor;
    debugState.currentStory = checkpoint.cursor.story;
    return false;
  }

  const lookup = await executeScenarioCommand<FastForwardCheckpointLookup>({
    subCommand: 'getFastForwardCheckpoint',
    key: markerId,
  });

  if (!lookup.markerExists) {
    throw new Error(
      targetStory ? `Marker ${markerId} is not available in story ${targetStory}` : `Marker ${markerId} is not available in the current story`,
    );
  }

  const startCheckpointKey = lookup.checkpointKey ?? null;

  if (!startCheckpointKey) {
    if (!restartedFromHead) {
      await restartJumpStoryFromHead(targetStory, currentStory);
    }
    return true;
  }

  const startCheckpoint = debugState.checkpoints[startCheckpointKey];
  if (!startCheckpoint) {
    throw new Error(`Local checkpoint ${startCheckpointKey} not found`);
  }

  const restored = await executeScenarioCommand<boolean>({
    subCommand: 'restoreCheckpoint',
    key: startCheckpointKey,
  });
  if (!restored) {
    throw new Error(`Failed to restore checkpoint ${startCheckpointKey}`);
  }

  if (appStateAdapter) {
    await appStateAdapter.restore(startCheckpoint.appState);
  }

  debugState.currentCursor = startCheckpoint.cursor;
  debugState.currentStory = startCheckpoint.cursor.story;
  return true;
}

function handleMarkerEnter(marker: MarkerEnter) {
  const checkpoint: CombinedCheckpoint<unknown> = {
    cursor: {
      story: marker.story,
      paragraph: marker.paragraph,
      markerId: marker.markerId,
    },
    appState: captureCheckpointAppStateSync(),
  };

  debugState.currentCursor = checkpoint.cursor;
  debugState.currentStory = checkpoint.cursor.story;
  debugState.checkpoints[marker.markerId] = checkpoint;
  notifyMarkerEnter(checkpoint);

  if (activeFastForward?.targetMarkerId === marker.markerId) {
    void settleActiveFastForward();
  }
}

async function runFastForwardToMarker(markerId: string) {
  if (!appStateAdapter?.enterFastForwardMode || !appStateAdapter?.exitFastForwardMode) {
    throw new Error('Fast-forward mode is not supported by the app state adapter');
  }

  if (activeFastForward !== null || activeWarp !== null) {
    throw new Error('A debug seek operation is already active');
  }

  await appStateAdapter.enterFastForwardMode({
    onAbort: (error) => {
      void settleActiveFastForward(error);
    },
  });

  const completion = new Promise<void>((resolve, reject) => {
    activeFastForward = {
      targetMarkerId: markerId,
      resolve,
      reject,
    };
  });

  try {
    await executeScenarioCommand<void>({ subCommand: 'nextLine' });
  } catch (error) {
    await settleActiveFastForward(error);
  }

  await completion;
}

async function runWarp(options: DebugWarpOptions) {
  if (!appStateAdapter?.enterFastForwardMode || !appStateAdapter?.exitFastForwardMode) {
    throw new Error('Warp mode is not supported by the app state adapter');
  }

  if (activeFastForward !== null || activeWarp !== null) {
    throw new Error('A debug seek operation is already active');
  }

  const boundary = options.boundary ?? 'before';

  await appStateAdapter.enterFastForwardMode({ warp: true });

  const completion = new Promise<void>((resolve, reject) => {
    activeWarp = {
      targetMarkerId: options.markerId,
      resolve,
      reject,
    };
  });

  try {
    await executeScenarioCommand<void>({
      subCommand: 'warp',
      markerId: options.markerId,
      boundary,
    });
  } catch (error) {
    await settleActiveWarp(error);
  }

  await completion;
}

const debugSessionController: DebugSessionController = {
  async restoreCheckpoint(markerId, options) {
    debugState.restoring = true;
    debugState.lastError = null;

    try {
      const needsFastForward = await enqueueDebugOperation(async () => prepareJumpStart(markerId, options));

      if (needsFastForward) {
        await runFastForwardToMarker(markerId);
      }

      return true;
    } catch (error) {
      debugState.lastError = toErrorMessage(error);
      throw error;
    } finally {
      debugState.restoring = false;
    }
  },

  async warp(options) {
    debugState.restoring = true;
    debugState.lastError = null;

    try {
      const needsWarp = await enqueueDebugOperation(async () => prepareJumpStart(options.markerId, options));

      if (needsWarp) {
        await runWarp(options);
      }
    } catch (error) {
      debugState.lastError = toErrorMessage(error);
      throw error;
    } finally {
      debugState.restoring = false;
    }
  },

  async switchRoute(routeName, params) {
    return enqueueDebugOperation(async () => {
      debugState.lastError = null;

      try {
        const navigator = getNavigator();

        if (navigator.hasPage(routeName)) {
          if (!appStateAdapter?.switchPage) {
            throw new Error('Page switching is not supported by the app state adapter');
          }

          await appStateAdapter.switchPage(routeName, params);
          return;
        }

        if (navigator.hasOverlay(routeName)) {
          if (!appStateAdapter?.switchOverlay) {
            throw new Error('Overlay switching is not supported by the app state adapter');
          }

          await appStateAdapter.switchOverlay(routeName, params);
          return;
        }

        throw new Error(`Route "${routeName}" not found`);
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
        throw error;
      }
    });
  },
};

export function registerAppStateAdapter<TState = unknown>(adapter: AppStateAdapter<TState> | null): void {
  appStateAdapter = adapter as AnyAdapter | null;
}

export function getAppStateAdapter<TState = unknown>(): AppStateAdapter<TState> | null {
  return appStateAdapter as AppStateAdapter<TState> | null;
}

export async function startDebugSession(config: DebugSessionConfig = {}): Promise<DebugSessionController> {
  await stopDebugSession();

  currentConfig = {
    onMarkerEnter: config.onMarkerEnter ?? (() => {}),
    onError: config.onError ?? (() => {}),
  };

  clearLocalRuntimeState();
  debugState.active = true;

  disposeMarkerListener = addEventListener('scenariomarkerenter', (event) => {
    const marker = event as MarkerEnter;

    try {
      handleMarkerEnter(marker);
    } catch (error) {
      reportDebugSessionError(error);
    }
  });

  disposeFinishedListener = addEventListener('scenariofinished', () => {
    debugState.currentCursor = null;

    if (activeFastForward) {
      void settleActiveFastForward(
        new Error(`Marker ${activeFastForward.targetMarkerId} was not reached before the scenario finished`),
      );
    }

    if (activeWarp) {
      const targetMarkerId = activeWarp.targetMarkerId;
      void settleActiveWarp(new Error(`Marker ${targetMarkerId} was not reached before the scenario finished`));
    }
  });

  disposeWarpFinishedListener = addEventListener('scenariowarpfinished', () => {
    const warp = activeWarp;
    if (!warp) {
      return;
    }

    void enqueueDebugOperation(async () => {
      try {
        await syncLocalWarpCheckpoint(warp.targetMarkerId);
        await settleActiveWarp();
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
        await settleActiveWarp(error);
        await currentConfig.onError(error);
      }
    });
  });

  try {
    await clearRemoteCheckpoints();
    debugState.currentCursor = await executeScenarioCommand<ExecutionCursor | null>({
      subCommand: 'getExecutionCursor',
    });
    debugState.currentStory = debugState.currentCursor?.story ?? null;
  } catch (error) {
    debugState.active = false;
    clearLocalRuntimeState();
    disposeMarkerListener?.();
    disposeFinishedListener?.();
    disposeWarpFinishedListener?.();
    disposeMarkerListener = null;
    disposeFinishedListener = null;
    disposeWarpFinishedListener = null;
    await currentConfig.onError(error);
    throw error;
  }

  return debugSessionController;
}

export async function stopDebugSession(): Promise<void> {
  disposeMarkerListener?.();
  disposeFinishedListener?.();
  disposeWarpFinishedListener?.();
  disposeMarkerListener = null;
  disposeFinishedListener = null;
  disposeWarpFinishedListener = null;
  await settleActiveFastForward(new Error('Debug session stopped'));
  await settleActiveWarp(new Error('Debug session stopped'));

  if (debugState.active) {
    await enqueueDebugOperation(async () => {
      try {
        await clearRemoteCheckpoints();
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
        await currentConfig.onError(error);
      }
    });
  }

  debugState.active = false;
  clearLocalRuntimeState();
}

export async function clearDebugSessionRuntimeState(): Promise<void> {
  await clearLocalDebugSessionRuntimeState();

  if (!debugState.active) {
    return;
  }

  await enqueueDebugOperation(async () => {
    try {
      await clearRemoteCheckpoints();
    } catch (error) {
      debugState.lastError = toErrorMessage(error);
      await currentConfig.onError(error);
    }
  });
}

export function useDebugSession() {
  const snapshot = useSnapshot(debugState);

  return {
    ...snapshot,
    startDebugSession,
    stopDebugSession,
    restoreCheckpoint: debugSessionController.restoreCheckpoint,
    warp: debugSessionController.warp,
    switchRoute: debugSessionController.switchRoute,
  };
}
