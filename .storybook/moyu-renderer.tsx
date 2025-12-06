/**
 * Moyu Renderer for Storybook
 *
 * This implements a custom Storybook renderer that:
 * 1. Initializes the Moyu engine once
 * 2. Uses a single root instance for all stories
 * 3. Re-renders different stories by calling root.render() with new components
 */

import { createRoot, addEventListener } from '@momoyu-ink/kit';
import type { ReactElement } from 'react';

let rootInstance: ReturnType<typeof createRoot> | null = null;
let isInitialized = false;
let engineReady = false;

// Promise that resolves when engine is ready
let readyPromise: Promise<void> | null = null;

/**
 * Initialize Moyu engine
 * This should be called once before any story rendering
 */
export async function initializeMoyuEngine(): Promise<void> {
  if (isInitialized && readyPromise) {
    return readyPromise;
  }

  isInitialized = true;

  readyPromise = new Promise<void>((resolve) => {
    // Import and initialize the engine
    import(/* @vite-ignore */ '../dist/moyu.js').then(async (moyu) => {
      // Initialize WASM
      await moyu.default();

      // Setup global moyu object
      window.moyu = {
        pushCommand(name: string, args: any[]) {
          return (moyu as any)[name](...args);
        },
        executeNodeCommand(node_id: number, payload: any) {
          return moyu.execute_node_command(node_id, payload);
        },
        executePluginCommand(plugin_name: string, payload: any) {
          return moyu.execute_plugin_command(plugin_name, payload);
        },
      };

      // Initialize the engine with storybook-root container
      moyu.moyu_init('storybook-root', {
        appName: 'gallery',
        initialSurfaceSize: '600x400',
        stageSize: '600x400',
        backgroundColor: '#222325',
        entry: './index.json',
        entryFilename: './main.js',
        autorun: 'all',
        fontFile: 'default.otf',
        enableGamepads: true,
        skipSplash: true,
      });

      console.log('[Moyu Storybook] Engine initialized');

      // Wait for ready event to create root
      addEventListener('ready', () => {
        rootInstance = createRoot();
        engineReady = true;
        console.log('[Moyu Storybook] Root created, ready to render stories');
        resolve();
      });
    });
  });

  return readyPromise;
}

/**
 * Render a story component using the Moyu root
 */
export function renderStory(StoryComponent: ReactElement): void {
  if (!rootInstance || !engineReady) {
    console.error('[Moyu Storybook] Engine not ready yet');
    return;
  }

  console.log('[Moyu Storybook] Rendering story component');

  // Wrap in a container to ensure proper cleanup between stories
  rootInstance.render(StoryComponent);

  console.log('[Moyu Storybook] Story rendered');
}

/**
 * Check if engine is ready
 */
export function isEngineReady(): boolean {
  return engineReady && rootInstance !== null;
}

/**
 * Get the root instance (for debugging)
 */
export function getRootInstance() {
  return rootInstance;
}
