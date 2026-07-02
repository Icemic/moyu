declare module 'zod' {
  interface GlobalMeta {
    title?: string;
    format?: string;
    'x-asset-kind'?: 'audio' | 'image' | 'font' | 'video' | 'other';
    /// The step value for number inputs, used in UI components to determine the increment/decrement step size.
    /// It is different from the `multipleOf` constraint, which is a validation rule.
    'x-step'?: number;
    'x-i18n'?: Record<string, string>;
    'x-i18n-desc'?: Record<string, string>;
  }
}
