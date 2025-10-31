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
