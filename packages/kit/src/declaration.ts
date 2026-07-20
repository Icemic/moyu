/* eslint-disable @typescript-eslint/no-empty-object-type */
import type { ClassAttributes, ReactNode } from 'react';
import type { Node } from './node';
import type { BubbleEvent } from './events/base';
import type { MouseEvent } from './events/mouse';
import type { TouchEvent } from './events/touch';
import type { KeyboardEvent } from './events/keyboard';
import type { NodeProps } from './bindings/NodeProps';
import type { VBoxProps } from './bindings/VBoxProps';
import type { HBoxProps } from './bindings/HBoxProps';
import type { ClipProps } from './bindings/ClipProps';
import type { FilterProps } from './bindings/FilterProps';
import type { BackdropProps } from './bindings/BackdropProps';
import type { SpriteProps } from './bindings/SpriteProps';
import type { TextProps } from './bindings/TextProps';
import type { ShaderProps } from './bindings/ShaderProps';
import type { ShaderSlotProps } from './bindings/ShaderSlotProps';
import type { AnimationProps } from './bindings/AnimationProps';
import type { VideoProps } from './bindings/VideoProps';

export type Tuple2 = [number, number];
export type Tuple3 = [number, number, number];
export type Tuple4 = [number, number, number, number];

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

export type MoyuNodeAttributes = MoyuListenerAttributes &
  NodeProps & {
    children?: ReactNode;
  };

export type MoyuContainerAttributes = MoyuNodeAttributes;
export type MoyuVBoxAttributes = VBoxProps & MoyuNodeAttributes;
export type MoyuHBoxAttributes = HBoxProps & MoyuNodeAttributes;
export type MoyuClipAttributes = ClipProps & MoyuNodeAttributes;
export type MoyuFilterAttributes = FilterProps & MoyuNodeAttributes;
export type MoyuBackdropAttributes = BackdropProps & MoyuNodeAttributes;
export type MoyuAnimationAttributes = AnimationProps & MoyuNodeAttributes;
export type MoyuShaderAttributes = ShaderProps &
  MoyuNodeAttributes & {
    onPrepared?: () => void;
    onFinished?: () => void;
  };
export type MoyuShaderSlotAttributes = ShaderSlotProps & MoyuNodeAttributes;
export type MoyuVideoAttributes = VideoProps &
  MoyuNodeAttributes & {
    onEnded?: () => void;
    onStateChange?: (state: string) => void;
  };
export type MoyuSpriteAttributes = SpriteProps & MoyuNodeAttributes;
export type MoyuTextAttributes = TextProps &
  MoyuNodeAttributes & {
    onStart?: () => void;
    onFinish?: () => void;
    onProgress?: (progress: number) => void;
  };

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
    vbox: DetailedMoyuProps<MoyuVBoxAttributes>;
    hbox: DetailedMoyuProps<MoyuHBoxAttributes>;
    sprite: DetailedMoyuProps<MoyuSpriteAttributes>;
    text: DetailedMoyuProps<MoyuTextAttributes>;
    clip: DetailedMoyuProps<MoyuClipAttributes>;
    filter: DetailedMoyuProps<MoyuFilterAttributes>;
    backdrop: DetailedMoyuProps<MoyuBackdropAttributes>;
    animation: DetailedMoyuProps<MoyuAnimationAttributes>;
    shader: DetailedMoyuProps<MoyuShaderAttributes>;
    'shader-slot': DetailedMoyuProps<MoyuShaderSlotAttributes>;
    video: DetailedMoyuProps<MoyuVideoAttributes>;
  }
}
