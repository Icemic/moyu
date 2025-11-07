import type { BubbleEvent } from './base';

export enum MouseEventKind {
  MouseDown = 'MouseDown',
  MouseUp = 'MouseUp',
  MouseMove = 'MouseMove',
  MouseEnter = 'MouseEnter',
  MouseLeave = 'MouseLeave',
  Click = 'Click',
  DoubleClick = 'DoubleClick',
  ContextMenu = 'ContextMenu',
}

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
