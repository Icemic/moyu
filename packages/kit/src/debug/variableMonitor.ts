import type { JsonValue } from '../bindings/serde_json/JsonValue';
import { executePluginCommand } from '../moyu';
import { loadVariablesDocument } from '../variables';

type RuntimeDebugVariableScope = 'archive' | 'global';

type RuntimeDebugVariableRecord = Record<string, JsonValue>;

type RuntimeDebugVariableAllowlist = {
  archive: Set<string>;
  global: Set<string>;
};

export type RuntimeDebugVariablesSnapshotReason =
  | 'init'
  | 'jump-done'
  | 'jump-error'
  | 'route-done'
  | 'route-error';

export interface RuntimeDebugVariablesSnapshotPayload {
  type: 'variables:snapshot';
  reason: RuntimeDebugVariablesSnapshotReason;
  archive: RuntimeDebugVariableRecord;
  global: RuntimeDebugVariableRecord;
}

export interface RuntimeDebugVariablesChangedPayload {
  type: 'variables:changed';
  archiveSet?: RuntimeDebugVariableRecord;
  globalSet?: RuntimeDebugVariableRecord;
}

export type RuntimeDebugVariableMessagePayload =
  | RuntimeDebugVariablesSnapshotPayload
  | RuntimeDebugVariablesChangedPayload;

let sendRuntimeDebugVariableMessage: ((message: RuntimeDebugVariableMessagePayload) => void) | null = null;
let allowlistCache: RuntimeDebugVariableAllowlist | null = null;
let allowlistPromise: Promise<RuntimeDebugVariableAllowlist> | null = null;
let variableEmissionSuspended = false;

function createEmptyAllowlist(): RuntimeDebugVariableAllowlist {
  return {
    archive: new Set<string>(),
    global: new Set<string>(),
  };
}

function isJsonValue(value: unknown): value is JsonValue {
  if (
    value === null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean'
  ) {
    return true;
  }

  if (Array.isArray(value)) {
    return value.every((item) => isJsonValue(item));
  }

  if (!value || typeof value !== 'object') {
    return false;
  }

  return Object.values(value).every((item) => isJsonValue(item));
}

async function loadRuntimeDebugVariableAllowlist(): Promise<RuntimeDebugVariableAllowlist> {
  const document = await loadVariablesDocument();
  if (!document) {
    return createEmptyAllowlist();
  }

  return {
    archive: new Set(document.archive.map((def) => def.id)),
    global: new Set(document.global.map((def) => def.id)),
  };
}

function toJsonValueRecord(value: unknown): RuntimeDebugVariableRecord {
  if (value instanceof Map) {
    return Object.fromEntries(
      Array.from(value.entries()).flatMap(([key, entryValue]) => {
        if (typeof key !== 'string' || !isJsonValue(entryValue)) {
          return [];
        }

        return [[key, entryValue]];
      }),
    );
  }

  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value).flatMap(([key, entryValue]) => {
      if (!isJsonValue(entryValue)) {
        return [];
      }

      return [[key, entryValue]];
    }),
  );
}

function filterVariablesByAllowlist(
  scope: RuntimeDebugVariableScope,
  values: RuntimeDebugVariableRecord,
  allowlist: RuntimeDebugVariableAllowlist,
): RuntimeDebugVariableRecord {
  const allowedIds = allowlist[scope];
  if (allowedIds.size === 0) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(values).flatMap(([key, value]) => (allowedIds.has(key) ? [[key, value]] : [])),
  );
}

function hasRecordEntries(value: RuntimeDebugVariableRecord | undefined): value is RuntimeDebugVariableRecord {
  return !!value && Object.keys(value).length > 0;
}

async function ensureRuntimeDebugVariableAllowlist(): Promise<RuntimeDebugVariableAllowlist> {
  if (allowlistCache) {
    return allowlistCache;
  }

  if (!allowlistPromise) {
    allowlistPromise = loadRuntimeDebugVariableAllowlist()
      .then((allowlist) => {
        allowlistCache = allowlist;
        return allowlist;
      })
      .finally(() => {
        allowlistPromise = null;
      });
  }

  return allowlistPromise;
}

async function emitRuntimeDebugVariablesChanged(
  scope: RuntimeDebugVariableScope,
  variables: RuntimeDebugVariableRecord,
): Promise<void> {
  const sender = sendRuntimeDebugVariableMessage;
  if (!sender || variableEmissionSuspended) {
    return;
  }

  const allowlist = await ensureRuntimeDebugVariableAllowlist();
  if (!sendRuntimeDebugVariableMessage || sendRuntimeDebugVariableMessage !== sender || variableEmissionSuspended) {
    return;
  }

  const filtered = filterVariablesByAllowlist(scope, variables, allowlist);
  if (!hasRecordEntries(filtered)) {
    return;
  }

  sender({
    type: 'variables:changed',
    archiveSet: scope === 'archive' ? filtered : undefined,
    globalSet: scope === 'global' ? filtered : undefined,
  });
}

export function setRuntimeDebugVariableMessageSender(
  sender: ((message: RuntimeDebugVariableMessagePayload) => void) | null,
): void {
  sendRuntimeDebugVariableMessage = sender;

  if (sender) {
    void ensureRuntimeDebugVariableAllowlist();
  }
}

export function setRuntimeDebugVariableEmissionSuspended(suspended: boolean): void {
  variableEmissionSuspended = suspended;
}

export function resetRuntimeDebugVariableMonitor(): void {
  sendRuntimeDebugVariableMessage = null;
  allowlistCache = null;
  allowlistPromise = null;
  variableEmissionSuspended = false;
}

export function reportRuntimeDebugVariableSet(
  scope: RuntimeDebugVariableScope,
  key: string,
  value: JsonValue,
): void {
  if (key.length === 0) {
    return;
  }

  void emitRuntimeDebugVariablesChanged(scope, {
    [key]: value,
  });
}

export function reportRuntimeDebugVariablesSet(
  scope: RuntimeDebugVariableScope,
  variables: RuntimeDebugVariableRecord,
): void {
  if (Object.keys(variables).length === 0) {
    return;
  }

  void emitRuntimeDebugVariablesChanged(scope, variables);
}

export async function syncRuntimeDebugVariablesSnapshot(
  reason: RuntimeDebugVariablesSnapshotReason,
): Promise<void> {
  const sender = sendRuntimeDebugVariableMessage;
  if (!sender) {
    return;
  }

  const [allowlist, archiveVariables, globalVariables] = await Promise.all([
    ensureRuntimeDebugVariableAllowlist(),
    executePluginCommand('scenario', {
      subCommand: 'getVariables',
    }),
    executePluginCommand('scenario', {
      subCommand: 'getPermanentVariables',
    }),
  ]);

  if (!sendRuntimeDebugVariableMessage || sendRuntimeDebugVariableMessage !== sender) {
    return;
  }

  sender({
    type: 'variables:snapshot',
    reason,
    archive: filterVariablesByAllowlist('archive', toJsonValueRecord(archiveVariables), allowlist),
    global: filterVariablesByAllowlist('global', toJsonValueRecord(globalVariables), allowlist),
  });
}
