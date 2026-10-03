import type { Node } from './node';

export type State = {
  // stores the node instances
  // inserts at `createInstance` and removes after `removeChild` (which calls `destroyInstance`
  // and it emits `NodeDestroyed` event)
  nodeMap: Record<string, Node>;
};

export const STATE: State = {
  nodeMap: {},
};
// STATE.nodeMap[node.nodeId] = node;
