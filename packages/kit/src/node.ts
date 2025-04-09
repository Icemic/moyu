import type { BubbleEvent } from './events/base';
import * as moyu from './moyu';
import { STATE } from './state';

export class Node {
  nodeId!: number;
  label?: string;

  props: Record<string, any> = {};
  listeners: Record<string, (evt?: BubbleEvent | Record<string, any>) => any> = {};

  static create(label: string, type: string, props: Record<string, any>) {
    const { children: _, ...rest } = props;
    const node = new Node();
    node.label = label;

    const [restProps, listeners] = filterProps(rest);
    node.props = restProps;
    node.listeners = listeners;

    node.nodeId = moyu.createInstance(type, label, restProps);
    STATE.nodeMap[node.nodeId] = node;
    return node;
  }

  static rootNode() {
    const node = new Node();
    node.label = 'rootNode';
    node.nodeId = 0;
    return node;
  }

  addChild(child: Node) {
    moyu.addChild(this.nodeId, child.nodeId);
  }

  insertChild(index: number, child: Node) {
    moyu.insertChild(this.nodeId, 0, child.nodeId);
  }

  insertChildBefore(beforeChild: Node, child: Node) {
    moyu.insertChildBefore(this.nodeId, beforeChild.nodeId, child.nodeId);
  }

  // removeChildAt(index: number): Node | undefined {
  //   return this.children.splice(index, 1)[0];
  // }

  removeChild(child: Node) {
    moyu.removeChild(this.nodeId, child.nodeId);
    moyu.destroyInstance(child.nodeId);
  }

  updateProps(props: Record<string, any>) {
    const [restProps, listeners] = filterProps(props);
    Object.assign(this.props, restProps);
    Object.assign(this.listeners, listeners);
    moyu.updateProps(this.nodeId, restProps);
  }

  executeCommand(payload: moyu.MoyuCommandPayload) {
    return moyu.executeNodeCommand(this.nodeId, payload);
  }
}

/// filter props to props and listeners
function filterProps(props: Record<string, any>) {
  const ret: Record<string, any> = {};
  const listeners: Record<string, (evt: BubbleEvent) => any> = {};
  for (const key in props) {
    // FIXME: it is not quite right to check if a key starts with 'on'
    if (key.startsWith('on')) {
      listeners[key] = props[key];
    } else {
      ret[key] = props[key];
    }
  }
  return [ret, listeners];
}
