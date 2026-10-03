import type { MouseEventKind } from '../bindings/MouseEventKind';
import type { BubbleEvent } from './base';

/**
 * Buttons of a mouse, as `MouseEvent.button` reports them. Middle is 1, not 2: the
 * middle button is DOM's auxiliary button.
 */
export const MouseButton = {
  Left: 0,
  Middle: 1,
  Right: 2,
  Back: 3,
  Forward: 4,
} as const;

/**
 * Bits of the `MouseEvent.buttons` mask. The auxiliary (middle) button is 4, because
 * the low three bits are reserved for primary, secondary and auxiliary.
 */
export const MouseButtons = {
  Left: 1,
  Right: 2,
  Middle: 4,
  Back: 8,
  Forward: 16,
} as const;

export interface MouseEvent extends BubbleEvent {
  kind: MouseEventKind;
  targetId: number;
  currentTargetId: number;
  targetLabel?: string;
  currentTargetLabel?: string;
  /**
   * Stage logical coordinates of the pointer, an alias of `clientX` / `clientY` that
   * matches the DOM `MouseEvent.x` / `MouseEvent.y`.
   */
  x: number;
  y: number;
  clientX: number;
  clientY: number;
  screenX: number;
  screenY: number;
  offsetX: number;
  offsetY: number;
  /**
   * The button this event is about: 0 left, 1 middle, 2 right, 3 back, 4 forward.
   */
  button: number;
  /**
   * Buttons held when the event fired, as a bitmask: 1 left, 2 right, 4 middle, 8 back,
   * 16 forward.
   */
  buttons: number;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  /**
   * True when the engine produced this event rather than a mouse device: the mouse
   * compatibility gesture of a touch tap, and the hover refresh a frame performs while
   * the pointer stays where it is. A consumer that wants to know whether the user is
   * operating a mouse should ignore synthetic events.
   */
  synthetic: boolean;
}
