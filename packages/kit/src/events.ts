import { type BubbleEvent, createBubbleEvent } from './events/base';
import { type NodeEvent, NodeEventKind } from './events/node';
import type { AnimationFrameCallbackEvent } from './events/raf';
import type { MouseEvent, MouseEventKind } from './events/mouse';
import { type TouchEvent, TouchEventKind } from './events/touch';
import { STATE } from './state';

export interface HaiEvent<T extends Record<string, unknown> = Record<string, unknown>> {
  name: string;
  body: T;
}

const globalEventListeners: Record<string, ((event: BubbleEvent) => void)[]> = {};
const globalRequestAnimationFrameListeners: FrameRequestCallback[] = [];

const BUBBLE_EVENT_NAMES = ['mouseevent', 'touchevent', 'keyboardevent'];

globalThis.__hai_receive_event = (raw_event: HaiEvent) => {
  const { name, body } = raw_event;

  if (BUBBLE_EVENT_NAMES.includes(name)) {
    handleBubbleEvent(name, body as unknown as MouseEvent | TouchEvent);
  } else {
    // handles non-dom events and return
    switch (name) {
      case 'nodeevent': {
        const { kind, targetId } = body as unknown as NodeEvent;
        if (kind === NodeEventKind.Destory) {
          delete STATE.nodeMap[targetId];
        }
        return;
      }
      case 'animationframecallbackevent': {
        const listeners = globalRequestAnimationFrameListeners.splice(0);
        const { timestamp } = body as unknown as AnimationFrameCallbackEvent;
        for (const listener of listeners) {
          listener?.(timestamp);
        }
        return;
      }
    }
  }
};

function handleBubbleEvent(name: string, body: MouseEvent | TouchEvent) {
  // handles dom events

  const event: MouseEvent | TouchEvent = createBubbleEvent(body, body.targetId, body.targetLabel ?? '');

  const node = STATE.nodeMap[body.targetId];

  if (!node) {
    console.error(`Node not found: ${body.targetId}`);
    return;
  }

  // let propagate = true;

  // const event: HaiEvent = {
  //   kind,
  //   targetId,
  //   currentTargetId: targetId,
  //   targetLabel: node.label,
  //   currentTargetLabel: node.label,
  //   stopPropagation: () => {
  //     propagate = false;
  //   },
  //   preventDefault: () => {
  //     event.defaultPrevented = true;
  //   },
  //   defaultPrevented: false,
  // };
  const { kind, bubbleTargetIds } = body;

  if (['mouseevent', 'keyboardevent'].includes(name)) {
    node?.listeners?.[`on${kind}`]?.(event);
    while (event.bubbles && bubbleTargetIds.length) {
      // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
      event.currentTargetId = bubbleTargetIds.pop()!;
      event.currentTargetLabel = STATE.nodeMap[event.currentTargetId]?.label;
      STATE.nodeMap[event.currentTargetId]?.listeners?.[`on${kind}`]?.(event);
    }
  } else if (name === 'touchevent') {
    const { identifier } = body as TouchEvent;
    (event as TouchEvent).identifier = identifier;
    node?.listeners?.[`on${kind}`]?.(event);
    {
      const _bubbleTargetIds = [...bubbleTargetIds];
      while (event.bubbles && _bubbleTargetIds.length) {
        // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
        event.currentTargetId = _bubbleTargetIds.pop()!;
        event.currentTargetLabel = STATE.nodeMap[event.currentTargetId]?.label;
        STATE.nodeMap[event.currentTargetId]?.listeners?.[`on${kind}`]?.(event);
      }
    }

    // simulate mouse events as same as browsers
    if (kind === 'TouchEnd' && !STATE.touchMoved[identifier] && !event.defaultPrevented) {
      for (const eventKind of ['MouseMove', 'MouseDown', 'MouseUp', 'Click'] as MouseEventKind[]) {
        event.bubbles = true;
        event.kind = eventKind;
        node?.listeners?.[`on${eventKind}`]?.(event);
        const _bubbleTargetIds = [...bubbleTargetIds];
        while (event.bubbles && _bubbleTargetIds.length) {
          // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
          event.currentTargetId = _bubbleTargetIds.pop()!;
          event.currentTargetLabel = STATE.nodeMap[event.currentTargetId]?.label;
          STATE.nodeMap[event.currentTargetId]?.listeners?.[`on${eventKind}`]?.(event);
        }
      }
    }

    if (kind === 'TouchStart') {
      STATE.touchMoved[identifier] = false;
    } else if (kind === 'TouchMove') {
      STATE.touchMoved[identifier] = true;
    } else if (kind === 'TouchEnd' || kind === 'TouchCancel') {
      delete STATE.touchMoved[identifier];
    }
  }

  if (event.bubbles) {
    for (const listener of globalEventListeners[kind.toLowerCase()] ?? []) {
      listener(event);
    }
  }
}

if (!globalThis.document) {
  // detect if it is running in browser, if not, polyfill requestAnimationFrame
  globalThis.requestAnimationFrame = (callback: FrameRequestCallback) => {
    return globalRequestAnimationFrameListeners.push(callback) - 1;
  };

  globalThis.cancelAnimationFrame = (handle: number) => {
    delete globalRequestAnimationFrameListeners[handle];
  };
}

export function addEventListener(name: string, callback: (...args: any[]) => void): () => void {
  if (!globalEventListeners[name]) {
    globalEventListeners[name] = [];
  }

  globalEventListeners[name].push(callback);

  return () => {
    globalEventListeners[name] = globalEventListeners[name].filter((cb) => cb !== callback);
  };
}
