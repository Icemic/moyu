/* eslint-disable @typescript-eslint/no-unsafe-call */

import { STATE } from './state';

/* eslint-disable @typescript-eslint/no-unsafe-member-access */
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

  // eslint-disable-next-line no-var
  var __hai_receive_event: (event: HaiRawEvent) => void;
}

interface HaiRawEvent {
  kind: string;
  targetId: number;
  bubbleTargetIds: number[];
  location?: [number, number, number, number];
  identifier?: number;
}

globalThis.__hai_receive_event = (raw_event: HaiRawEvent) => {
  const { kind, targetId: target_id, bubbleTargetIds: bubble_target_ids, location, identifier } = raw_event;

  const node = STATE.nodeMap[target_id];

  let propagate = true;
  let preventDefault = false;

  const event: HaiEvent = {
    kind,
    target_id,
    current_target_id: target_id,
    target_label: node.label,
    current_target_label: node.label,
    stopPropagation: () => {
      propagate = false;
    },
    preventDefault: () => {
      preventDefault = true;
    },
  };

  if (location) {
    event.client_x = location[0];
    event.client_y = location[1];
    event.screen_x = location[2];
    event.screen_y = location[3];
  }

  if (identifier) {
    event.identifier = identifier;
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
      node?.listeners?.['on' + kind]?.(event);
      while (propagate && bubble_target_ids.length) {
        event.current_target_id = bubble_target_ids.pop()!;
        event.current_target_label = STATE.nodeMap[event.current_target_id]?.label;
        STATE.nodeMap[event.current_target_id]?.listeners?.['on' + kind]?.(event);
      }

      break;
    case 'TouchStart':
    case 'TouchMove':
    case 'TouchEnd':
    case 'TouchCancel':
      node?.listeners?.['on' + kind]?.(event);
      {
        const _bubble_target_ids = [...bubble_target_ids];
        while (propagate && _bubble_target_ids.length) {
          event.current_target_id = _bubble_target_ids.pop()!;
          event.current_target_label = STATE.nodeMap[event.current_target_id]?.label;
          STATE.nodeMap[event.current_target_id]?.listeners?.['on' + kind]?.(event);
        }
      }

      // simulate mouse events as same as browsers
      if (kind === 'TouchEnd' && !STATE.touchMoved[identifier!] && !preventDefault) {
        for (const eventKind of ['MouseMove', 'MouseDown', 'MouseUp', 'Click']) {
          propagate = true;
          event.kind = eventKind;
          node?.listeners?.['on' + eventKind]?.(event);
          const _bubble_target_ids = [...bubble_target_ids];
          while (propagate && _bubble_target_ids.length) {
            event.current_target_id = _bubble_target_ids.pop()!;
            event.current_target_label = STATE.nodeMap[event.current_target_id]?.label;
            STATE.nodeMap[event.current_target_id]?.listeners?.['on' + eventKind]?.(event);
          }
        }
      }

      if (kind === 'TouchStart') {
        STATE.touchMoved[identifier!] = false;
      } else if (kind === 'TouchMove') {
        STATE.touchMoved[identifier!] = true;
      } else if (kind === 'TouchEnd' || kind === 'TouchCancel') {
        delete STATE.touchMoved[identifier!];
      }

      break;
    default:
      break;
  }
};

export interface HaiEvent {
  kind: string;
  target_id: number;
  current_target_id: number;
  target_label?: string;
  current_target_label?: string;
  client_x?: number;
  client_y?: number;
  screen_x?: number;
  screen_y?: number;
  identifier?: number;
  stopPropagation: () => void;
  preventDefault: () => void;
}

export function addEventListener(name: string, callback: (...args: any[]) => void) {
  hai.pushCommand('add_event_listener', [name, callback]);
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
