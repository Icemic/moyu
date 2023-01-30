import Reconciler from 'react-reconciler';
import { HostConfig } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants';
import { Node } from './node';
import { ReactElement } from 'react';
import { DetailedHaiProps, HaiNodeLikeAttributes } from './declaration';

type Type = 'sprite' | 'node';
type Props = DetailedHaiProps<HaiNodeLikeAttributes>;
type Container = Node;
type Instance = Node;
type TextInstance = never;
type SuspenseInstance = never;
type HydratableInstance = never;
type PublicInstance = any;
type HostContext = Record<string, any>;
type UpdatePayload = Record<string, any>;
type ChildSet = any; // TODO Placeholder for undocumented API
type TimeoutHandle = any;
type NoTimeout = any;

const hostConfig: HostConfig<
  Type,
  Props,
  Container,
  Instance,
  TextInstance,
  SuspenseInstance,
  HydratableInstance,
  PublicInstance,
  HostContext,
  UpdatePayload,
  ChildSet,
  TimeoutHandle,
  NoTimeout
> = {
  isPrimaryRenderer: false,
  supportsHydration: false,
  supportsMutation: true,
  supportsPersistence: false,

  /**
   * @required
   */
  createInstance(type, props, _root, _hostContext, _internalInstanceHandle) {
    console.log('createInstance', type);
    const node = Node.create(props.label, type, props);
    return node;
  },

  /**
   * throw if not supported
   * @required
   */
  createTextInstance(_text, _rootContainerInstance, _hostContext, _internalInstanceHandle) {
    console.log('createTextInstance');
    // return SpanNode({}, text) as SkNode;
    throw new Error('Text nodes are not supported yet');
  },

  /**
   * @required
   */
  appendInitialChild(parentInstance, child) {
    console.log('appendInitialChild');
    parentInstance.addChild(child);
  },

  /**
   * return true if needs `commitMount`
   * @required
   */
  finalizeInitialChildren(parentInstance, _type, _props, _rootContainerInstance, _hostContext) {
    console.log('finalizeInitialChildren', parentInstance);
    return false;
  },

  prepareUpdate: (instance, type, oldProps, newProps, _rootContainerInstance, _hostContext) => {
    console.log('prepareUpdate');
    // const propsAreEqual = shallowEq(oldProps, newProps);
    // if (propsAreEqual && !instance.memoizable) {
    //   return null;
    // }
    // console.log('update ', type);
    if (oldProps.label === newProps.label) {
      return null;
    }

    return { data: newProps };
  },

  shouldSetTextContent(_type, _props) {
    return false;
  },

  getRootHostContext: (_rootContainerInstance: Node) => {
    console.log('getRootHostContext');
    return null;
  },

  getChildHostContext(_parentHostContext, _type, _rootContainerInstance) {
    console.log('getChildHostContext');
    return {};
  },

  getPublicInstance(node: Instance) {
    console.log('getPublicInstance');
    return node;
  },

  prepareForCommit(_containerInfo) {
    console.log('prepareForCommit');
    return null;
  },

  resetAfterCommit(container) {
    console.log('resetAfterCommit');
    // TODO: this is not necessary in continuous rendering
    // container.redraw();
  },

  preparePortalMount: () => {
    console.log('preparePortalMount');
  },

  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  noTimeout: -1,

  // optional
  appendChild(parent, child) {
    console.log('appendChild', parent, child);
    parent.addChild(child);
  },

  appendChildToContainer(container, child) {
    console.log('appendChildToContainer', container, child);
    container.addChild(child);
  },

  insertBefore: (parent, child, before) => {
    parent.insertChildBefore(before, child);
  },

  insertInContainerBefore: (parent, child, before) => {
    parent.insertChildBefore(before, child);
  },

  removeChild: (parent, child) => {
    parent.removeChild(child);
  },

  removeChildFromContainer: (parent, child) => {
    parent.removeChild(child);
  },

  finalizeContainerChildren: () => {
    console.log('finalizeContainerChildren');
  },

  commitMount(instance, type, props, internalInstanceHandle) {
    // if finalizeInitialChildren = true
    console.log('commitMount');
  },

  commitUpdate(instance, _updatePayload, type, prevProps, nextProps, _internalHandle) {
    console.log('commitUpdate: ', type);
    // if (shallowEq(prevProps, nextProps) && allChildrenAreMemoized(instance)) {
    //   return;
    // }
    // bustBranchMemoization(instance);
    // instance.props = nextProps;
    instance.label = nextProps.label;
  },

  commitTextUpdate: (_textInstance: TextInstance, _oldText: string, _newText: string) => {
    //  textInstance.instance = newText;
  },

  clearContainer: (container) => {
    console.error('clearContainer not implement');
    // container.children.splice(0);
  },
  getCurrentEventPriority: function (): number {
    return DefaultEventPriority;
  },
  getInstanceFromNode(node: any): Reconciler.Fiber | null | undefined {
    throw new Error('Function not implemented.');
  },
  beforeActiveInstanceBlur: function (): void {
    throw new Error('Function not implemented.');
  },
  afterActiveInstanceBlur: function (): void {
    throw new Error('Function not implemented.');
  },
  prepareScopeUpdate: function (scopeInstance: any, instance: any): void {
    throw new Error('Function not implemented.');
  },
  getInstanceFromScope: function (scopeInstance: any): Node | null {
    throw new Error('Function not implemented.');
  },
  detachDeletedInstance: function (node: Node): void {
    throw new Error('Function not implemented.');
  },
};

export const HaiRenderer = Reconciler(hostConfig);

export interface RootOptions {
  /**
   * Prefix for `useId`.
   */
  identifierPrefix?: string;
  onRecoverableError?: (error: unknown) => void;
}

export function createRoot(options?: RootOptions) {
  const rootNode = Node.rootNode();
  const rootElement = HaiRenderer.createContainer(
    rootNode,
    0,
    null,
    false,
    false,
    options?.identifierPrefix ?? 'hai',
    options?.onRecoverableError ??
      ((err) => {
        console.error('unrecoverable error: ', err);
      }),
    null
  );

  return {
    render: (reactElement: ReactElement, callback?: (() => void) | null) => {
      // update the root Container
      return HaiRenderer.updateContainer(reactElement, rootElement, null, callback);
    },
  };
}
