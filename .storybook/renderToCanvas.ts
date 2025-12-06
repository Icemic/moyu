/**
 * Custom Storybook renderer configuration for Moyu
 * This file defines how Storybook should render stories using Moyu instead of react-dom
 */

import type { ProjectAnnotations, Renderer } from 'storybook/internal/types';
import { renderStory, isEngineReady, initializeMoyuEngine } from './moyu-renderer';

// Define our custom renderer type
export interface MoyuRenderer extends Renderer {
  component: any;
  storyResult: any;
}

// Global flag to track initialization
let engineInitialized = false;

// Custom render function that uses Moyu instead of react-dom
export const renderToCanvas: ProjectAnnotations<MoyuRenderer>['renderToCanvas'] = async (
  { storyContext, unboundStoryFn, showMain, showError, showException },
  _canvasElement,
) => {
  console.log('[Moyu Renderer] renderToCanvas called for:', storyContext.id);

  try {
    showMain();

    if (!engineInitialized) {
      console.log('[Moyu Storybook] Initializing engine...');
      await initializeMoyuEngine();
      engineInitialized = true;
    }

    // Ensure engine is initialized
    if (!isEngineReady()) {
      console.log('[Moyu Renderer] Waiting for engine to be ready...');
      // Wait a bit and retry
      await new Promise((resolve) => setTimeout(resolve, 100));
      if (!isEngineReady()) {
        showError({
          title: 'Moyu Engine Not Ready',
          description: 'The Moyu engine is not ready to render stories.',
        });
        return;
      }
    }

    // Get the story element
    const storyElement = unboundStoryFn(storyContext);

    console.log('[Moyu Renderer] Rendering story to Moyu canvas');

    // Render to Moyu
    renderStory(storyElement);

    // Return cleanup function
    return () => {
      console.log('[Moyu Renderer] Cleanup for:', storyContext.id);
    };
  } catch (error) {
    console.error('[Moyu Renderer] Error rendering story:', error);
    showException(error as Error);
  }
};
