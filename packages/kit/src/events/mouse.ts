import { MouseEventKind } from '../bindings/MouseEventKind';
import type { BubbleEvent } from './base';

export interface MouseEvent extends BubbleEvent {
  kind: MouseEventKind;
  targetId: number;
  currentTargetId: number;
  targetLabel?: string;
  currentTargetLabel?: string;
  clientX: number;
  clientY: number;
  screenX: number;
  screenY: number;
  offsetX: number;
  offsetY: number;
}
