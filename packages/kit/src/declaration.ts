/* eslint-disable @typescript-eslint/no-empty-object-type */
import type { ClassAttributes, ReactNode } from 'react';
import type { Node } from './node';
import type { BubbleEvent } from './events/base';
import type { MouseEvent } from './events/mouse';
import type { TouchEvent } from './events/touch';
import type { KeyboardEvent } from './events/keyboard';

export type Tuple2 = [number, number] | readonly [number, number];
export type Tuple3 = [number, number, number] | readonly [number, number, number];
export type Tuple4 = [number, number, number, number] | readonly [number, number, number, number];

export type DetailedMoyuProps<E extends MoyuNodeAttributes> = ClassAttributes<Node> & E;

// interface DOMElement<P extends HTMLAttributes<T> | SVGAttributes<T>, T extends Element>
//   extends ReactElement<P, string> {
//   ref: LegacyRef<T>;
// }

export type MoyuEventHandler<T extends BubbleEvent> = (event: T) => void;

export interface MoyuListenerAttributes {
  onClick?: MoyuEventHandler<MouseEvent>;
  onMouseEnter?: MoyuEventHandler<MouseEvent>;
  onMouseLeave?: MoyuEventHandler<MouseEvent>;
  onMouseDown?: MoyuEventHandler<MouseEvent>;
  onMouseUp?: MoyuEventHandler<MouseEvent>;
  onMouseMove?: MoyuEventHandler<MouseEvent>;
  onKeyDown?: MoyuEventHandler<KeyboardEvent>;
  onKeyUp?: MoyuEventHandler<KeyboardEvent>;
  onKeyPress?: MoyuEventHandler<KeyboardEvent>;
  onTouchStart?: MoyuEventHandler<TouchEvent>;
  onTouchMove?: MoyuEventHandler<TouchEvent>;
  onTouchEnd?: MoyuEventHandler<TouchEvent>;
  onTouchCancel?: MoyuEventHandler<TouchEvent>;
}

export interface MoyuNodeAttributes extends MoyuListenerAttributes {
  label?: string;
  x?: number;
  y?: number;
  // the anchor for node, values range from 0.0 to 1.0.
  anchor?: Tuple2;
  // the pivot on ratation, values in pixels.
  pivot?: Tuple2;
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

export type FilterKind =
  | { type: 'blur'; radius: number }
  | {
      type: 'colorAdjust';
      brightness?: number;
      contrast?: number;
      saturation?: number;
      hue?: number;
    };

export type MoyuContainerAttributes = MoyuNodeAttributes;
export interface MoyuClipAttribute extends MoyuNodeAttributes {
  width?: number;
  height?: number;
}
export interface MoyuFilterAttribute extends MoyuNodeAttributes {
  filters?: FilterKind[];
}
export interface MoyuBackdropAttribute extends MoyuNodeAttributes {
  filters?: FilterKind[];
  width: number;
  height: number;
}

export interface MoyuSpriteAttribute extends MoyuNodeAttributes {
  src?: string;
  area?: Tuple4;
  mode?: 'normal' | 'nineslice';
  bounds?: Tuple4;
  nineSliceMode?: 'stretch' | 'repeat' | 'mirror' | 'blank';
  targetWidth?: number;
  targetHeight?: number;
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

export interface MoyuTextAttribute extends MoyuNodeAttributes {
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

  onStart?: () => void;
  onFinish?: () => void;
  onProgress?: (progress: number) => void;
}

// eslint-disable-next-line @typescript-eslint/no-namespace
export declare namespace JSX {
  type ElementType = string | React.JSXElementConstructor<any>;
  interface Element extends React.ReactElement<any, any> {}
  interface ElementClass extends React.Component<any> {
    render(): React.ReactNode;
  }
  interface ElementAttributesProperty {
    props: {};
  }
  interface ElementChildrenAttribute {
    children: {};
  }

  interface IntrinsicAttributes extends React.Attributes {}
  interface IntrinsicClassAttributes<T> extends React.ClassAttributes<T> {}

  interface IntrinsicElements {
    container: DetailedMoyuProps<MoyuContainerAttributes>;
    sprite: DetailedMoyuProps<MoyuSpriteAttribute>;
    clip: DetailedMoyuProps<MoyuClipAttribute>;
    filter: DetailedMoyuProps<MoyuFilterAttribute>;
    backdrop: DetailedMoyuProps<MoyuBackdropAttribute>;
    text: DetailedMoyuProps<MoyuTextAttribute>;
  }
}
