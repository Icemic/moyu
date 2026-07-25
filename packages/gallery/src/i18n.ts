import { i18n } from '@lingui/core';
import { messages } from './locales/zh/messages.po';

export const DEFAULT_LOCALE = 'zh';

export function activateLocale(locale: typeof DEFAULT_LOCALE) {
  i18n.loadAndActivate({ locale, messages });
}

export { i18n };
