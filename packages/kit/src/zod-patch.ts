declare module 'zod' {
  interface GlobalMeta {
    title?: string;
    format?: string;
    'x-asset-kind'?: 'audio' | 'image' | 'font' | 'video' | 'other';
    'x-i18n'?: Record<string, string>;
    'x-i18n-desc'?: Record<string, string>;
  }
}
