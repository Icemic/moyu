import { addEventListener, createRoot, executePluginCommand } from '@momoyu-ink/kit';
import { I18nProvider } from '@lingui/react';
import { Gallery } from './gallery';
import { activateLocale, DEFAULT_LOCALE, i18n } from './i18n';
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
    activateLocale(DEFAULT_LOCALE);
    createRoot().render(
      <I18nProvider i18n={i18n}>
        <Main />
      </I18nProvider>,
    );
  } catch (error) {
    console.error('Failed to render Moyu Gallery:', error);
  }
});
