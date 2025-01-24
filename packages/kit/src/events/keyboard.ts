import type { BubbleEvent } from './base';

export enum KeyboardEventKind {}

export interface KeyboardEvent extends BubbleEvent {
  kind: KeyboardEventKind;
}
