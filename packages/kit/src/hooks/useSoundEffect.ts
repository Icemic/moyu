import { executePluginCommand } from '../moyu';
import { useCallback, useEffect } from 'react';

/**
 * Used to load a sound effect and provide a function to play it.
 */
export function useSoundEffect(src?: string) {
  useEffect(() => {
    // Cleanup function to unload the sound effect when the component unmounts
    return () => {
      try {
        if (src) {
          executePluginCommand('audio', {
            subCommand: 'release',
            name: src,
            silentFail: true,
          });
        }
      } catch (error) {
        console.error(`Failed to release sound effect ${src}:`, error);
      }
    };
  }, [src]);

  // Function to play the sound effect
  return useCallback(() => {
    try {
      if (src) {
        executePluginCommand('audio', {
          subCommand: 'loadAndPlay',
          name: src,
          src: src,
        });
      }
    } catch (error) {
      console.error(`Failed to play sound effect ${src}:`, error);
    }
  }, [src]);
}
