import { useSnapshot, proxy } from 'valtio';
import type { ExecutionCursor } from '../bindings/ExecutionCursor';
import type { MarkerEnter } from '../bindings/MarkerEnter';
import type { ScenarioCommand } from '../bindings/ScenarioCommand';
import { addEventListener } from '../events';
import { executePluginCommand } from '../moyu';

export interface AppStateAdapter<TState = unknown> {
  capture(): TState | Promise<TState>;
  restore(state: TState): void | Promise<void>;
}

export type CombinedCheckpoint<TState = unknown> = {
  key: string;
  cursor: ExecutionCursor;
  appState: TState;
};

export interface DebugSessionConfig {
  capturePolicy?: (marker: MarkerEnter) => boolean | Promise<boolean>;
  checkpointKey?: (marker: MarkerEnter) => string;
}

export interface DebugSessionController {
  restoreCheckpoint(key: string): Promise<boolean>;
  dropCheckpoint(key: string): Promise<boolean>;
  clearCheckpoints(): Promise<void>;
}

type AnyAdapter = AppStateAdapter<any>;

type DebugSessionState = {
  active: boolean;
  checkpoints: Record<string, CombinedCheckpoint<unknown>>;
  currentCursor: ExecutionCursor | null;
  restoring: boolean;
  lastError: string | null;
};

const debugState = proxy<DebugSessionState>({
  active: false,
  checkpoints: {},
  currentCursor: null,
  restoring: false,
  lastError: null,
});

let appStateAdapter: AnyAdapter | null = null;
let currentConfig: Required<DebugSessionConfig> = {
  capturePolicy: () => true,
  checkpointKey: (marker) => marker.markerId,
};
let disposeMarkerListener: (() => void) | null = null;
let disposeFinishedListener: (() => void) | null = null;
let debugOperationQueue: Promise<void> = Promise.resolve();

function enqueueDebugOperation<T>(task: () => Promise<T>): Promise<T> {
  const nextTask = debugOperationQueue.then(task, task);
  debugOperationQueue = nextTask.then(
    () => undefined,
    () => undefined,
  );
  return nextTask;
}

function executeScenarioCommand<T>(payload: ScenarioCommand): Promise<T> {
  return Promise.resolve(executePluginCommand('scenario', payload as never) as T);
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function clearLocalRuntimeState() {
  debugState.checkpoints = {};
  debugState.currentCursor = null;
  debugState.restoring = false;
  debugState.lastError = null;
}

async function clearRemoteCheckpoints() {
  await executeScenarioCommand<void>({ subCommand: 'clearCheckpoints' });
}

async function handleMarkerEnter(marker: MarkerEnter) {
  const cursor: ExecutionCursor = {
    story: marker.story,
    paragraph: marker.paragraph,
    markerId: marker.markerId,
  };

  debugState.currentCursor = cursor;

  const shouldCapture = await currentConfig.capturePolicy(marker);
  if (!shouldCapture) {
    return;
  }

  const key = currentConfig.checkpointKey(marker);
  await executeScenarioCommand<boolean>({ subCommand: 'captureCheckpoint', key });

  try {
    const appState = appStateAdapter ? await appStateAdapter.capture() : undefined;
    debugState.checkpoints = {
      ...debugState.checkpoints,
      [key]: {
        key,
        cursor,
        appState,
      },
    };
  } catch (error) {
    await executeScenarioCommand<boolean>({ subCommand: 'dropCheckpoint', key });
    throw error;
  }
}

const debugSessionController: DebugSessionController = {
  async restoreCheckpoint(key) {
    const checkpoint = debugState.checkpoints[key];
    if (!checkpoint) {
      return false;
    }

    return enqueueDebugOperation(async () => {
      debugState.restoring = true;
      debugState.lastError = null;

      try {
        const restored = await executeScenarioCommand<boolean>({
          subCommand: 'restoreCheckpoint',
          key,
        });
        if (!restored) {
          return false;
        }

        if (appStateAdapter) {
          await appStateAdapter.restore(checkpoint.appState);
        }

        debugState.currentCursor = checkpoint.cursor;
        return true;
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
        throw error;
      } finally {
        debugState.restoring = false;
      }
    });
  },

  async dropCheckpoint(key) {
    return enqueueDebugOperation(async () => {
      const removed = await executeScenarioCommand<boolean>({
        subCommand: 'dropCheckpoint',
        key,
      });

      if (removed) {
        const nextCheckpoints = { ...debugState.checkpoints };
        delete nextCheckpoints[key];
        debugState.checkpoints = nextCheckpoints;
      }

      return removed;
    });
  },

  async clearCheckpoints() {
    await enqueueDebugOperation(async () => {
      await clearRemoteCheckpoints();
      debugState.checkpoints = {};
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
    capturePolicy: config.capturePolicy ?? (() => true),
    checkpointKey: config.checkpointKey ?? ((marker) => marker.markerId),
  };

  clearLocalRuntimeState();
  debugState.active = true;

  disposeMarkerListener = addEventListener('scenarioMarkerEnter', (event) => {
    void enqueueDebugOperation(async () => {
      try {
        await handleMarkerEnter(event as MarkerEnter);
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
      }
    });
  });

  disposeFinishedListener = addEventListener('scenarioFinished', () => {
    debugState.currentCursor = null;
  });

  try {
    await clearRemoteCheckpoints();
    debugState.currentCursor = await executeScenarioCommand<ExecutionCursor | null>({
      subCommand: 'getExecutionCursor',
    });
  } catch (error) {
    debugState.active = false;
    clearLocalRuntimeState();
    disposeMarkerListener?.();
    disposeFinishedListener?.();
    disposeMarkerListener = null;
    disposeFinishedListener = null;
    throw error;
  }

  return debugSessionController;
}

export async function stopDebugSession(): Promise<void> {
  disposeMarkerListener?.();
  disposeFinishedListener?.();
  disposeMarkerListener = null;
  disposeFinishedListener = null;

  if (debugState.active) {
    await enqueueDebugOperation(async () => {
      try {
        await clearRemoteCheckpoints();
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
      }
    });
  }

  debugState.active = false;
  clearLocalRuntimeState();
}

export function clearDebugSessionRuntimeState(): void {
  clearLocalRuntimeState();

  if (!debugState.active) {
    return;
  }

  void enqueueDebugOperation(async () => {
    try {
      await clearRemoteCheckpoints();
    } catch (error) {
      debugState.lastError = toErrorMessage(error);
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
    dropCheckpoint: debugSessionController.dropCheckpoint,
    clearCheckpoints: debugSessionController.clearCheckpoints,
  };
}
