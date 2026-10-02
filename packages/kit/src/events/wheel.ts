import type { WheelEventDeltaMode } from '../events';
import type { BubbleEvent } from './base';

export interface WheelEvent extends BubbleEvent {
  kind: 'Wheel';
  currentTargetLabel?: string;
  deltaX: number;
  deltaY: number;
  deltaZ: number;
  deltaMode: WheelEventDeltaMode;
}
