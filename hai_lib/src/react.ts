import Reconciler from 'react-reconciler';
import { HostConfig } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants';
import { omitBy } from 'lodash-es';
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
    console.debug('createInstance', type);
    const node = Node.create(props.label, type, props);
    return node;
  },

  /**
   * throw if not supported
   * @required
   */
  createTextInstance(_text, _rootContainerInstance, _hostContext, _internalInstanceHandle) {
    console.debug('createTextInstance');
    // return SpanNode({}, text) as SkNode;
    throw new Error('Text nodes are not supported yet');
  },

  /**
   * @required
   */
  appendInitialChild(parentInstance, child) {
    console.debug('appendInitialChild');
    parentInstance.addChild(child);
  },

  /**
   * return true if needs `commitMount`
   * @required
   */
  finalizeInitialChildren(parentInstance, _type, _props, _rootContainerInstance, _hostContext) {
    console.debug('finalizeInitialChildren', parentInstance);
    return false;
  },

  prepareUpdate: (
    instance,
    type,
    oldProps: Record<string, any>,
    newProps: Record<string, any>,
    _rootContainerInstance,
    _hostContext
  ) => {
    const changedProps = omitBy(newProps, (value, key) => oldProps[key] === value);

    if (Object.keys(changedProps).length) {
      return changedProps;
    }

    return null;
  },

  shouldSetTextContent(_type, _props) {
    return false;
  },

  getRootHostContext: (_rootContainerInstance: Node) => {
    console.debug('getRootHostContext');
    return null;
  },

  getChildHostContext(_parentHostContext, _type, _rootContainerInstance) {
    console.debug('getChildHostContext');
    return {};
  },

  getPublicInstance(node: Instance) {
    console.debug('getPublicInstance');
    return node;
  },

  prepareForCommit(_containerInfo) {
    console.debug('prepareForCommit');
    return null;
  },

  resetAfterCommit(container) {
    console.debug('resetAfterCommit');
    // TODO: this is not necessary in continuous rendering
    // container.redraw();
  },

  preparePortalMount: () => {
    console.debug('preparePortalMount');
  },

  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  noTimeout: -1,

  // optional
  appendChild(parent, child) {
    console.debug('appendChild', parent, child);
    parent.addChild(child);
  },

  appendChildToContainer(container, child) {
    console.debug('appendChildToContainer', container, child);
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
    console.debug('finalizeContainerChildren');
  },

  commitMount(instance, type, props, internalInstanceHandle) {
    // if finalizeInitialChildren = true
    console.debug('commitMount');
  },

  commitUpdate(instance, _updatePayload, type, prevProps, nextProps, _internalHandle) {
    console.debug('commitUpdate: ', type, JSON.stringify(_updatePayload));
    // if (shallowEq(prevProps, nextProps) && allChildrenAreMemoized(instance)) {
    //   return;
    // }
    // bustBranchMemoization(instance);
    // instance.props = nextProps;
    // instance.label = nextProps.label;
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
