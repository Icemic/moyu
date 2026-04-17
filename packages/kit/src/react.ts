/** biome-ignore-all lint/suspicious/noExplicitAny: original react code */
import React from 'react';
import type { Component, ReactElement } from 'react';
import Reconciler from 'react-reconciler';
import type { BaseErrorInfo, HostConfig } from 'react-reconciler';
import { DefaultEventPriority, NoEventPriority } from 'react-reconciler/constants.js';
import type { DetailedMoyuProps, MoyuNodeAttributes } from './declaration';
import { Node } from './node';

declare const process: {
  env: {
    NODE_ENV: string;
  };
};

type Type = string;
type Props = DetailedMoyuProps<MoyuNodeAttributes>;
type Container = Node;
type Instance = Node;
type TextInstance = never;
type SuspenseInstance = never;
type HydratableInstance = never;
type FormInstance = never;
type PublicInstance = any;
type HostContext = Record<string, any>;
type ChildSet = any; // TODO Placeholder for undocumented API
type TimeoutHandle = any;
type NoTimeout = any;
type TransitionStatus = any;

let currentUpdatePriority: number = NoEventPriority;

const hostConfig: HostConfig<
  Type,
  Props,
  Container,
  Instance,
  TextInstance,
  SuspenseInstance,
  HydratableInstance,
  FormInstance,
  PublicInstance,
  HostContext,
  ChildSet,
  TimeoutHandle,
  NoTimeout,
  TransitionStatus
> = {
  isPrimaryRenderer: true,
  supportsHydration: false,
  supportsMutation: true,
  supportsPersistence: false,

  /**
   * @required
   */
  createInstance(type, props, _root, _hostContext, _internalInstanceHandle) {
    // console.debug('createInstance', type);
    const node = Node.create(props.label ?? '', type, props);
    return node;
  },

  /**
   * throw if not supported
   * @required
   */
  createTextInstance(_text, _rootContainerInstance, _hostContext, _internalInstanceHandle) {
    // console.debug('createTextInstance');
    // return SpanNode({}, text) as SkNode;
    throw new Error('Text nodes are not supported yet');
  },

  /**
   * @required
   */
  appendInitialChild(parentInstance, child) {
    // console.debug('appendInitialChild');
    parentInstance.addChild(child);
  },

  /**
   * return true if needs `commitMount`
   * @required
   */
  finalizeInitialChildren(_parentInstance, _type, _props, _rootContainerInstance, _hostContext) {
    // console.debug('finalizeInitialChildren', parentInstance);
    return true;
  },

  shouldSetTextContent(_type, _props) {
    return false;
  },

  getRootHostContext: (_rootContainerInstance: Node) => {
    // console.debug('getRootHostContext');
    return {};
  },

  getChildHostContext(_parentHostContext, _type, _rootContainerInstance) {
    // console.debug('getChildHostContext');
    return _parentHostContext;
  },

  getPublicInstance(node: Instance) {
    // console.debug('getPublicInstance');
    return node;
  },

  prepareForCommit(_containerInfo) {
    // console.debug('prepareForCommit');
    return null;
  },

  resetAfterCommit(_container) {
    // console.debug('resetAfterCommit');
    // TODO: this is not necessary in continuous rendering
    // container.redraw();
  },

  preparePortalMount: () => {
    // console.debug('preparePortalMount');
  },

  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  noTimeout: -1,

  // optional
  appendChild(parent, child) {
    // console.debug('appendChild', parent, child);
    parent.addChild(child);
  },

  appendChildToContainer(container, child) {
    // console.debug('appendChildToContainer', container, child);
    container.addChild(child);
  },

  insertBefore: (parent, child, before) => {
    // console.debug('insertBefore', parent, child, before);
    parent.insertChildBefore(before, child);
  },

  insertInContainerBefore: (parent, child, before) => {
    parent.insertChildBefore(before, child);
  },

  removeChild: (parent, child) => {
    // console.debug('removeChild', parent, child);
    parent.removeChild(child);
  },

  removeChildFromContainer: (parent, child) => {
    // console.debug('removeChildFromContainer', parent, child);
    parent.removeChild(child);
  },

  finalizeContainerChildren: () => {
    // console.debug('finalizeContainerChildren');
  },

  commitMount(_instance, _type, _props, _internalInstanceHandle) {
    // if finalizeInitialChildren = true
    // console.debug('commitMount');
  },

  commitUpdate(instance, _type, prevProps, nextProps, _internalHandle) {
    // console.debug('commitUpdate: ', type, JSON.stringify(_updatePayload));
    const { label: _oldLabel, children: _oldChildren, ...oldProps } = prevProps;
    const { label, children, ...newProps } = nextProps;

    const changedProps: Record<string, any> = {};

    // Check for changed or new props
    for (const key in newProps) {
      if ((newProps as any)[key] !== (oldProps as any)[key]) {
        changedProps[key] = (newProps as any)[key];
      }
    }

    // Check for removed props
    for (const key in oldProps) {
      if (!(key in newProps)) {
        changedProps[key] = null;
      }
    }

    // Handle label especially if it's reactive
    if (label !== _oldLabel) {
      changedProps.label = label ?? null;
    }

    if (Object.keys(changedProps).length) {
      instance.updateProps(changedProps);
    }
  },

  commitTextUpdate: (_textInstance: TextInstance, _oldText: string, _newText: string) => {
    //  textInstance.instance = newText;
  },

  clearContainer: (_container) => {
    // console.error('clearContainer not implement');
    // container.children.splice(0);
  },
  getInstanceFromNode(_node: any): Reconciler.Fiber | null | undefined {
    console.error('getInstanceFromNode not implement');
    throw new Error('Function not implemented.');
  },
  beforeActiveInstanceBlur: (): void => {
    console.error('beforeActiveInstanceBlur not implement');
    // throw new Error('Function not implemented.');
  },
  afterActiveInstanceBlur: (): void => {
    console.error('afterActiveInstanceBlur not implement');
    // throw new Error('Function not implemented.');
  },
  prepareScopeUpdate: (_scopeInstance: any, _instance: any): void => {
    console.error('prepareScopeUpdate not implement');
    // throw new Error('Function not implemented.');
  },
  getInstanceFromScope: (_scopeInstance: any): Node | null => {
    console.error('getInstanceFromScope not implement');
    // throw new Error('Function not implemented.');
    return null;
  },
  detachDeletedInstance: (_node: Node): void => {
    // node will be destroyed by the engine
    // just do nothing here
  },
  NotPendingTransition: undefined,
  // see https://github.com/pmndrs/react-three-fiber/blob/2541e81fb6ddc22d0869b9eb5cdbedcbbc62324c/packages/fiber/src/core/reconciler.tsx#L570
  HostTransitionContext: /* @__PURE__ */ React.createContext<TransitionStatus>(
    null,
  ) as unknown as Reconciler.ReactContext<TransitionStatus>,
  setCurrentUpdatePriority(newPriority: number) {
    currentUpdatePriority = newPriority;
  },
  getCurrentUpdatePriority() {
    return currentUpdatePriority;
  },
  resolveUpdatePriority() {
    if (currentUpdatePriority !== NoEventPriority) return currentUpdatePriority;

    return DefaultEventPriority;
  },
  resetFormInstance() {},
  requestPostPaintCallback() {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => -1.1,
  maySuspendCommit: () => false,
  preloadInstance: () => true,
  startSuspendingCommit() {},
  suspendInstance() {},
  waitForCommitToBeReady: () => null,
};

export const MoyuRenderer = Reconciler(hostConfig);

export interface RootOptions {
  /**
   * Prefix for `useId`.
   */
  identifierPrefix?: string;
  onUncaughtError: (error: Error, info: BaseErrorInfo & { errorBoundary?: Component }) => void;
  onCaughtError: (error: Error, info: BaseErrorInfo) => void;
  onRecoverableError: (error: Error, info: BaseErrorInfo) => void;
  onDefaultTransitionIndicator: () => void;
}

export function createRoot(options?: RootOptions) {
  const rootNode = Node.rootNode();
  const rootElement = MoyuRenderer.createContainer(
    rootNode,
    0,
    null,
    false,
    false,
    options?.identifierPrefix ?? 'moyu',
    options?.onUncaughtError ??
      ((err) => {
        console.error('uncaught error: ', err);
      }),
    options?.onCaughtError ??
      ((err) => {
        console.error('caught error: ', err);
      }),
    options?.onRecoverableError ??
      ((err) => {
        console.error('unrecoverable error: ', err);
      }),
    options?.onDefaultTransitionIndicator ?? (() => {}),
  );

  MoyuRenderer.injectIntoDevTools({
    bundleType: process.env.NODE_ENV === 'production' ? 0 : 1,
    version: '0.1.0',
    rendererPackageName: '@momoyu-ink/kit',
    // eslint-disable-next-line @typescript-eslint/no-redundant-type-constituents
    findFiberByHostInstance: (instance: Instance | TextInstance) => {
      return instance as any;
    },
  });

  return {
    render: (reactElement: ReactElement, callback?: (() => void) | null) => {
      // update the root Container
      return MoyuRenderer.updateContainer(reactElement, rootElement, null, callback);
    },
  };
}
