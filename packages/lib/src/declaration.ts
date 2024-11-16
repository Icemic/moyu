import type * as PropTypes from 'prop-types';
import type { ClassAttributes, ReactNode } from 'react';
import type { HaiEvent } from './hai';
import type { Node } from './node';

export type DetailedHaiProps<E extends HaiNodeAttributes> = ClassAttributes<Node> & E;

// interface DOMElement<P extends HTMLAttributes<T> | SVGAttributes<T>, T extends Element>
//   extends ReactElement<P, string> {
//   ref: LegacyRef<T>;
// }

export type HaiEventHandler = (event: HaiEvent) => void;

export interface HaiListenerAttributes {
  onClick?: HaiEventHandler;
  onMouseEnter?: HaiEventHandler;
  onMouseLeave?: HaiEventHandler;
  onMouseDown?: HaiEventHandler;
  onMouseUp?: HaiEventHandler;
  onMouseMove?: HaiEventHandler;
  onKeyDown?: HaiEventHandler;
  onKeyUp?: HaiEventHandler;
  onKeyPress?: HaiEventHandler;
  onTouchStart?: HaiEventHandler;
  onTouchMove?: HaiEventHandler;
  onTouchEnd?: HaiEventHandler;
  onTouchCancel?: HaiEventHandler;
}

export interface HaiNodeAttributes extends HaiListenerAttributes {
  label?: string;
  x?: number;
  y?: number;
  // the anchor for node, values range from 0.0 to 1.0.
  anchor?: [number, number];
  // the pivot on ratation, values in pixels.
  pivot?: [number, number];
  scale?: number;
  scaleX?: number;
  scaleY?: number;
  rotation?: number;
  skew?: number;
  skewX?: number;
  skewY?: number;
  visible?: boolean;
  tint?: string;
  opacity?: number;
  children?: ReactNode;
  interactive?: boolean;
  cursor?: Cursor;
}

export type HaiContainerAttributes = HaiNodeAttributes;
export interface HaiSpriteAttribute extends HaiNodeAttributes {
  src?: string;
  area?: [number, number, number, number];
  mode?: 'normal' | 'nineslice';
  bounds?: [number, number, number, number];
  nineSliceMode?: 'stretch' | 'repeat' | 'mirror' | 'blank';
  targetWidth?: number;
  targetHeight?: number;
}

export interface HaiYUVSpriteAttribute extends HaiNodeAttributes {
  area?: [number, number, number, number];
}

export interface HaiVideoAttribute extends HaiSpriteAttribute {
  src: string;
  area?: [number, number, number, number];
  autoplay?: boolean;
}

export type Color = number | string;

export type Cursor =
  | 'hidden'
  | 'default'
  | 'context-menu'
  | 'help'
  | 'pointer'
  | 'progress'
  | 'wait'
  | 'cell'
  | 'crosshair'
  | 'text'
  | 'vertical-text'
  | 'alias'
  | 'copy'
  | 'move'
  | 'no-drop'
  | 'not-allowed'
  | 'grab'
  | 'grabbing'
  | 'e-resize'
  | 'n-resize'
  | 'ne-resize'
  | 'nw-resize'
  | 's-resize'
  | 'se-resize'
  | 'sw-resize'
  | 'w-resize'
  | 'ew-resize'
  | 'ns-resize'
  | 'nesw-resize'
  | 'nwse-resize'
  | 'col-resize'
  | 'row-resize'
  | 'all-scroll'
  | 'zoom-in'
  | 'zoom-out';

export interface HaiTextAttribute extends HaiNodeAttributes {
  text?: string;
  printMode?: 'instant' | 'typewriter' | 'printer';
  printSpeed?: number;

  /* layout styles */
  /// the writing direction of the text in the box,
  /// only `Horizontal` (right-to-left) or `Vertical` (top-to-bottom) is valid.
  direction?: 'horizontal' | 'vertical';
  /// the width of box.
  boxWidth?: number;
  /// the height of box.
  boxHeight?: number;
  /// the size of the glyph grid which each character be fit to, usually equals to `font_size`.
  glyphGridSize?: number;

  /* text styles */
  fontSize?: number;
  fillColor?: Color;
  lineHeight?: number;
  indent?: number;

  stroke?: boolean;
  shadow?: boolean;

  strokeColor?: Color;
  strokeWidth?: number;

  shadowColor?: Color;
  shadowOffsetX?: number;
  shadowOffsetY?: number;
  shadowBlur?: number;
  shadowWidth?: number;
}

export declare namespace JSX {
  interface IntrinsicElements {
    container: DetailedHaiProps<HaiContainerAttributes>;
    sprite: DetailedHaiProps<HaiSpriteAttribute>;
    yuvsprite: DetailedHaiProps<HaiYUVSpriteAttribute>;
    video: DetailedHaiProps<HaiVideoAttribute>;
    text: DetailedHaiProps<HaiTextAttribute>;
  }
}
