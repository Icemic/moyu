import { useSnapshot, proxy } from 'valtio';
import type { ExecutionCursor } from '../bindings/ExecutionCursor';
import type { MarkerEnter } from '../bindings/MarkerEnter';
import type { ScenarioCommand } from '../bindings/ScenarioCommand';
import { addEventListener } from '../events';
import { executePluginCommand } from '../moyu';

export interface AppStateAdapter<TState = unknown> {
  capture(): TState | Promise<TState>;
  restore(state: TState): void | Promise<void>;
  switchPage?(page: string, params?: Record<string, unknown>): void | Promise<void>;
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
  restoreCheckpoint(markerId: string): Promise<boolean>;
  switchPage(page: string, params?: Record<string, unknown>): Promise<void>;
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
  onMarkerEnter: () => {},
  onError: () => {},
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
  const checkpoint: CombinedCheckpoint<unknown> = {
    cursor: {
      story: marker.story,
      paragraph: marker.paragraph,
      markerId: marker.markerId,
    },
    appState: appStateAdapter ? await appStateAdapter.capture() : undefined,
  };

  debugState.currentCursor = checkpoint.cursor;
  debugState.checkpoints = {
    ...debugState.checkpoints,
    [marker.markerId]: checkpoint,
  };
  await currentConfig.onMarkerEnter(checkpoint);
}

const debugSessionController: DebugSessionController = {
  async restoreCheckpoint(markerId) {
    return enqueueDebugOperation(async () => {
      debugState.restoring = true;
      debugState.lastError = null;

      try {
        const checkpoint = debugState.checkpoints[markerId];
        if (!checkpoint) {
          return false;
        }

        const restored = await executeScenarioCommand<boolean>({
          subCommand: 'restoreCheckpoint',
          key: markerId,
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

  async switchPage(page, params) {
    return enqueueDebugOperation(async () => {
      debugState.lastError = null;

      try {
        if (!appStateAdapter?.switchPage) {
          throw new Error('Page switching is not supported by the app state adapter');
        }

        await appStateAdapter.switchPage(page, params);
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
    void enqueueDebugOperation(async () => {
      try {
        await handleMarkerEnter(event as MarkerEnter);
      } catch (error) {
        debugState.lastError = toErrorMessage(error);
        await currentConfig.onError(error);
      }
    });
  });

  disposeFinishedListener = addEventListener('scenariofinished', () => {
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
    await currentConfig.onError(error);
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
        await currentConfig.onError(error);
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
    switchPage: debugSessionController.switchPage,
  };
}
