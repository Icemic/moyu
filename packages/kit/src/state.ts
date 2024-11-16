import type { Node } from './node';

export type State = {
  // stores the node instances
  // inserts at `createInstance` and removes after `removeChild` (which calls `destroyInstance`
  // and it emits `NodeDestroyed` event)
  nodeMap: Record<string, Node>;
  // record whether a touch is moved in its lifecycle
  touchMoved: Record<number, boolean>;
};

export const STATE: State = {
  nodeMap: {},
  touchMoved: {},
};
// STATE.nodeMap[node.nodeId] = node;
