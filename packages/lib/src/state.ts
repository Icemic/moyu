import type { Node } from './node';

export const STATE: { nodeMap: Record<number, Node> } = {
  // stores the node instances
  // inserts at `createInstance` and removes after `removeChild` (which calls `destroyInstance`
  // and it emits `NodeDestroyed` event)
  nodeMap: {},
};
// STATE.nodeMap[node.nodeId] = node;
