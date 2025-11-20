import { type BubbleEvent, createBubbleEvent } from './events/base';
import { type NodeEvent, NodeEventKind } from './events/node';
import type { AnimationFrameCallbackEvent } from './events/raf';
import type { MouseEvent, MouseEventKind } from './events/mouse';
import { TouchEvent, TouchEventKind } from './events/touch';
import { KeyboardEvent, KeyboardEventKind } from './events/keyboard';
import type { CustomEvent } from './events/custom';
import { STATE } from './state';

export type {
  BubbleEvent,
  NodeEvent,
  NodeEventKind,
  AnimationFrameCallbackEvent,
  MouseEvent,
  MouseEventKind,
  TouchEvent,
  TouchEventKind,
  KeyboardEvent,
  KeyboardEventKind,
};

export interface MoyuEvent<T extends Record<string, unknown> = Record<string, unknown>> {
  name: string;
  body: T;
}

const globalEventListeners: Record<string, ((event: any) => void)[]> = {};
const globalRequestAnimationFrameListeners: FrameRequestCallback[] = [];

const BUBBLE_EVENT_NAMES = ['mouseevent', 'touchevent', 'keyboardevent'];

globalThis.__moyu_receive_event = (raw_event: MoyuEvent) => {
  const { name, body } = raw_event;

  if (BUBBLE_EVENT_NAMES.includes(name)) {
    handleBubbleEvent(name, body as unknown as MouseEvent | TouchEvent);
  } else {
    // handles non-dom events and return
    switch (name) {
      case 'customevent': {
        const { name, body: _body, targetId } = body as unknown as CustomEvent;
        // if targetId is 0, it is a global event (send to root node or send from plugin)
        if (targetId === 0) {
          for (const listener of globalEventListeners[name.toLowerCase()] ?? []) {
            listener(_body);
          }
          return;
        }
        STATE.nodeMap[targetId]?.listeners?.[`on${name.charAt(0).toUpperCase() + name.slice(1)}`]?.(_body);
        return;
      }
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
      default: {
        globalEventListeners[name]?.forEach((listener) => listener(body));
        return;
      }
    }
  }
};

function handleBubbleEvent(name: string, body: MouseEvent | TouchEvent) {
  const event: MouseEvent | TouchEvent = createBubbleEvent(body, body.targetId, body.targetLabel ?? '');

  const { kind, bubbleTargetIds } = body;

  // if targetId is 0, it is a global event (send to root node or send from plugin)
  if (body.targetId !== 0) {
    const node = STATE.nodeMap[body.targetId];

    if (!node) {
      return;
    }

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
      // if (kind === TouchEventKind.TouchEnd && !STATE.touchMoved[identifier] && !event.defaultPrevented) {
      //   for (const eventKind of ['MouseMove', 'MouseDown', 'MouseUp', 'Click'] as MouseEventKind[]) {
      //     event.bubbles = true;
      //     event.kind = eventKind;
      //     node?.listeners?.[`on${eventKind}`]?.(event);
      //     const _bubbleTargetIds = [...bubbleTargetIds];
      //     while (event.bubbles && _bubbleTargetIds.length) {
      //       // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
      //       event.currentTargetId = _bubbleTargetIds.pop()!;
      //       event.currentTargetLabel = STATE.nodeMap[event.currentTargetId]?.label;
      //       STATE.nodeMap[event.currentTargetId]?.listeners?.[`on${eventKind}`]?.(event);
      //     }
      //   }
      // }

      if (kind === TouchEventKind.TouchStart) {
        STATE.touchMoved[identifier] = false;
      } else if (kind === TouchEventKind.TouchMove) {
        STATE.touchMoved[identifier] = true;
      } else if (kind === TouchEventKind.TouchEnd || kind === TouchEventKind.TouchCancel) {
        delete STATE.touchMoved[identifier];
      }
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
    globalRequestAnimationFrameListeners.splice(handle, 1);
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
