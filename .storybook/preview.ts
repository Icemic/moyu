import type { Preview } from '@storybook/react';
import { renderToCanvas } from './renderToCanvas';

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    // Disable docs mode for now since it requires simultaneous rendering
    docs: {
      disable: false,
      codePanel: true,
    },
  },
};

// Export custom renderToCanvas to override default react-dom rendering
export { renderToCanvas };

export default preview;
