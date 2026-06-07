import { createContext, createElement, useContext, type ReactNode } from 'react';
import { proxy, snapshot, useSnapshot } from 'valtio';

// A typed alias for a Valtio proxy store owned by a concrete app state shape.
export type GameStateStore<TState extends object> = TState;

// Props for a scoped state provider created by the generic runtime.
export interface GameStateSourceProps<TState extends object> {
  store: GameStateStore<TState>;
  children: ReactNode;
}

function cloneStateValue<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

// Recursively mutates an existing proxy subtree to match a plain source value.
function syncStateValue(target: unknown, source: unknown): unknown {
  if (Array.isArray(source)) {
    if (!Array.isArray(target)) {
      return cloneStateValue(source);
    }

    target.length = source.length;
    for (let index = 0; index < source.length; index += 1) {
      target[index] = syncStateValue(target[index], source[index]);
    }
    return target;
  }

  if (isPlainObject(source)) {
    if (!isPlainObject(target)) {
      return cloneStateValue(source);
    }

    for (const key of Object.keys(target)) {
      if (!Object.hasOwn(source, key)) {
        delete target[key];
      }
    }

    for (const [key, nextValue] of Object.entries(source)) {
      target[key] = syncStateValue(target[key], nextValue);
    }

    return target;
  }

  return source;
}

// Creates a generic runtime that knows how to clone, snapshot, sync, and provide a game state of a specific shape.
export function createGameStateContext<TState extends object>(defaults: TState) {
  const defaultState = cloneStateValue(defaults);
  const gameState = proxy<TState>(cloneStateValue(defaults));

  function snapshotGameState(): TState {
    return snapshot(gameState) as TState;
  }

  function syncGameState(source: TState): void {
    const target = gameState as unknown as Record<string, unknown>;

    for (const key of Object.keys(source)) {
      target[key] = syncStateValue(target[key], source[key as keyof TState]);
    }
  }

  function resetGameState(): void {
    syncGameState(defaultState);
  }

  const StoreContext = createContext<GameStateStore<TState>>(gameState);

  function GameStateProvider({ store, children }: GameStateSourceProps<TState>) {
    return createElement(StoreContext.Provider, { value: store }, children);
  }

  function useGameStateStore(): GameStateStore<TState> {
    return useContext(StoreContext);
  }

  function useGameStateSection<K extends keyof TState>(key: K): TState[K] {
    const store = useGameStateStore();
    return (useSnapshot(store) as TState)[key];
  }

  return {
    gameState,
    snapshotGameState,
    syncGameState,
    resetGameState,
    GameStateProvider,
    useGameStateStore,
    useGameStateSection,
  };
}
