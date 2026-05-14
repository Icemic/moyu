import { executePluginCommand } from './moyu';
import { proxy, useSnapshot } from 'valtio';
import type { ZodType } from 'zod';

const UI_DATA_ASSET_PATH = 'assets:///data/ui.json';

type UiDocument = Record<string, unknown>;
type UiSchema<TUiDocument extends UiDocument> = ZodType<TUiDocument>;

const uiDataState = proxy<UiDocument>({});
const uiRuntimeState = proxy({
  initialized: false,
  error: null as string | null,
});

export interface RootUiDataMap {
  __uiDataBrand__?: never;
}

type RootUiDataKeys = Exclude<keyof RootUiDataMap, '__uiDataBrand__'>;
type ResolvedUiDataMap = RootUiDataKeys extends never ? Record<string, unknown> : Omit<RootUiDataMap, '__uiDataBrand__'>;
type UiDataName = Extract<keyof ResolvedUiDataMap, string>;

export async function initUiData<TUiDocument extends UiDocument>(schema: UiSchema<TUiDocument>): Promise<void> {
  uiRuntimeState.initialized = false;
  uiRuntimeState.error = null;

  let rawData: unknown;
  try {
    const rawText = await Promise.resolve(
      executePluginCommand('system', {
        subCommand: 'readFile',
        path: UI_DATA_ASSET_PATH,
      }) as string | Promise<string>,
    );
    rawData = JSON.parse(rawText);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    uiRuntimeState.error = message;
    throw new Error(`Failed to initialize UI data: ${message}`);
  }

  const parsed = schema.safeParse(rawData);
  if (!parsed.success) {
    uiRuntimeState.error = parsed.error.message;
    throw new Error(`Invalid UI data: ${parsed.error.message}`);
  }

  for (const key of Object.keys(uiDataState)) {
    delete uiDataState[key];
  }

  Object.assign(uiDataState, parsed.data);
  uiRuntimeState.initialized = true;
}

export function getUiData<Name extends UiDataName>(name: Name): ResolvedUiDataMap[Name] {
  if (!uiRuntimeState.initialized) {
    throw new Error('UI data has not been initialized. Call initUiData() before reading it.');
  }

  if (!(name in uiDataState)) {
    throw new Error(`UI data "${name}" is not available.`);
  }

  return uiDataState[name] as ResolvedUiDataMap[Name];
}

export function useUiData<Name extends UiDataName>(name: Name): ResolvedUiDataMap[Name] {
  if (!uiRuntimeState.initialized) {
    throw new Error('UI data has not been initialized. Call initUiData() before rendering UI pages.');
  }

  const snapshot = useSnapshot(uiDataState);
  if (!(name in snapshot)) {
    throw new Error(`UI data "${name}" is not available.`);
  }

  return snapshot[name] as ResolvedUiDataMap[Name];
}
