import { addEventListener, createRoot, executePluginCommand } from '@momoyu-ink/kit';
import { Gallery } from './gallery';
import { useEffect } from 'react';

function Main() {
  useEffect(() => {
    return addEventListener('beforeunload', () => {
      executePluginCommand('system', {
        subCommand: 'quit',
      });
    });
  }, []);

  return <Gallery />;
}

addEventListener('ready', () => {
  try {
    console.log('Rendering Moyu Gallery...');
    createRoot().render(<Main />);
  } catch (error) {
    console.error('Failed to render Moyu Gallery:', error);
  }
});
