import { executePluginCommand } from './moyu';
import type { BubbleEvent } from './events/base';
import type { MoyuEventHandler } from './declaration';

export function getStageSize(): { width: number; height: number; scaleFactor: number } {
  const stageSize = executePluginCommand('system', {
    subCommand: 'getStageSize',
  }) as {
    width: number;
    height: number;
    scaleFactor: number;
  };
  return stageSize;
}

export type MakeNullOptional<T> = T extends unknown
  ? {
      [K in keyof T as null extends T[K] ? never : K]: T[K];
    } & {
      [K in keyof T as null extends T[K] ? K : never]?: T[K];
    }
  : never;

export function mergeEvent<T extends BubbleEvent, K extends BubbleEvent>(
  handlers: MoyuEventHandler<T> | readonly MoyuEventHandler<T>[] | undefined,
  defaultHandler?: MoyuEventHandler<K>,
) {
  return (event: T | K) => {
    if (handlers) {
      const handlerList = Array.isArray(handlers) ? handlers : [handlers];
      for (const handler of handlerList) {
        handler(event as T);
      }
    }

    if (!event.defaultPrevented) {
      defaultHandler?.(event as K);
    }
  };
}
