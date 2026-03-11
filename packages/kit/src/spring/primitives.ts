import type { JSX } from '../declaration';
export type Primitives = keyof JSX.IntrinsicElements;

export const primitives: Primitives[] = [
  'container',
  'sprite',
  'text',
  'clip',
  'filter',
  'backdrop',
  'animation',
  'video',
];
