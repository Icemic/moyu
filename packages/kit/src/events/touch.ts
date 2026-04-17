import type { TouchEventKind } from '../bindings/TouchEventKind';
import type { BubbleEvent } from './base';

export interface TouchEvent extends BubbleEvent {
  kind: TouchEventKind;
  targetId: number;
  currentTargetId: number;
  targetLabel: string;
  currentTargetLabel: string;
  clientX: number;
  clientY: number;
  screenX: number;
  screenY: number;
  offsetX: number;
  offsetY: number;
  identifier: number;
}
