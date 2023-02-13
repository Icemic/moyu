import { ReactNode } from 'react';

export type DetailedHaiProps<E extends HaiNodeAttributes> = E;

export interface HaiNodeAttributes {
  label?: string;
  x?: number;
  y?: number;
  // the anchor for node, values range from 0.0 to 1.0.
  anchor?: [number, number];
  // the pivot on ratation, values in pixels.
  pivot?: [number, number];
  scale?: number;
  scaleX?: number;
  scaley?: number;
  rotation?: number;
  skew?: number;
  skewX?: number;
  skewy?: number;
  children?: ReactNode;
}

export type HaiContainerAttributes = HaiNodeAttributes;
export interface HaiSpriteAttribute extends HaiNodeAttributes {
  src: string;
  area?: [number, number, number, number];
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      container: DetailedHaiProps<HaiContainerAttributes>;
      sprite: DetailedHaiProps<HaiSpriteAttribute>;
    }
  }
}
