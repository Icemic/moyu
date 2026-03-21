import type { Command, MaybePromise } from './commands';
import type { MoyuEvent } from './events';

declare const moyu: {
  pushCommand: (name: string, args: any[], callback?: (...args: any[]) => void) => any;
  executeNodeCommand: (nodeId: number, payload: Command) => MaybePromise;
  executePluginCommand: (pluginName: string, payload: Command) => MaybePromise;
  [key: string]: (...args: any[]) => any;
};

declare global {
  interface Window {
    moyu: any;
  }

  // eslint-disable-next-line no-var
  var __moyu_receive_event: (event: MoyuEvent) => void;
  // eslint-disable-next-line no-var
  var __moyu_eval_sandbox: (code: string) => any;
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

export function updateProps(nodeId: number, props: Record<string, any>) {
  moyu.pushCommand('update_props', [nodeId, props]);
}

export function executeNodeCommand(nodeId: number, payload: Command): MaybePromise {
  return moyu.executeNodeCommand(nodeId, payload);
}

export function executePluginCommand(pluginName: string, payload: Command): MaybePromise {
  return moyu.executePluginCommand(pluginName, payload);
}
