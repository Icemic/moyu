import { executePluginCommand } from '../moyu';
import { useEffect } from 'react';

interface ScenarioSessionConfig {
  stories: string[];
  startName?: string;
  entryName?: string;
  goNextOnLoad: boolean;
}

interface ScenarioSession {
  id: number;
  key: string;
  refCount: number;
  started: boolean;
  startPromise: Promise<void> | null;
  disposeTimer: ReturnType<typeof setTimeout> | null;
  disposed: boolean;
}

let currentScenarioSession: ScenarioSession | null = null;
let scenarioSessionId = 0;
let scenarioLifecycleQueue: Promise<void> = Promise.resolve();

function normalizeStories(stories: string[]): string[] {
  return stories.filter((story): story is string => Boolean(story));
}

function createScenarioSessionKey(config: ScenarioSessionConfig): string {
  return JSON.stringify({
    stories: config.stories,
    startName: config.startName ?? null,
    entryName: config.entryName ?? null,
    goNextOnLoad: config.goNextOnLoad,
  });
}

function isCurrentScenarioSession(session: ScenarioSession): boolean {
  return currentScenarioSession?.id === session.id && !session.disposed;
}

function enqueueScenarioLifecycleTask(task: () => Promise<void>): Promise<void> {
  const nextTask = scenarioLifecycleQueue.then(task, task);
  scenarioLifecycleQueue = nextTask.catch(() => {});
  return nextTask;
}

function clearScenarioDisposeTimer(session: ScenarioSession) {
  if (session.disposeTimer !== null) {
    clearTimeout(session.disposeTimer);
    session.disposeTimer = null;
  }
}

async function terminateCurrentScenario() {
  try {
    await executePluginCommand('scenario', {
      subCommand: 'terminateStory',
    });
  } catch (error) {
    console.error('Failed to terminate scenario:', error);
  }
}

async function startScenarioSession(session: ScenarioSession, config: ScenarioSessionConfig) {
  for (const story of config.stories) {
    if (!isCurrentScenarioSession(session)) return;

    try {
      await executePluginCommand('scenario', {
        subCommand: 'addStory',
        name: story,
      });
    } catch (error) {
      console.error('Failed to load scenario:', error);
    }
  }

  if (!isCurrentScenarioSession(session)) return;

  try {
    if (config.startName) {
      await executePluginCommand('scenario', {
        subCommand: 'startStory',
        name: config.startName,
        entry: config.entryName,
      });
    }

    if (!isCurrentScenarioSession(session)) return;

    if (config.goNextOnLoad) {
      await nextLine();
    }
  } catch (error) {
    console.error(`Failed to start scenario ${config.startName}:`, error);
  }
}

function ensureScenarioSessionStarted(session: ScenarioSession, config: ScenarioSessionConfig) {
  if (session.started || session.startPromise !== null) {
    return;
  }

  const startPromise = enqueueScenarioLifecycleTask(async () => {
    if (!isCurrentScenarioSession(session)) return;

    await startScenarioSession(session, config);

    if (isCurrentScenarioSession(session)) {
      session.started = true;
    }
  });

  session.startPromise = startPromise;
  void startPromise.finally(() => {
    if (session.startPromise === startPromise) {
      session.startPromise = null;
    }
  });
}

function disposeScenarioSession(session: ScenarioSession, delayMs: number) {
  clearScenarioDisposeTimer(session);

  session.disposeTimer = setTimeout(() => {
    if (!isCurrentScenarioSession(session) || session.refCount > 0) {
      return;
    }

    session.disposeTimer = null;
    session.disposed = true;
    currentScenarioSession = null;

    void enqueueScenarioLifecycleTask(terminateCurrentScenario);
  }, delayMs);
}

function acquireScenarioSession(config: ScenarioSessionConfig): ScenarioSession {
  const key = createScenarioSessionKey(config);

  if (currentScenarioSession !== null && currentScenarioSession.key !== key) {
    const previousSession = currentScenarioSession;
    clearScenarioDisposeTimer(previousSession);
    previousSession.disposed = true;
    currentScenarioSession = null;

    void enqueueScenarioLifecycleTask(terminateCurrentScenario);
  }

  if (currentScenarioSession === null) {
    currentScenarioSession = {
      id: ++scenarioSessionId,
      key,
      refCount: 0,
      started: false,
      startPromise: null,
      disposeTimer: null,
      disposed: false,
    };
  }

  clearScenarioDisposeTimer(currentScenarioSession);
  currentScenarioSession.refCount += 1;
  ensureScenarioSessionStarted(currentScenarioSession, config);

  return currentScenarioSession;
}

function releaseScenarioSession(session: ScenarioSession) {
  if (!isCurrentScenarioSession(session)) {
    return;
  }

  session.refCount = Math.max(0, session.refCount - 1);

  if (session.refCount > 0) {
    return;
  }

  // Delay teardown so transient remounts such as Fast Refresh can reuse the live session.
  disposeScenarioSession(session, 0);
}

/** Advance to the next line in the current story. */
export function nextLine() {
  return executePluginCommand('scenario', { subCommand: 'nextLine' });
}

/** Set a timed wait, optionally skippable. */
export function setWaiting(time: number, skippable: boolean) {
  executePluginCommand('scenario', { subCommand: 'setWaiting', time, skippable });
}

/**
 * Custom hook to manage scenario lifecycle (load, start, terminate).
 * The underlying session is resilient to transient remounts such as Fast Refresh.
 *
 * @param {string[]} stories - An array of story names to load.
 * @param {string} [startName] - The name of the story to start.
 * @param {string} [entryName] - The entry point within the story to start from.
 * @param {boolean} [goNextOnLoad=false] - Whether to automatically advance to the next line after loading.
 */
export function useScenario(stories: string[], startName?: string, entryName?: string, goNextOnLoad = false) {
  useEffect(() => {
    const session = acquireScenarioSession({
      stories: normalizeStories(stories),
      startName,
      entryName,
      goNextOnLoad,
    });

    return () => {
      releaseScenarioSession(session);
    };
  }, [stories, startName, entryName, goNextOnLoad]);
}
