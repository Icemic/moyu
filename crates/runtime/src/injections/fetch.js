/**
 * Fetch polyfill for QuickJS environment
 */

(function () {
  if (typeof TextDecoder === 'undefined') {
    globalThis.TextDecoder = class TextDecoder {
      constructor(encoding = 'utf-8') {
        this.encoding = encoding;
      }
      decode(view) {
        const bytes = view instanceof Uint8Array ? view : new Uint8Array(view);
        let str = '';
        for (let i = 0; i < bytes.length; i++) {
          const byte = bytes[i];
          if (byte < 0x80) {
            str += String.fromCharCode(byte);
          } else if (byte < 0xe0) {
            str += String.fromCharCode(((byte & 0x1f) << 6) | (bytes[++i] & 0x3f));
          } else if (byte < 0xf0) {
            str += String.fromCharCode(((byte & 0x0f) << 12) | ((bytes[++i] & 0x3f) << 6) | (bytes[++i] & 0x3f));
          }
        }
        return str;
      }
    };
  }

  async function fetch(url, options = {}) {
    try {
      const responseData = await globalThis.__moyu_fetch(url, options);

      return {
        status: responseData.status,
        statusText: responseData.status_text,
        ok: responseData.ok,
        headers: {
          get: (name) => responseData.headers[name.toLowerCase()] || null,
          has: (name) => !!responseData.headers[name.toLowerCase()],
          forEach: (callback) => {
            for (const [key, value] of Object.entries(responseData.headers)) {
              callback(value, key);
            }
          },
        },
        async json() {
          const text = new TextDecoder().decode(new Uint8Array(responseData.bytes));
          return JSON.parse(text);
        },
        async text() {
          return new TextDecoder().decode(new Uint8Array(responseData.bytes));
        },
        async arrayBuffer() {
          return new Uint8Array(responseData.bytes).buffer;
        },
      };
    } catch (e) {
      console.error('Fetch error:', e);
      throw e;
    }

    console.debug('Fetch response data:', responseData);
  }

  globalThis.fetch = fetch;
  if (typeof window !== 'undefined') {
    window.fetch = fetch;
  }
})();
