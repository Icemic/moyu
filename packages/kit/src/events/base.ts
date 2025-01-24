export interface BubbleEvent {
  targetId: number;
  bubbleTargetIds: number[];
  currentTargetId: number;
  targetLabel?: string;
  currentTargetLabel?: string;
  stopPropagation: () => void;
  preventDefault: () => void;
  defaultPrevented: boolean;
  bubbles: boolean;
}

export function createBubbleEvent<T extends BubbleEvent>(
  body: Record<string, any> & { bubbleTargetIds: number[] },
  targetId: number,
  label: string,
): T {
  const event: BubbleEvent = {
    ...body,
    targetId,
    currentTargetId: targetId,
    targetLabel: label,
    currentTargetLabel: label,
    stopPropagation: () => {
      event.bubbles = false;
    },
    preventDefault: () => {
      event.defaultPrevented = true;
    },
    defaultPrevented: false,
    bubbles: true,
  };

  return event as T;
}
