import * as hai from './hai';
import { STATE } from './state';

// const HAI_EVENT_KINDS = [
//   'NodeDestroyed',
//   'MouseEnter',
//   'MouseLeave',
//   'MouseDown',
//   'MouseUp',
//   'MouseMove',
//   'Click',
// ] as const;

export class Node {
  nodeId!: number;
  label?: string;

  props: Record<string, any> = {};
  listeners: Record<string, (evt: hai.HaiEvent) => any> = {};

  static create(label = '', type: string, props: Record<string, any>) {
    const { children: _, ...rest } = props;
    const node = new Node();
    node.label = label;

    const [restProps, listeners] = filterProps(rest);
    node.props = restProps;
    node.listeners = listeners;

    node.nodeId = hai.createInstance(type, label, restProps);
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
    hai.addChild(this.nodeId, child.nodeId);
  }

  insertChild(index: number, child: Node) {
    hai.insertChild(this.nodeId, 0, child.nodeId);
  }

  insertChildBefore(beforeChild: Node, child: Node) {
    hai.insertChildBefore(this.nodeId, beforeChild.nodeId, child.nodeId);
  }

  // removeChildAt(index: number): Node | undefined {
  //   return this.children.splice(index, 1)[0];
  // }

  removeChild(child: Node) {
    hai.removeChild(this.nodeId, child.nodeId);
    hai.destroyInstance(child.nodeId);
  }

  updateProps(props: Record<string, any>) {
    const [restProps, listeners] = filterProps(props);
    Object.assign(this.props, restProps);
    Object.assign(this.listeners, listeners);
    hai.updateProps(this.nodeId, restProps);
  }
}

/// filter props to props and listeners
function filterProps(props: Record<string, any>) {
  const ret: Record<string, any> = {};
  const listeners: Record<string, (evt: hai.HaiEvent) => any> = {};
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
