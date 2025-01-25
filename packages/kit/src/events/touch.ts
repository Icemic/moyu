import type { BubbleEvent } from './base';

export enum TouchEventKind {
  TouchStart = 'TouchStart',
  TouchMove = 'TouchMove',
  TouchEnd = 'TouchEnd',
  TouchCancel = 'TouchCancel',
}

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
  layerX: number;
  layerY: number;
  identifier: number;
}
