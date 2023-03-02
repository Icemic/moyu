/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
declare const hai: any;

if (hai && typeof hai.pushCommand === 'undefined') {
  const __hai = hai;
  window.hai = {};
  hai.pushCommand = function pushCommand(name: string, args: any[]) {
    return __hai[name](...args);
  };
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
