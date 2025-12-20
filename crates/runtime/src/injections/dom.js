/**
 * Minimal DOM polyfill for Webpack HMR
 */

(function () {
  if (typeof eval === 'undefined' && typeof globalThis.__moyu_eval === 'function') {
    globalThis.eval = globalThis.__moyu_eval;
  }

  class ScriptElement {
    constructor() {
      this.src = '';
      this.onload = null;
      this.onerror = null;
      this._type = 'script';
    }

    setAttribute(name, value) {
      this[name] = value;
    }

    getAttribute(name) {
      return this[name];
    }
  }

  const document = {
    createElement(tagName) {
      if (tagName.toLowerCase() === 'script') {
        return new ScriptElement();
      }
      return {
        setAttribute: () => {},
        getAttribute: () => {},
        style: {},
      };
    },
    getElementsByTagName(tagName) {
      if (tagName.toLowerCase() === 'head') {
        return [document.head];
      }
      return [];
    },
    head: {
      appendChild(element) {
        if (element instanceof ScriptElement && element.src) {
          // Simulate script loading
          (async () => {
            try {
              const response = await fetch(element.src);
              if (!response.ok) throw new Error(`Failed to load script: ${response.status}`);
              const code = await response.text();

              // Execute the script in global scope
              // We use eval.call(null, ...) to ensure it runs in global scope
              (0, eval)(code);

              if (typeof element.onload === 'function') {
                element.onload({ type: 'load', target: element });
              }
            } catch (e) {
              console.error(`[HMR] Script load error (${element.src}):`, e.toString());
              if (typeof element.onerror === 'function') {
                element.onerror({ type: 'error', target: element });
              }
            }
          })();
        }
        return element;
      },
    },
    // Some HMR logic might check for these
    body: {
      appendChild: (el) => el,
    },
    // for webpack's auto public path
    currentScript: {
      get tagName() {
        return 'script';
      },
      get src() {
        return globalThis.__moyu_base_url || '';
      },
    },
  };

  globalThis.document = document;
  if (typeof window !== 'undefined') {
    window.document = document;
  }
})();
