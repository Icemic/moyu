import { executePluginCommand } from './moyu';
import type { JsonValue } from './bindings/serde_json/JsonValue';

const VARIABLES_DATA_ASSET_PATH = 'assets:///data/variables.json';

type VariableScope = 'archive' | 'global';

type VariableDefinition = {
  id: string;
  defaultValue: JsonValue;
};

type VariablesDocument = {
  archive: VariableDefinition[];
  global: VariableDefinition[];
};

function normalizeVariableDefinitions(value: unknown): VariableDefinition[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.flatMap((entry) => {
    if (!entry || typeof entry !== 'object') {
      return [];
    }

    const definition = entry as {
      id?: unknown;
      defaultValue?: unknown;
    };

    if (typeof definition.id !== 'string' || definition.id.length === 0) {
      return [];
    }

    return [{ id: definition.id, defaultValue: definition.defaultValue as JsonValue }];
  });
}

async function loadVariablesDocument(): Promise<VariablesDocument | null> {
  let rawText: string;

  try {
    rawText = await executePluginCommand('system', {
      subCommand: 'readFile',
      path: VARIABLES_DATA_ASSET_PATH,
    }) as string;
  } catch {
    return null;
  }

  let rawData: unknown;
  try {
    rawData = JSON.parse(rawText);
  } catch (error) {
    console.error('Failed to parse variable defaults data:', error);
    return null;
  }

  if (!rawData || typeof rawData !== 'object') {
    return {
      archive: [],
      global: [],
    };
  }

  const document = rawData as {
    archive?: unknown;
    global?: unknown;
  };

  return {
    archive: normalizeVariableDefinitions(document.archive),
    global: normalizeVariableDefinitions(document.global),
  };
}

function toVariableRecord(value: unknown): Record<string, unknown> {
  if (value instanceof Map) {
    return Object.fromEntries(value);
  }

  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }

  return value as Record<string, unknown>;
}

async function ensureVariableDefaults(scope: VariableScope): Promise<void> {
  const document = await loadVariablesDocument();
  if (!document) {
    return;
  }

  const definitions = document[scope];
  if (definitions.length === 0) {
    return;
  }

  const currentVariables = toVariableRecord(
    await executePluginCommand('scenario',
      scope === 'global'
        ? { subCommand: 'getPermanentVariables' }
        : { subCommand: 'getVariables' },
    ),
  );

  const missingVariables: Record<string, JsonValue> = {};
  for (const definition of definitions) {
    if (!Object.hasOwn(currentVariables, definition.id)) {
      missingVariables[definition.id] = definition.defaultValue;
    }
  }

  if (Object.keys(missingVariables).length === 0) {
    return;
  }

  await executePluginCommand('scenario',
    scope === 'global'
      ? {
          subCommand: 'setPermanentVariables',
          variables: missingVariables,
        }
      : {
          subCommand: 'setVariables',
          variables: missingVariables,
        },
  );
}

export async function initGlobalVariableDefaults(): Promise<void> {
  await ensureVariableDefaults('global');
}

export async function ensureArchiveVariableDefaults(): Promise<void> {
  await ensureVariableDefaults('archive');
}
