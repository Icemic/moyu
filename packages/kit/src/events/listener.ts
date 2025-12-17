import { globalEventListeners } from './globals';

export function addEventListener(name: string, callback: (...args: any[]) => void): () => void {
  if (!globalEventListeners[name]) {
    globalEventListeners[name] = [];
  }

  globalEventListeners[name].push(callback);

  return () => {
    globalEventListeners[name] = globalEventListeners[name].filter((cb) => cb !== callback);
  };
}
