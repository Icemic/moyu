/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
declare const hai: any;

export function loadPreset(name: string) {
  hai.pushCommand('load_preset', [name]);
}

export function resizeWindow(logicalWidth: number, logicalHeight: number, factor?: number) {
  hai.pushCommand('resize_window', [logicalWidth, logicalHeight, factor]);
}

export function quit() {
  hai.pushCommand('quit', []);
}

export function createInstance(nodeType: 'node' | 'sprite', props: Record<string, any>) {
  let node_id = 0;
  hai.pushCommand('create_instance', [nodeType, props], (id: number) => {
    node_id = id;
  });
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
  hai.pushCommand('move_to', [nodeId, x, y]);
}

export function getTranslate(nodeId: number) {
  let x = 0;
  let y = 0;
  hai.pushCommand('get_translate', [nodeId], (_x: number, _y: number) => {
    x = _x;
    y = _y;
  });

  return { x, y };
}
