import type { MoyuEvent } from './events';
import { STATE } from './state';

declare const moyu: {
  pushCommand: (name: string, args: any[], callback?: (...args: any[]) => void) => any;
  executeNodeCommand: (nodeId: number, payload: MoyuCommandPayload) => any;
  executePluginCommand: (pluginName: string, payload: MoyuCommandPayload) => any;
  [key: string]: (...args: any[]) => any;
};

export interface MoyuCommandPayload extends Record<string, any> {
  subCommand: string;
}

declare global {
  interface Window {
    moyu: any;
  }

  var __moyu_receive_event: (event: MoyuEvent) => void;
}

export function createInstance(nodeType: string, label: string | undefined, props: Record<string, any>) {
  let node_id = 0;
  const ret: number = moyu.pushCommand('create_instance', [nodeType, label, props], (id: number) => {
    node_id = id;
  });
  if (ret) {
    return ret;
  }
  return node_id;
}

export function destroyInstance(nodeId: number) {
  moyu.pushCommand('destroy_instance', [nodeId]);
}

export function addChild(nodeId: number, childNodeId: number) {
  moyu.pushCommand('add_child', [nodeId, childNodeId]);
}

export function insertChild(nodeId: number, index: number, childNodeId: number) {
  moyu.pushCommand('insert_child', [nodeId, index, childNodeId]);
}

export function insertChildBefore(nodeId: number, beforeChildNodeId: number, childNodeId: number) {
  moyu.pushCommand('insert_child_before', [nodeId, beforeChildNodeId, childNodeId]);
}

export function removeChildAt(nodeId: number, index: number) {
  moyu.pushCommand('remove_child_at', [nodeId, index]);
}

export function removeChild(nodeId: number, childNodeId: number) {
  moyu.pushCommand('remove_child', [nodeId, childNodeId]);
}

export function moveTo(nodeId: number, x: number, y: number) {
  // moyu.pushCommand('move_to', [nodeId, x, y]);
  moyu.pushCommand('update_props', [nodeId, { x, y }]);
}

export function getTranslate(nodeId: number) {
  let x = 0;
  let y = 0;
  const ret = moyu.pushCommand('get_translate', [nodeId], (_x: number, _y: number) => {
    x = _x;
    y = _y;
  });

  if (ret) {
    return { x: ret[0], y: ret[1] };
  }

  return { x, y };
}

export function updateProps(nodeId: number, props: Record<string, any>) {
  moyu.pushCommand('update_props', [nodeId, props]);
}

export function executeNodeCommand(nodeId: number, payload: MoyuCommandPayload) {
  return moyu.executeNodeCommand(nodeId, payload);
}

export function executePluginCommand(pluginName: string, payload: MoyuCommandPayload) {
  return moyu.executePluginCommand(pluginName, payload);
}
