import type { JSX } from 'react';
export type Primitives = keyof JSX.IntrinsicElements;

export const primitives: Primitives[] = ['container', 'sprite', 'yuvsprite', 'video', 'text'];
