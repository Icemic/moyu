import { executePluginCommand } from '../moyu';
import { useEffect } from 'react';

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
 *
 * @param {string[]} stories - An array of story names to load.
 * @param {string} [startName] - The name of the story to start.
 * @param {string} [entryName] - The entry point within the story to start from.
 * @param {boolean} [goNextOnLoad=false] - Whether to automatically advance to the next line after loading.
 */
export function useScenario(stories: string[], startName?: string, entryName?: string, goNextOnLoad = false) {
  useEffect(() => {
    const loadScenario = async () => {
      for (const story of stories) {
        if (!story) continue;
        try {
          await executePluginCommand('scenario', {
            subCommand: 'addStory',
            name: story,
          });
        } catch (error) {
          console.error(`Failed to load scenario:`, error);
        }
      }
    };

    void loadScenario().then(async () => {
      try {
        if (startName) {
          await executePluginCommand('scenario', {
            subCommand: 'startStory',
            name: startName,
            entry: entryName,
          });
        }

        if (goNextOnLoad) {
          await nextLine();
        }
      } catch (error) {
        console.error(`Failed to start scenario ${startName}:`, error);
      }
    });

    return () => {
      executePluginCommand('scenario', {
        subCommand: 'terminateStory',
      });
    };
  }, [stories, startName, entryName, goNextOnLoad]);
}
