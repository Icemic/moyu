import type { WheelEventDeltaMode } from '../events';
import type { MouseEvent } from './mouse';

export interface WheelEvent extends Omit<MouseEvent, 'kind'> {
  kind: 'Wheel';
  deltaX: number;
  deltaY: number;
  deltaZ: number;
  deltaMode: WheelEventDeltaMode;
}
