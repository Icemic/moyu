import React, { createContext, useContext } from 'react';
import { proxy, useSnapshot } from 'valtio';

// ============================================================================
// React Contexts for Parameters
// ============================================================================

/**
 * Context for page parameters
 * Used to provide params to page components via React tree
 */
export const PageParamsContext = createContext<Record<string, any> | undefined>(undefined);

/**
 * Context for overlay parameters
 * Used to provide params to overlay components via React tree
 */
export const OverlayParamsContext = createContext<Record<string, any> | undefined>(undefined);

// ============================================================================
// Type Definitions
// ============================================================================

/**
 * Placeholder for global navigation types.
 * Projects can augment this interface to provide type safety for the global navigator.
 * @example
 * declare module './utils/navigation' {
 *   interface RootNavigatorList {
 *     pages: typeof myPages;
 *     overlays: typeof myOverlays;
 *   }
 * }
 */
// biome-ignore lint/suspicious/noEmptyInterface: This interface is intended to be augmented by projects for type safety.
export interface RootNavigatorList {}

/**
 * Type helper to register a navigator instance's types globally.
 * Use this in a declare module block to get automatic type safety.
 */
export type RegisterNavigator<T extends Navigator<any, any>> = {
  pages: T extends Navigator<infer P, any> ? Record<P, any> : never;
  overlays: T extends Navigator<any, infer O> ? Record<O, any> : never;
};

type GetPages<T> = T extends { pages: infer P } ? (P extends Record<string, any> ? keyof P & string : string) : string;
type GetOverlays<T> = T extends { overlays: infer O }
  ? O extends Record<string, any>
    ? keyof O & string
    : string
  : string;

export type DefaultPageName = GetPages<RootNavigatorList>;
export type DefaultOverlayName = GetOverlays<RootNavigatorList>;

export interface PageDescriptor {
  component: React.ComponentType<any>;
  options?: Record<string, any>;
  requiredParams?: string[];
}

export interface OverlayDescriptor {
  component: React.ComponentType<any>;
  options?: Record<string, any>;
  requiredParams?: string[];
}

// Allow shorthand syntax: can pass component directly or as descriptor
export type PageConfig = PageDescriptor['component'] | PageDescriptor;
export type OverlayConfig = OverlayDescriptor['component'] | OverlayDescriptor;

export interface StackNavigatorOptions<
  Pages extends Record<string, PageConfig>,
  Overlays extends Record<string, OverlayConfig>,
> {
  initialPage: keyof Pages & string;
  pages: Pages;
  overlays?: Overlays;
}

export interface Navigator<PageName extends string = DefaultPageName, OverlayName extends string = DefaultOverlayName> {
  /**
   * Navigate to a page
   * @param page Page name
   * @param params Parameters to pass to the page (optional)
   */
  navigate: (page: PageName, params?: Record<string, any>) => void;

  /**
   * Go back to the previous page (not implemented yet)
   */
  goBack: () => void;

  /**
   * Push an overlay to the overlay stack
   * @param overlay Overlay name
   * @param params Parameters to pass to the overlay (optional)
   */
  pushOverlay: (overlay: OverlayName, params?: Record<string, any>) => void;

  /**
   * Pop the top overlay from the stack
   */
  popOverlay: () => void;

  /**
   * Clear all overlays
   */
  clearOverlays: () => void;

  /**
   * Get current page name
   */
  getCurrentPage: () => PageName;

  /**
   * Get current page parameters
   */
  getParams: () => Record<string, any> | undefined;

  /**
   * Get overlay stack
   */
  getOverlayStack: () => OverlayInfo[];

  /**
   * Check if an overlay is active
   * @param type Overlay type/name
   */
  isOverlayActive: (type: OverlayName) => boolean;

  /** @internal */
  _descriptors: {
    pages: Record<string, PageDescriptor>;
    overlays: Record<string, OverlayDescriptor>;
  };
  /** @internal */
  _state: NavigationState;
}

// ============================================================================
// Navigation State
// ============================================================================

export interface OverlayInfo {
  type: string; // Overlay type
  id: string; // Unique ID
}

interface NavigationState {
  currentPage: string;
  pageParams: Record<string, Record<string, any>>;
  overlayStack: OverlayInfo[];
  overlayParams: Record<string, Record<string, any>>;
}

// ============================================================================
// Helper Functions
// ============================================================================

let overlayIdCounter = 0;
const generateOverlayId = (): string => {
  return `overlay-${++overlayIdCounter}-${Date.now()}`;
};

/**
 * Validate navigation parameters against descriptor schema
 */
function validateParams(
  type: 'page' | 'overlay',
  name: string,
  descriptor: PageDescriptor | OverlayDescriptor | undefined,
  params: Record<string, any> | undefined,
) {
  if (!descriptor?.requiredParams) return;

  const actualParams = params ?? {};
  for (const key of descriptor.requiredParams) {
    if (actualParams[key] === undefined) {
      throw new Error(
        `Navigation Error: Missing required parameter "${key}" for ${type} "${name}". ` +
          `Expected: { ${descriptor.requiredParams.join(', ')} }`,
      );
    }
  }
}

// ============================================================================
// Global Navigation State and Navigator
// ============================================================================

let globalNavigator: Navigator<any, any> | null = null;

/**
 * Normalize page/overlay config to descriptor format
 * Allows shorthand syntax: Component or { component: Component, options?: {} }
 */
function normalizeDescriptor<T extends PageDescriptor | OverlayDescriptor>(config: React.ComponentType<any> | T): T {
  if (typeof config === 'function' || (config as any).$$typeof) {
    // It's a component (function or React element)
    return { component: config } as T;
  }
  // It's already a descriptor
  return config as T;
}

/**
 * Create a stack navigator
 * @param options Navigator configuration
 * @returns Navigator instance
 */
export function createStackNavigator<
  Pages extends Record<string, PageConfig>,
  Overlays extends Record<string, OverlayConfig>,
>(options: StackNavigatorOptions<Pages, Overlays>): Navigator<keyof Pages & string, keyof Overlays & string> {
  // Normalize pages and overlays
  const normalizedPages: Record<string, PageDescriptor> = {};
  for (const [key, config] of Object.entries(options.pages)) {
    normalizedPages[key] = normalizeDescriptor(config);
  }

  const normalizedOverlays: Record<string, OverlayDescriptor> = {};
  if (options.overlays) {
    for (const [key, config] of Object.entries(options.overlays)) {
      normalizedOverlays[key] = normalizeDescriptor(config);
    }
  }

  // Initialize navigation state
  const navigationState = proxy<NavigationState>({
    currentPage: options.initialPage as string,
    pageParams: {},
    overlayStack: [],
    overlayParams: {},
  });

  // Create navigator API
  const navigator: Navigator<keyof Pages & string, keyof Overlays & string> = {
    navigate: (page, params) => {
      // Check if page exists
      if (!normalizedPages[page as string]) {
        console.error(`Navigation Error: Page "${page}" not found in navigator configuration.`);
        return;
      }

      // Validate parameters
      validateParams('page', page as string, normalizedPages[page as string], params);

      navigationState.currentPage = page as string;
      if (params !== undefined) {
        navigationState.pageParams[page as string] = params;
      } else {
        // Clear params if no params provided
        delete navigationState.pageParams[page as string];
      }
    },

    goBack: () => {
      // TODO: Implement history stack
      console.warn('goBack() not implemented yet');
    },

    pushOverlay: (overlay, params) => {
      // Check if overlay exists
      if (!normalizedOverlays[overlay as string]) {
        console.error(`Navigation Error: Overlay "${overlay}" not found in navigator configuration.`);
        return;
      }

      // Validate parameters
      validateParams('overlay', overlay as string, normalizedOverlays[overlay as string], params);

      const id = generateOverlayId();
      const overlayInfo: OverlayInfo = { type: overlay as string, id };
      navigationState.overlayStack.push(overlayInfo);

      if (params !== undefined) {
        navigationState.overlayParams[id] = params;
      }
    },

    popOverlay: () => {
      const overlay = navigationState.overlayStack.pop();
      if (overlay) {
        delete navigationState.overlayParams[overlay.id];
      }
    },

    clearOverlays: () => {
      // Clear all overlay params
      for (const overlay of navigationState.overlayStack) {
        delete navigationState.overlayParams[overlay.id];
      }
      // Clear stack
      navigationState.overlayStack = [];
    },

    getCurrentPage: () => {
      return navigationState.currentPage as keyof Pages & string;
    },

    getParams: () => {
      return navigationState.pageParams[navigationState.currentPage];
    },

    getOverlayStack: () => {
      return [...navigationState.overlayStack];
    },

    isOverlayActive: (type) => {
      return navigationState.overlayStack.some((overlay) => overlay.type === (type as string));
    },

    _descriptors: {
      pages: normalizedPages,
      overlays: normalizedOverlays,
    },
    _state: navigationState,
  };

  // Store as global navigator
  globalNavigator = navigator;

  return navigator;
}

/**
 * Create a navigation component for a navigator
 * @param navigator The navigator instance
 * @returns A React component that renders the navigation state
 */
export function createStaticNavigation(navigator: Navigator<any, any>): React.FC {
  return () => {
    // navigator._state is a Valtio proxy, useSnapshot makes it reactive
    const navState = useSnapshot(navigator._state as any) as NavigationState;
    const { pages, overlays } = navigator._descriptors;

    // Get current page component
    const pageDescriptor = pages[navState.currentPage];
    const Page = pageDescriptor?.component || (() => null);

    return (
      <>
        <PageParamsContext.Provider value={navState.pageParams[navState.currentPage]}>
          <Page />
        </PageParamsContext.Provider>

        {navState.overlayStack.map((overlay) => {
          const overlayDescriptor = overlays[overlay.type];
          const OverlayComponent = overlayDescriptor?.component || (() => null);
          return (
            <OverlayParamsContext.Provider key={overlay.id} value={navState.overlayParams[overlay.id]}>
              <OverlayComponent />
            </OverlayParamsContext.Provider>
          );
        })}
      </>
    );
  };
}

// ============================================================================
// React Hooks
// ============================================================================

/**
 * Get the navigator instance (stable reference)
 * Can be used in both components and state logic
 * @returns Navigator object
 */
export function getNavigator<
  PageName extends string = DefaultPageName,
  OverlayName extends string = DefaultOverlayName,
>(): Navigator<PageName, OverlayName> {
  if (!globalNavigator) {
    throw new Error('Navigator not initialized. Call createStackNavigator first.');
  }
  return globalNavigator as unknown as Navigator<PageName, OverlayName>;
}

/**
 * Hook to get the navigator instance in components
 */
export const useNavigation = getNavigator;

/**
 * Get navigation parameters for current component
 * @returns Parameters object (cast as T)
 *
 * This hook reads params from React Context, which is provided by the router.
 * - If called from within an overlay context, returns overlay params
 * - If called from within a page context, returns page params
 * - Supports multiple nested overlays (each gets its own params)
 *
 * Note: We cast the result to T and provide an empty object as fallback.
 * Since required parameters are validated at navigation entry points, we can
 * safely assume the runtime object matches T.
 */
export function useNavigationParams<T = Record<string, any>>(): T {
  const overlayParams = useContext(OverlayParamsContext);
  const pageParams = useContext(PageParamsContext);

  // If in overlay context, return overlay params; otherwise return page params
  // Default to empty object if both are undefined
  return (overlayParams ?? pageParams ?? {}) as T;
}
