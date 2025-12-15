import { executePluginCommand } from './moyu';

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

export type MakeNullOptional<T> = T extends any
  ? {
      [K in keyof T as null extends T[K] ? never : K]: T[K];
    } & {
      [K in keyof T as null extends T[K] ? K : never]?: T[K];
    }
  : never;
