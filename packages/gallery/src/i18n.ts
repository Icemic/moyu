import { i18n } from '@lingui/core';
import { messages as enMessages } from './locales/en/messages.po';
import { messages as jaMessages } from './locales/ja/messages.po';
import { messages as zhMessages } from './locales/zh/messages.po';

export const DEFAULT_LOCALE = 'zh';
export type Locale = 'zh' | 'en' | 'ja';

const catalogs = {
  zh: zhMessages,
  en: enMessages,
  ja: jaMessages,
} satisfies Record<Locale, typeof zhMessages>;

export function activateLocale(locale: Locale) {
  i18n.loadAndActivate({ locale, messages: catalogs[locale] });
}

export { i18n };
