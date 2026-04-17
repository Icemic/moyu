import type { KeyboardEventKind } from '../bindings/KeyboardEventKind';
import type { BubbleEvent } from './base';

/**
 * KeyboardEvent objects describe a user interaction with the keyboard; each event describes a single interaction between the user and a key (or combination of a key with modifier keys) on the keyboard.
 *
 * [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent)
 */
export interface KeyboardEvent extends BubbleEvent {
  readonly kind: KeyboardEventKind;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/altKey) */
  readonly altKey: boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/code) */
  readonly code: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/ctrlKey) */
  readonly ctrlKey: boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/isComposing) */
  readonly isComposing: boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/key) */
  readonly key: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/location) */
  readonly location: number;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/metaKey) */
  readonly metaKey: boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/repeat) */
  readonly repeat: boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/shiftKey) */
  readonly shiftKey: boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/KeyboardEvent/getModifierState)
   *
   * Not implemented yet
   */
  getModifierState(keyArg: string): boolean;
  readonly DOM_KEY_LOCATION_STANDARD: 0x00;
  readonly DOM_KEY_LOCATION_LEFT: 0x01;
  readonly DOM_KEY_LOCATION_RIGHT: 0x02;
  readonly DOM_KEY_LOCATION_NUMPAD: 0x03;
}
