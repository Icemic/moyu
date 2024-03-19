import { ClassAttributes, ReactNode } from 'react';
import * as PropTypes from 'prop-types';
import type { HaiEvent } from './hai';
import { Node } from './node';

export type DetailedHaiProps<E extends HaiNodeAttributes> = ClassAttributes<Node> & E;

// interface DOMElement<P extends HTMLAttributes<T> | SVGAttributes<T>, T extends Element>
//   extends ReactElement<P, string> {
//   ref: LegacyRef<T>;
// }

export interface HaiListenerAttributes {
  onClick?: (event: HaiEvent) => void;
  onMouseEnter?: (event: HaiEvent) => void;
  onMouseLeave?: (event: HaiEvent) => void;
  onMouseDown?: (event: HaiEvent) => void;
  onMouseUp?: (event: HaiEvent) => void;
  onMouseMove?: (event: HaiEvent) => void;
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

declare module 'react' {
  /* eslint-disable */
  namespace JSX {
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

    // naked 'any' type in a conditional type will short circuit and union both the then/else branches
    // so boolean is only resolved for T = any
    type IsExactlyAny<T> = boolean extends (T extends never ? true : false) ? true : false;

    type ExactlyAnyPropertyKeys<T> = { [K in keyof T]: IsExactlyAny<T[K]> extends true ? K : never }[keyof T];
    type NotExactlyAnyPropertyKeys<T> = Exclude<keyof T, ExactlyAnyPropertyKeys<T>>;

    // Try to resolve ill-defined props like for JS users: props can be any, or sometimes objects with properties of type any
    type MergePropTypes<P, T> =
      // Distribute over P in case it is a union type
      P extends any
        ? // If props is type any, use propTypes definitions
          IsExactlyAny<P> extends true
          ? T
          : // If declared props have indexed properties, ignore inferred props entirely as keyof gets widened
          string extends keyof P
          ? P
          : // Prefer declared types which are not exactly any
            Pick<P, NotExactlyAnyPropertyKeys<P>> &
              // For props which are exactly any, use the type inferred from propTypes if present
              Pick<T, Exclude<keyof T, NotExactlyAnyPropertyKeys<P>>> &
              // Keep leftover props not specified in propTypes
              Pick<P, Exclude<keyof P, keyof T>>
        : never;

    type InexactPartial<T> = { [K in keyof T]?: T[K] | undefined };

    // Any prop that has a default prop becomes optional, but its type is unchanged
    // Undeclared default props are augmented into the resulting allowable attributes
    // If declared props have indexed properties, ignore default props entirely as keyof gets widened
    // Wrap in an outer-level conditional type to allow distribution over props that are unions
    type Defaultize<P, D> = P extends any
      ? string extends keyof P
        ? P
        : Pick<P, Exclude<keyof P, keyof D>> &
            InexactPartial<Pick<P, Extract<keyof P, keyof D>>> &
            InexactPartial<Pick<D, Exclude<keyof D, keyof P>>>
      : never;

    type ReactManagedAttributes<C, P> = C extends { propTypes: infer T; defaultProps: infer D }
      ? Defaultize<MergePropTypes<P, PropTypes.InferProps<T>>, D>
      : C extends { propTypes: infer T }
      ? MergePropTypes<P, PropTypes.InferProps<T>>
      : C extends { defaultProps: infer D }
      ? Defaultize<P, D>
      : P;

    // We can't recurse forever because `type` can't be self-referential;
    // let's assume it's reasonable to do a single React.lazy() around a single React.memo() / vice-versa
    type LibraryManagedAttributes<C, P> = C extends
      | React.MemoExoticComponent<infer T>
      | React.LazyExoticComponent<infer T>
      ? T extends React.MemoExoticComponent<infer U> | React.LazyExoticComponent<infer U>
        ? ReactManagedAttributes<U, P>
        : ReactManagedAttributes<T, P>
      : ReactManagedAttributes<C, P>;

    interface IntrinsicAttributes extends React.Attributes {}
    interface IntrinsicClassAttributes<T> extends React.ClassAttributes<T> {}
    /* eslint-enable */

    interface IntrinsicElements {
      container: DetailedHaiProps<HaiContainerAttributes>;
      sprite: DetailedHaiProps<HaiSpriteAttribute>;
      yuvsprite: DetailedHaiProps<HaiYUVSpriteAttribute>;
      video: DetailedHaiProps<HaiVideoAttribute>;
      text: DetailedHaiProps<HaiTextAttribute>;
    }
  }
}
