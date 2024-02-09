import * as hai from './hai';
import { STATE } from './state';

export class Node {
  nodeId!: number;
  label?: string;

  props: Record<string, any> = {};

  static create(label = '', type: string, props: Record<string, any>) {
    const { children: _, ...rest } = props;
    const node = new Node();
    node.label = label;
    node.props = rest;
    node.nodeId = hai.createInstance(type, label, rest);
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
    hai.updateProps(this.nodeId, props);
  }
}
