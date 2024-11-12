import { STATE } from './state';

declare const hai: {
  pushCommand: (name: string, args: any[], callback?: (...args: any[]) => void) => any;
  executeNodeCommand: (nodeId: number, payload: HaiCommandPayload) => any;
  executePluginCommand: (pluginName: string, payload: HaiCommandPayload) => any;
  [key: string]: (...args: any[]) => any;
};

export interface HaiCommandPayload extends Record<string, any> {
  subCommand: string;
}

declare global {
  interface Window {
    hai: any;
  }

  var __hai_receive_event: (event: HaiRawEvent) => void;
}

interface HaiRawEvent {
  kind: string;
  targetId: number;
  bubbleTargetIds: number[];
  location?: [number, number, number, number, number, number];
  identifier?: number;
}

const globalEventListeners: Record<string, ((event: HaiEvent) => void)[]> = {};

globalThis.__hai_receive_event = (raw_event: HaiRawEvent) => {
  const { kind, targetId: target_id, bubbleTargetIds: bubble_target_ids, location, identifier } = raw_event;

  const node = STATE.nodeMap[target_id];

  let propagate = true;

  const event: HaiEvent = {
    kind,
    targetId: target_id,
    currentTargetId: target_id,
    targetLabel: node.label,
    currentTargetLabel: node.label,
    stopPropagation: () => {
      propagate = false;
    },
    preventDefault: () => {
      event.defaultPrevented = true;
    },
    defaultPrevented: false,
  };

  if (location) {
    event.clientX = location[0];
    event.clientY = location[1];
    event.screenX = location[2];
    event.screenY = location[3];
    event.layerX = location[4];
    event.layerY = location[5];
  }

  switch (kind) {
    case 'NodeDestroyed':
      delete STATE.nodeMap[target_id];
      break;
    case 'MouseEnter':
    case 'MouseLeave':
    case 'MouseDown':
    case 'MouseUp':
    case 'MouseMove':
    case 'Click':
    case 'KeyDown':
    case 'KeyUp':
    case 'KeyPress':
      node?.listeners?.[`on${kind}`]?.(event);
      while (propagate && bubble_target_ids.length) {
        // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
        event.currentTargetId = bubble_target_ids.pop()!;
        event.currentTargetLabel = STATE.nodeMap[event.currentTargetId]?.label;
        STATE.nodeMap[event.currentTargetId]?.listeners?.[`on${kind}`]?.(event);
      }

      break;
    case 'TouchStart':
    case 'TouchMove':
    case 'TouchEnd':
    case 'TouchCancel':
      if (typeof identifier === 'undefined') {
        console.error('Touch event without identifier');
        break;
      }

      event.identifier = identifier;
      node?.listeners?.[`on${kind}`]?.(event);
      {
        const _bubble_target_ids = [...bubble_target_ids];
        while (propagate && _bubble_target_ids.length) {
          // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
          event.currentTargetId = _bubble_target_ids.pop()!;
          event.currentTargetLabel = STATE.nodeMap[event.currentTargetId]?.label;
          STATE.nodeMap[event.currentTargetId]?.listeners?.[`on${kind}`]?.(event);
        }
      }

      // simulate mouse events as same as browsers
      if (kind === 'TouchEnd' && !STATE.touchMoved[identifier] && !event.defaultPrevented) {
        for (const eventKind of ['MouseMove', 'MouseDown', 'MouseUp', 'Click']) {
          propagate = true;
          event.kind = eventKind;
          node?.listeners?.[`on${eventKind}`]?.(event);
          const _bubble_target_ids = [...bubble_target_ids];
          while (propagate && _bubble_target_ids.length) {
            // biome-ignore lint/style/noNonNullAssertion: we are sure that the array is not empty, it is a bug of biomejs
            event.currentTargetId = _bubble_target_ids.pop()!;
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

      break;
    default:
      break;
  }

  if (propagate) {
    for (const listener of globalEventListeners[kind.toLowerCase()] ?? []) {
      listener(event);
    }
  }
};

export interface HaiEvent {
  kind: string;
  targetId: number;
  currentTargetId: number;
  targetLabel?: string;
  currentTargetLabel?: string;
  clientX?: number;
  clientY?: number;
  screenX?: number;
  screenY?: number;
  layerX?: number;
  layerY?: number;
  identifier?: number;
  stopPropagation: () => void;
  preventDefault: () => void;
  defaultPrevented: boolean;
}

export function loadPreset(name: string) {
  hai.pushCommand('load_preset', [name]);
}

export function loadResources() {
  hai.pushCommand('load_resources', []);
}

export function resizeWindow(logicalWidth: number, logicalHeight: number, factor?: number) {
  hai.pushCommand('resize_window', [logicalWidth, logicalHeight, factor]);
}

export function setIdle() {
  hai.pushCommand('set_idle', []);
}

export function setFullscreen() {
  hai.pushCommand('set_fullscreen', []);
}

export function setMaximized() {
  hai.pushCommand('set_maximized', []);
}

export function setMinimized() {
  hai.pushCommand('set_minimized', []);
}

export function quit() {
  hai.pushCommand('quit', []);
}

export function createInstance(nodeType: string, label: string | undefined, props: Record<string, any>) {
  let node_id = 0;
  const ret: number = hai.pushCommand('create_instance', [nodeType, label, props], (id: number) => {
    node_id = id;
  });
  if (ret) {
    return ret;
  }
  return node_id;
}

export function destroyInstance(nodeId: number) {
  hai.pushCommand('destroy_instance', [nodeId]);
}

export function addChild(nodeId: number, childNodeId: number) {
  hai.pushCommand('add_child', [nodeId, childNodeId]);
}

export function insertChild(nodeId: number, index: number, childNodeId: number) {
  hai.pushCommand('insert_child', [nodeId, index, childNodeId]);
}

export function insertChildBefore(nodeId: number, beforeChildNodeId: number, childNodeId: number) {
  hai.pushCommand('insert_child_before', [nodeId, beforeChildNodeId, childNodeId]);
}

export function removeChildAt(nodeId: number, index: number) {
  hai.pushCommand('remove_child_at', [nodeId, index]);
}

export function removeChild(nodeId: number, childNodeId: number) {
  hai.pushCommand('remove_child', [nodeId, childNodeId]);
}

export function moveTo(nodeId: number, x: number, y: number) {
  // hai.pushCommand('move_to', [nodeId, x, y]);
  hai.pushCommand('update_props', [nodeId, { x, y }]);
}

export function getTranslate(nodeId: number) {
  let x = 0;
  let y = 0;
  const ret = hai.pushCommand('get_translate', [nodeId], (_x: number, _y: number) => {
    x = _x;
    y = _y;
  });

  if (ret) {
    return { x: ret[0], y: ret[1] };
  }

  return { x, y };
}

export function updateProps(nodeId: number, props: Record<string, any>) {
  hai.pushCommand('update_props', [nodeId, props]);
}

export function executeNodeCommand(nodeId: number, payload: HaiCommandPayload) {
  return hai.executeNodeCommand(nodeId, payload);
}

export function executePluginCommand(pluginName: string, payload: HaiCommandPayload) {
  return hai.executePluginCommand(pluginName, payload);
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
