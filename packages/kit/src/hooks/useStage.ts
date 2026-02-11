import { createContext, useContext, useEffect, createElement } from 'react';
import { proxy, useSnapshot } from 'valtio';
import { nextLine, setWaiting } from './useScenario';
import { ZodType } from 'zod';
import { addEventListener, ResolvedCommandLine, TextLine } from '../events';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Flow control interface passed to every handler. */
export interface GameControl {
  /** Timed wait — blocks auto-advance. Engine fires scenariowaitingcancelled on timeout or skip. */
  setWaiting(time: number, skippable: boolean): void;
  /** Indefinite hold — blocks auto-advance until user action. */
  hold(): void;
  /** Manually advance (rarely needed, e.g. `!` tailing). */
  nextLine(): void;
  /** Mark this dispatch cycle as unskippable. If skipping, hold() will stop skip instead of advancing. */
  unskippable(): void;
}

export interface ScenarioCommandBaseType {
  command: string;
  [key: string]: unknown;
}

/** Command handler function. */
export type CommandHandler<T extends ScenarioCommandBaseType = ScenarioCommandBaseType> = (
  cmd: T,
  control: GameControl,
) => void;

/** Text line handler function. */
export type TextLineHandler = (text: TextLine, control: GameControl) => void;

// ---------------------------------------------------------------------------
// Skip state — independent valtio proxy (not serialized with game saves)
// ---------------------------------------------------------------------------

const SKIP_INTERVAL_MS = 5;

/** Reactive skip state. Actors read via useIsSkipping(). */
export const skipState = proxy({ active: false });

/** Reactively returns whether skip (Ctrl fast-forward) is active. */
export function useIsSkipping(): boolean {
  return useSnapshot(skipState).active;
}

// ---------------------------------------------------------------------------
// createStage — module-level infrastructure
// ---------------------------------------------------------------------------

export function createStage() {
  // --- registries ---
  let commandSchema: ZodType | null = null;
  const commandHandlers = new Map<string, CommandHandler<any>>();
  const textLineHandlers: TextLineHandler[] = [];
  const skipCallbacks: Array<() => void> = [];
  const interruptCallbacks: Array<() => boolean> = [];
  const skipBlockers: Array<() => boolean> = [];
  const beforeHandleCommandCallbacks: Array<() => void> = [];

  // --- registration functions ---

  /**
   * Register command schema.
   * Returns an unregister function.
   */
  function registerCommandSchema<T extends ZodType>(schema: T): () => void {
    if (commandSchema !== null) {
      console.warn('[Stage] Command schema is already registered — overwriting.');
    }
    commandSchema = schema;
    return () => {
      if (commandSchema === schema) {
        commandSchema = null;
      }
    };
  }

  /**
   * Register a command handler for one or more command names.
   * Returns an unregister function.
   */
  function registerCommand<T extends ScenarioCommandBaseType>(
    commands: string | string[],
    handler: CommandHandler<T>,
  ): () => void {
    const names = Array.isArray(commands) ? commands : [commands];
    for (const name of names) {
      if (commandHandlers.has(name)) {
        console.warn(`[Stage] Command handler for "${name}" is already registered — overwriting.`);
      }
      commandHandlers.set(name, handler);
    }
    return () => {
      for (const name of names) {
        if (commandHandlers.get(name) === handler) {
          commandHandlers.delete(name);
        }
      }
    };
  }

  /**
   * Register a text line handler.
   * Returns an unregister function.
   */
  function registerTextLine(handler: TextLineHandler): () => void {
    textLineHandlers.push(handler);
    return () => {
      const idx = textLineHandlers.indexOf(handler);
      if (idx !== -1) textLineHandlers.splice(idx, 1);
    };
  }

  /**
   * Add a skip callback (called when scenariowaitingcancelled fires).
   * Returns a remove function.
   */
  function addSkipCallback(cb: () => void): () => void {
    skipCallbacks.push(cb);
    return () => {
      const idx = skipCallbacks.indexOf(cb);
      if (idx !== -1) skipCallbacks.splice(idx, 1);
    };
  }

  /**
   * Add an interrupt callback (called on user click, in order).
   * Return `true` from the callback to consume the click.
   * Returns a remove function.
   */
  function addInterruptCallback(cb: () => boolean): () => void {
    interruptCallbacks.push(cb);
    return () => {
      const idx = interruptCallbacks.indexOf(cb);
      if (idx !== -1) interruptCallbacks.splice(idx, 1);
    };
  }

  /**
   * Try to interrupt: iterate through interrupt callbacks until one returns true.
   * Returns true if a callback consumed the event.
   */
  function tryInterrupt(): boolean {
    for (const cb of interruptCallbacks) {
      try {
        if (cb()) return true;
      } catch (err) {
        console.error('Error in interrupt callback:', err);
      }
    }
    return false;
  }

  // --- skip blocker ---

  /**
   * Add a skip blocker callback. Return `true` from the callback to block skipping.
   * Returns a remove function.
   */
  function addSkipBlocker(cb: () => boolean): () => void {
    skipBlockers.push(cb);
    return () => {
      const idx = skipBlockers.indexOf(cb);
      if (idx !== -1) skipBlockers.splice(idx, 1);
    };
  }

  /** Returns true if any registered skip blocker is active. */
  function isAnyBlockerActive(): boolean {
    for (const cb of skipBlockers) {
      try {
        if (cb()) return true;
      } catch (err) {
        console.error('Error in skip blocker callback:', err);
      }
    }
    return false;
  }

  /** Add a callback to be invoked before handling each command. */
  function addBeforeHandleCommandCallback(cb: () => void): () => void {
    beforeHandleCommandCallbacks.push(cb);
    return () => {
      const idx = beforeHandleCommandCallbacks.indexOf(cb);
      if (idx !== -1) beforeHandleCommandCallbacks.splice(idx, 1);
    };
  }

  // --- skip (Ctrl fast-forward) management ---

  let _skipTimer: ReturnType<typeof setTimeout> | null = null;

  /** Schedule a delayed nextLine() call for the skip chain. */
  function scheduleNextLine() {
    if (_skipTimer !== null) clearTimeout(_skipTimer);
    _skipTimer = setTimeout(() => {
      _skipTimer = null;
      if (skipState.active) nextLine();
    }, SKIP_INTERVAL_MS);
  }

  /** Enter skip mode. Called when Ctrl is pressed. */
  function startSkip() {
    if (skipState.active) return;
    if (isAnyBlockerActive()) return;
    skipState.active = true;
    invokeSkipCallbacks(); // finish current animations
    tryInterrupt(); // finish current printing
    scheduleNextLine(); // kick off the chain
  }

  /** Exit skip mode. Called when Ctrl is released, window blurs, or Stage unmounts. */
  function stopSkip() {
    skipState.active = false;
    if (_skipTimer !== null) {
      clearTimeout(_skipTimer);
      _skipTimer = null;
      // ensure engine continues normally
      nextLine();
    }
  }

  function isSkipping(): boolean {
    return skipState.active;
  }

  // --- internal helpers ---

  /** Build a one-shot GameControl for a single dispatch cycle. */
  function buildControl(): { control: GameControl; wasHandled: () => boolean } {
    let handled = false;
    let unskippableFlag = false;

    const control: GameControl = {
      setWaiting(time: number, skippable: boolean) {
        handled = true;
        if (skipState.active) {
          // Skip mode: finish residual animations and continue the chain
          invokeSkipCallbacks();
          scheduleNextLine();
        } else {
          setWaiting(time, skippable);
        }
      },
      hold() {
        handled = true;
        if (skipState.active && !unskippableFlag) {
          if (isAnyBlockerActive()) {
            stopSkip();
          } else {
            scheduleNextLine();
          }
        } else if (skipState.active && unskippableFlag) {
          stopSkip();
        }
        // When not skipping: do nothing (normal hold behavior)
      },
      nextLine() {
        handled = true;
        void nextLine();
      },
      unskippable() {
        unskippableFlag = true;
      },
    };
    return { control, wasHandled: () => handled };
  }

  // Transform actual command object to a more convenient format
  function transformCommand(command: ResolvedCommandLine) {
    if (commandSchema === null) {
      throw new Error('No command schema registered in Stage');
    }

    const parsed = commandSchema.safeParse({
      command: command.command,
      ...Object.fromEntries(command.arguments.map((arg) => [arg.name, arg.value])),
    });

    if (!parsed.success) {
      throw new Error(`Invalid command: ${command.command} with arguments ${JSON.stringify(command.arguments)}`);
    }

    return parsed.data as ScenarioCommandBaseType;
  }

  /** Dispatch a scenariocommandline event. */
  function dispatchCommand(e: ResolvedCommandLine) {
    // execute beforeHandleCommand callbacks
    for (const cb of beforeHandleCommandCallbacks) {
      try {
        cb();
      } catch (err) {
        console.error('Error in beforeHandleCommand callback:', err);
      }
    }

    let cmd: ScenarioCommandBaseType;
    try {
      cmd = transformCommand(e);
    } catch (err) {
      console.error('Failed to transform scenario command:', err);
      void nextLine();
      return;
    }

    const handler = commandHandlers.get(cmd.command);
    if (!handler) {
      console.warn(`[Stage] No handler registered for command "${cmd.command}". Auto-advancing.`);
      void nextLine();
      return;
    }

    const { control, wasHandled } = buildControl();
    handler(cmd, control);

    // Default: auto-advance if handler didn't explicitly control flow
    if (!wasHandled()) {
      void nextLine();
    }
  }

  /** Dispatch a scenariotext event. */
  function dispatchTextLine(e: TextLine) {
    const { control, wasHandled } = buildControl();
    for (const handler of textLineHandlers) {
      handler(e, control);
    }
    if (!wasHandled()) {
      void nextLine();
    }
  }

  /** Invoke all skip callbacks. */
  function invokeSkipCallbacks() {
    for (const cb of skipCallbacks) {
      try {
        cb();
      } catch (err) {
        console.error('Error in skip callback:', err);
      }
    }
  }

  // --- bind engine events (called inside StageContextProvider's useEffect) ---

  function bindEvents(): () => void {
    const cleanups = [
      addEventListener('scenariocommandline', (e: ResolvedCommandLine) => {
        dispatchCommand(e);
      }),
      addEventListener('scenariotext', (e: TextLine) => {
        dispatchTextLine(e);
      }),
      addEventListener('scenariowaiting', () => {
        console.log('still waiting...');
      }),
      addEventListener('scenariowaitingcancelled', () => {
        invokeSkipCallbacks();
      }),
    ];
    return () => {
      for (const cleanup of cleanups) cleanup();
    };
  }

  return {
    registerCommandSchema,
    registerCommand,
    registerTextLine,
    addSkipCallback,
    addInterruptCallback,
    addSkipBlocker,
    addBeforeHandleCommandCallback,
    tryInterrupt,
    startSkip,
    stopSkip,
    isSkipping,
    bindEvents,
  };
}

// ---------------------------------------------------------------------------
// Singleton stage instance & React Context
// ---------------------------------------------------------------------------

export type StageInstance = ReturnType<typeof createStage>;

const StageContext = createContext<StageInstance | null>(null);

/** Access the stage instance from within StageContextProvider. */
export function useStageContext(): StageInstance {
  const ctx = useContext(StageContext);
  if (!ctx) throw new Error('useStageContext must be used within StageContextProvider');
  return ctx;
}

// ---------------------------------------------------------------------------
// StageContextProvider — binds engine events within React lifecycle
// ---------------------------------------------------------------------------

export function StageContextProvider({ stage, children }: { stage: StageInstance; children: React.ReactNode }) {
  // Bind engine events on mount, unbind on unmount
  useEffect(() => {
    return stage.bindEvents();
  }, [stage]);

  return createElement(StageContext.Provider, { value: stage }, children);
}

// ---------------------------------------------------------------------------
// Actor hooks — register skip / interrupt callbacks
// ---------------------------------------------------------------------------

/**
 * Register a skip callback that fires when scenariowaitingcancelled is received.
 * Automatically removed on unmount.
 */
export function useSkipCallback(callback: () => void) {
  const stage = useStageContext();
  useEffect(() => {
    return stage.addSkipCallback(callback);
  }, [stage, callback]);
}

/**
 * Register an interrupt callback for user click handling.
 * Return `true` from the callback to consume the click.
 * Automatically removed on unmount.
 */
export function useInterruptCallback(callback: () => boolean) {
  const stage = useStageContext();
  useEffect(() => {
    return stage.addInterruptCallback(callback);
  }, [stage, callback]);
}

/**
 * Register a skip blocker callback. Return `true` to block skipping.
 * When any blocker is active, skip mode cannot start or will be stopped.
 * Automatically removed on unmount.
 *
 * NOTE: `callback` must be a stable reference (wrap with `useCallback`).
 */
export function useSkipBlocker(callback: () => boolean) {
  const stage = useStageContext();
  useEffect(() => {
    return stage.addSkipBlocker(callback);
  }, [stage, callback]);
}

/**
 * Register a callback to be invoked before handling each command.
 * Automatically removed on unmount.
 */
export function useBeforeHandleCommandCallback(callback: () => void) {
  const stage = useStageContext();
  useEffect(() => {
    return stage.addBeforeHandleCommandCallback(callback);
  }, [stage, callback]);
}
