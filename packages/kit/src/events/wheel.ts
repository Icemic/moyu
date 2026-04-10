import { WheelEventDeltaMode } from '../events';
import type { MouseEvent } from './mouse';

export interface WheelEvent extends Omit<MouseEvent, 'kind'> {
  kind: 'wheel';
  deltaX: number;
  deltaY: number;
  deltaZ: number;
  deltaMode: WheelEventDeltaMode;
}
