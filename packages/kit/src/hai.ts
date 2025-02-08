import type { HaiEvent } from './events';
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

  var __doufu_receive_event: (event: HaiEvent) => void;
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
