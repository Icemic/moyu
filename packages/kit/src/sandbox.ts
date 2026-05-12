import { executePluginCommand } from './moyu';

// Snapshot of JS built-in globals captured at init time, used to let them bypass the sandbox proxy
const JS_GLOBAL_KEYS = new Set(Object.getOwnPropertyNames(globalThis));
const JS_GLOBAL_KEYS_EXCLUDED = new Set(['window', 'globalThis', 'LOCAL', 'ARCHIVE', 'GLOBAL']);
JS_GLOBAL_KEYS_EXCLUDED.forEach((key) => {
  JS_GLOBAL_KEYS.delete(key);
});

const localVariables = new Proxy(
  {},
  {
    get(_, key) {
      if (typeof key === 'string') {
        return executePluginCommand('scenario', {
          subCommand: 'getLocalVariable',
          name: key,
        });
      }
      return undefined;
    },
    set(_, key, value) {
      if (typeof key === 'string') {
        executePluginCommand('scenario', {
          subCommand: 'setLocalVariable',
          name: key,
          value,
        });
        return true;
      }
      return false;
    },
  },
);

const archiveVariables = new Proxy(
  {},
  {
    get(_, key) {
      if (typeof key === 'string') {
        return executePluginCommand('scenario', {
          subCommand: 'getVariable',
          name: key,
        });
      }
      return undefined;
    },
    set(_, key, value) {
      if (typeof key === 'string') {
        executePluginCommand('scenario', {
          subCommand: 'setVariable',
          name: key,
          value,
        });
        return true;
      }
      return false;
    },
  },
);

const globalVariables = new Proxy(
  {},
  {
    get(_, key) {
      if (typeof key === 'string') {
        return executePluginCommand('scenario', {
          subCommand: 'getPermanentVariable',
          key,
        });
      }
      return undefined;
    },
    set(_, key, value) {
      if (typeof key === 'string') {
        executePluginCommand('scenario', {
          subCommand: 'setPermanentVariable',
          key,
          value,
        });
        return true;
      }
      return false;
    },
  },
);

function safeEval(code: string, sandbox: Record<string, any>) {
  const proxy = new Proxy(sandbox, {
    has(target, key) {
      // Let JS built-ins fall through to the actual global scope, intercept everything else
      if (typeof key === 'string' && JS_GLOBAL_KEYS.has(key)) {
        return false;
      }

      return (
        key in target ||
        key === 'LOCAL' ||
        key === 'ARCHIVE' ||
        key === 'GLOBAL' ||
        key === 'window' ||
        key === 'globalThis'
      );
    },
    get(target, key) {
      if (key === 'window' || key === 'globalThis') return proxy; // Make window/globalThis refer to the sandbox itself
      if (key === 'LOCAL') return localVariables;
      if (key === 'ARCHIVE') return archiveVariables;
      if (key === 'GLOBAL') return globalVariables;

      if (key === Symbol.unscopables) return undefined;

      return target[key as string];
    },
    set(target, key, value) {
      target[key as string] = value;
      return true;
    },
  });

  // eslint-disable-next-line @typescript-eslint/no-implied-eval
  return new Function('sandbox', `with(sandbox) { return (${code}) }`).bind(proxy)(proxy);
}

globalThis.__moyu_eval_sandbox = (code: string) => {
  try {
    return safeEval(code, {});
  } catch (err) {
    console.error('Error evaluating code in sandbox:', err);
    throw err;
  }
};
