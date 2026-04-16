import { createContext, useContext, useEffect, createElement, useCallback } from 'react';
import { proxy, useSnapshot } from 'valtio';
import { nextLine, setWaiting } from './useScenario';
import { ZodType } from 'zod';
import { addEventListener, ResolvedCommandLine, TextLine } from '../events';
import { executePluginCommand } from '../moyu';

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
  /** Record the current runtime snapshot for backlog usage. */
  record(meta: Record<string, any>): string;
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

type AutoBarrierReason = 'hold' | 'wait';

export interface AutoTicketOptions {
  label?: string;
  tailMs?: number;
}

export interface AutoTicketHandle {
  done(): void;
  cancel(): void;
}

interface AutoTicketRecord {
  id: number;
  label?: string;
  tailMs: number;
  doneAt: number | null;
  canceled: boolean;
  barrierId: number | null;
}

interface AutoBarrier {
  id: number;
  reason: AutoBarrierReason;
  collecting: boolean;
  openedAt: number;
  fallbackReadyAt: number;
  tickets: Map<number, AutoTicketRecord>;
  settled: boolean;
}

// ---------------------------------------------------------------------------
// Skip state — independent valtio proxy (not serialized with game saves)
// ---------------------------------------------------------------------------

const SKIP_INTERVAL_MS = 80;

const autoRuntime = {
  defaultTailMs: 0,
};

/** Reactive skip state. Actors read via useIsSkipping(). */
export const skipState = proxy({ active: false });

/** Reactive auto state. Actors read via useIsAutoing(). */
export const autoState = proxy({ active: false });

/** Reactively returns whether skip (Ctrl fast-forward) is active. */
export function useIsSkipping(): boolean {
  return useSnapshot(skipState).active;
}

/** Reactively returns whether auto mode is active. */
export function useIsAutoing(): boolean {
  return useSnapshot(autoState).active;
}

/** Set the default tail delay in milliseconds for newly created auto tickets. */
export function setDefaultAutoTailMs(ms: number): void {
  if (!Number.isFinite(ms)) {
    autoRuntime.defaultTailMs = 0;
    return;
  }

  autoRuntime.defaultTailMs = Math.max(0, ms);
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
  const autoBlockers: Array<() => boolean> = [];
  const beforeHandleCommandCallbacks: Array<(upcomingCommand: ResolvedCommandLine) => void> = [];

  // --- registration functions ---

  /** Push `item` to `registry` and return a function that removes it. */
  function addToRegistry<T>(registry: T[], item: T): () => void {
    registry.push(item);
    return () => {
      const idx = registry.indexOf(item);
      if (idx !== -1) registry.splice(idx, 1);
    };
  }

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
    return addToRegistry(textLineHandlers, handler);
  }

  /**
   * Add a skip callback (called when scenariowaitingcancelled fires).
   * Returns a remove function.
   */
  function addSkipCallback(cb: () => void): () => void {
    return addToRegistry(skipCallbacks, cb);
  }

  /**
   * Add an interrupt callback (called on user click, in order).
   * Return `true` from the callback to consume the click.
   * Returns a remove function.
   */
  function addInterruptCallback(cb: () => boolean): () => void {
    return addToRegistry(interruptCallbacks, cb);
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
    return addToRegistry(skipBlockers, cb);
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

  /**
   * Add an auto blocker callback. Return `true` from the callback to block auto mode.
   * Returns a remove function.
   */
  function addAutoBlocker(cb: () => boolean): () => void {
    return addToRegistry(autoBlockers, cb);
  }

  /** Returns true if any registered auto blocker is active. */
  function isAnyAutoBlockerActive(): boolean {
    for (const cb of autoBlockers) {
      try {
        if (cb()) return true;
      } catch (err) {
        console.error('Error in auto blocker callback:', err);
      }
    }
    return false;
  }

  /** Add a callback to be invoked before handling each command. */
  function addBeforeHandleCommandCallback(cb: (upcomingCommand: ResolvedCommandLine) => void): () => void {
    return addToRegistry(beforeHandleCommandCallbacks, cb);
  }

  // --- skip (Ctrl fast-forward) management ---

  let _skipTimer: ReturnType<typeof setTimeout> | null = null;
  let _autoTimer: ReturnType<typeof setTimeout> | null = null;
  let _autoBarrier: AutoBarrier | null = null;
  let _autoResumeTimer: ReturnType<typeof setTimeout> | null = null;
  let _autoCollectFrame: number | null = null;
  let _autoBarrierSeq = 0;
  let _autoTicketSeq = 0;
  const _pendingAutoTickets = new Map<number, AutoTicketRecord>();

  function clearAutoResumeTimer() {
    if (_autoResumeTimer !== null) {
      clearTimeout(_autoResumeTimer);
      _autoResumeTimer = null;
    }
  }

  function clearAutoCollectFrame() {
    if (_autoCollectFrame !== null) {
      cancelAnimationFrame(_autoCollectFrame);
      _autoCollectFrame = null;
    }
  }

  function clearAutoBarrier() {
    clearAutoCollectFrame();
    clearAutoResumeTimer();
    _autoBarrier = null;
  }

  function clearPendingAutoTickets() {
    _pendingAutoTickets.clear();
  }

  function getActiveAutoBarrier(barrierId: number): AutoBarrier | null {
    if (_autoBarrier === null || _autoBarrier.id !== barrierId) {
      return null;
    }

    return _autoBarrier;
  }

  function scheduleAutoResume(barrierId: number, delayMs: number) {
    clearAutoResumeTimer();

    const safeDelayMs = Number.isFinite(delayMs) ? Math.max(0, delayMs) : 0;

    _autoResumeTimer = setTimeout(() => {
      _autoResumeTimer = null;

      if (!autoState.active) return;
      if (_autoBarrier === null || _autoBarrier.id !== barrierId) return;

      _autoBarrier = null;
      void nextLine();
    }, safeDelayMs);
  }

  function trySettleAutoBarrier(barrier: AutoBarrier) {
    if (barrier.collecting || barrier.settled) return;

    const activeTickets = [...barrier.tickets.values()].filter((ticket) => !ticket.canceled);
    const hasPending = activeTickets.some((ticket) => ticket.doneAt === null);
    if (hasPending) return;

    const latestTicketReadyAt = activeTickets.reduce((maxReadyAt, ticket) => {
      const ticketReadyAt = (ticket.doneAt ?? barrier.openedAt) + ticket.tailMs;
      return Math.max(maxReadyAt, ticketReadyAt);
    }, barrier.fallbackReadyAt);

    barrier.settled = true;
    scheduleAutoResume(barrier.id, latestTicketReadyAt - Date.now());
  }

  function closeAutoCollection(barrierId: number) {
    const barrier = getActiveAutoBarrier(barrierId);
    if (barrier === null || barrier.settled) return;

    barrier.collecting = false;
    trySettleAutoBarrier(barrier);
  }

  function scheduleAutoCollectionClose(barrierId: number) {
    queueMicrotask(() => {
      const barrier = getActiveAutoBarrier(barrierId);
      if (barrier === null || barrier.settled) return;

      _autoCollectFrame = requestAnimationFrame(() => {
        _autoCollectFrame = null;
        closeAutoCollection(barrierId);
      });
    });
  }

  function markAutoTicketDone(ticket: AutoTicketRecord) {
    if (ticket.canceled || ticket.doneAt !== null) return;

    ticket.doneAt = Date.now();

    if (ticket.barrierId === null) return;

    const barrier = getActiveAutoBarrier(ticket.barrierId);
    if (barrier === null || barrier.settled) return;

    trySettleAutoBarrier(barrier);
  }

  function cancelAutoTicket(ticket: AutoTicketRecord) {
    if (ticket.canceled) return;

    ticket.canceled = true;

    if (ticket.barrierId === null) {
      _pendingAutoTickets.delete(ticket.id);
      return;
    }

    const barrier = getActiveAutoBarrier(ticket.barrierId);
    if (barrier === null || barrier.settled) return;

    trySettleAutoBarrier(barrier);
  }

  function createAutoTicketHandle(ticket: AutoTicketRecord): AutoTicketHandle {
    return {
      done() {
        markAutoTicketDone(ticket);
      },
      cancel() {
        cancelAutoTicket(ticket);
      },
    };
  }

  function schedulePendingAutoTicketExpiry(ticketId: number) {
    queueMicrotask(() => {
      requestAnimationFrame(() => {
        _pendingAutoTickets.delete(ticketId);
      });
    });
  }

  function createPendingAutoTicket(options?: AutoTicketOptions): AutoTicketHandle {
    const ticketId = ++_autoTicketSeq;
    const ticket: AutoTicketRecord = {
      id: ticketId,
      label: options?.label,
      tailMs: options?.tailMs ?? autoRuntime.defaultTailMs,
      doneAt: null,
      canceled: false,
      barrierId: null,
    };

    _pendingAutoTickets.set(ticketId, ticket);
    schedulePendingAutoTicketExpiry(ticketId);

    return createAutoTicketHandle(ticket);
  }

  function adoptPendingAutoTickets(barrier: AutoBarrier) {
    for (const [ticketId, ticket] of _pendingAutoTickets) {
      _pendingAutoTickets.delete(ticketId);

      if (ticket.canceled) continue;

      ticket.barrierId = barrier.id;
      barrier.tickets.set(ticketId, ticket);
    }
  }

  function openAutoBarrier(reason: AutoBarrierReason, fallbackMs: number) {
    const safeFallbackMs = Number.isFinite(fallbackMs) ? Math.max(0, fallbackMs) : 0;

    clearAutoBarrier();

    _autoBarrier = {
      id: ++_autoBarrierSeq,
      reason,
      collecting: true,
      openedAt: Date.now(),
      fallbackReadyAt: Date.now() + safeFallbackMs,
      tickets: new Map(),
      settled: false,
    };

    adoptPendingAutoTickets(_autoBarrier);

    scheduleAutoCollectionClose(_autoBarrier.id);
  }

  function issueAutoTicket(options?: AutoTicketOptions): AutoTicketHandle | null {
    if (!autoState.active) {
      return null;
    }

    if (_autoBarrier === null) {
      return createPendingAutoTicket(options);
    }

    if (!_autoBarrier.collecting || _autoBarrier.settled) {
      return null;
    }

    const ticketId = ++_autoTicketSeq;
    const ticket: AutoTicketRecord = {
      id: ticketId,
      label: options?.label,
      tailMs: options?.tailMs ?? autoRuntime.defaultTailMs,
      doneAt: null,
      canceled: false,
      barrierId: _autoBarrier.id,
    };

    _autoBarrier.tickets.set(ticketId, ticket);

    return createAutoTicketHandle(ticket);
  }

  /** Schedule a delayed nextLine() call for the auto. */
  function scheduleAutoNextLine() {
    if (_autoTimer !== null) clearTimeout(_autoTimer);
    _autoTimer = setTimeout(() => {
      _autoTimer = null;
      if (!autoState.active) return;
      if (isAnyAutoBlockerActive()) {
        stopAuto();
        return;
      }
      if (_autoBarrier === null) {
        void nextLine();
      }
    }, autoRuntime.defaultTailMs);
  }

  /** Schedule a delayed nextLine() call for the skip chain. */
  function scheduleSkipNextLine() {
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
    scheduleSkipNextLine(); // kick off the chain
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

  function startAuto() {
    if (autoState.active) return;
    if (isAnyAutoBlockerActive()) return;
    autoState.active = true;
    tryInterrupt(); // finish current printing
    scheduleAutoNextLine(); // kick off the chain
  }

  function stopAuto() {
    autoState.active = false;
    if (_autoTimer !== null) {
      clearTimeout(_autoTimer);
      _autoTimer = null;
    }
    clearAutoBarrier();
    clearPendingAutoTickets();
  }

  function isAutoing(): boolean {
    return autoState.active;
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
          scheduleSkipNextLine();
        } else if (autoState.active) {
          if (isAnyAutoBlockerActive()) {
            stopAuto();
            setWaiting(time, skippable);
          } else {
            openAutoBarrier('wait', time);
          }
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
            scheduleSkipNextLine();
          }
        } else if (skipState.active && unskippableFlag) {
          stopSkip();
        } else if (autoState.active) {
          if (isAnyAutoBlockerActive()) {
            stopAuto();
          } else {
            openAutoBarrier('hold', 0);
          }
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
      record(meta: Record<string, any>) {
        const result = executePluginCommand('scenario', {
          subCommand: 'record',
          meta,
        });

        if (result && typeof (result as PromiseLike<string>).then === 'function') {
          throw new Error('Scenario record command unexpectedly returned a Promise');
        }

        return result as string;
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
    for (const cb of beforeHandleCommandCallbacks) {
      try {
        cb(e);
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
    addAutoBlocker,
    addBeforeHandleCommandCallback,
    issueAutoTicket,
    tryInterrupt,
    startSkip,
    stopSkip,
    isSkipping,
    startAuto,
    stopAuto,
    isAutoing,
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
 * Register an auto blocker callback. Return `true` to block auto mode.
 * Automatically removed on unmount.
 *
 * NOTE: `callback` must be a stable reference (wrap with `useCallback`).
 */
export function useAutoBlocker(callback: () => boolean) {
  const stage = useStageContext();
  useEffect(() => {
    return stage.addAutoBlocker(callback);
  }, [stage, callback]);
}

/**
 * Get a function that issues an auto ticket for the current active barrier.
 * Returns `null` when auto is inactive or the collection window has closed.
 */
export function useAutoTicket() {
  const stage = useStageContext();
  return useCallback(
    (options?: AutoTicketOptions) => {
      return stage.issueAutoTicket(options);
    },
    [stage],
  );
}

/**
 * Register a callback to be invoked before handling each command.
 * The callback receives the upcoming command object, allowing conditional logic.
 * Automatically removed on unmount.
 */
export function useBeforeHandleCommandCallback(callback: (upcomingCommand: ResolvedCommandLine) => void) {
  const stage = useStageContext();
  useEffect(() => {
    return stage.addBeforeHandleCommandCallback(callback);
  }, [stage, callback]);
}
