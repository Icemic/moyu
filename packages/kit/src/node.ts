import type { Command } from './commands';
import type { BubbleEvent } from './events/base';
import * as moyu from './moyu';
import { STATE } from './state';

export class Node {
  public nodeId!: number;
  public label?: string;

  public props: Record<string, any> = {};
  public listeners: Record<string, (evt?: BubbleEvent | Record<string, any>) => any> = {};

  public static create(label: string, type: string, props: Record<string, any>) {
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

  public static rootNode() {
    const node = new Node();
    node.label = 'rootNode';
    node.nodeId = 0;
    return node;
  }

  public addChild(child: Node) {
    moyu.addChild(this.nodeId, child.nodeId);
  }

  public insertChild(_index: number, child: Node) {
    moyu.insertChild(this.nodeId, 0, child.nodeId);
  }

  public insertChildBefore(beforeChild: Node, child: Node) {
    moyu.insertChildBefore(this.nodeId, beforeChild.nodeId, child.nodeId);
  }

  // removeChildAt(index: number): Node | undefined {
  //   return this.children.splice(index, 1)[0];
  // }

  public removeChild(child: Node) {
    moyu.removeChild(this.nodeId, child.nodeId);
    moyu.destroyInstance(child.nodeId);
  }

  public updateProps(props: Record<string, any>) {
    const [restProps, listeners] = filterProps(props);
    Object.assign(this.props, restProps);
    Object.assign(this.listeners, listeners);

    // skip empty props
    if (Object.keys(restProps).length > 0) {
      moyu.updateProps(this.nodeId, restProps);
    }
  }

  public executeCommand(payload: Command) {
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
