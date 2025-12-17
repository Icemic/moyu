export interface MoyuEvent<T extends Record<string, unknown> = Record<string, unknown>> {
  name: string;
  body: T;
}

export interface BubbleEvent {
  targetId: number;
  bubbleTargetIds: number[];
  currentTargetId: number;
  stopPropagation: () => void;
  preventDefault: () => void;
  defaultPrevented: boolean;
  bubbles: boolean;
}

export function createBubbleEvent<T extends BubbleEvent>(
  body: Record<string, any> & { bubbleTargetIds: number[] },
  targetId: number,
): T {
  const event: BubbleEvent = {
    ...body,
    targetId,
    currentTargetId: targetId,
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
